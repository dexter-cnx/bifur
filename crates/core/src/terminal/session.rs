use super::{file_aware::shell_cd_command, history::CommandBlock, parser::ScreenBuffer};
use anyhow::{Context, Result};
use portable_pty::{Child, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, RwLock,
    },
    thread::{self, JoinHandle},
};

#[derive(Clone, Debug)]
pub struct TerminalConfig {
    pub shell: String,
    pub cols: u16,
    pub rows: u16,
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: default_shell(),
            cols: 120,
            rows: 30,
            cwd: std::env::current_dir().unwrap_or_default(),
            env: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalEvent {
    OutputReady,
}

/// Pure-Rust terminal owner. Frontends only send input/resize/cwd events and
/// render `screen_snapshot()`; they never own or read the PTY directly.
pub struct TerminalSession {
    pub config: TerminalConfig,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    screen: Arc<RwLock<ScreenBuffer>>,
    history: Arc<RwLock<Vec<CommandBlock>>>,
    event_rx: Option<Receiver<TerminalEvent>>,
    reader_thread: Option<JoinHandle<()>>,
}

impl TerminalSession {
    pub fn spawn(config: TerminalConfig) -> Result<Self> {
        let pty_system = NativePtySystem::default();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("open native PTY")?;

        let mut command = CommandBuilder::new(&config.shell);
        command.cwd(&config.cwd);
        for (key, value) in &config.env {
            command.env(key, value);
        }

        let child = pty_pair
            .slave
            .spawn_command(command)
            .context("spawn terminal shell")?;
        let writer = pty_pair.master.take_writer().context("take PTY writer")?;
        let mut reader = pty_pair
            .master
            .try_clone_reader()
            .context("clone PTY reader")?;

        let screen = Arc::new(RwLock::new(ScreenBuffer::new(
            config.cols as usize,
            config.rows as usize,
        )));
        let reader_screen = Arc::clone(&screen);
        let (event_tx, event_rx) = mpsc::channel();
        let reader_thread = spawn_reader_thread(reader, reader_screen, event_tx)
            .context("spawn terminal reader thread")?;

        Ok(Self {
            config,
            master: pty_pair.master,
            child,
            writer,
            screen,
            history: Arc::new(RwLock::new(Vec::new())),
            event_rx: Some(event_rx),
            reader_thread: Some(reader_thread),
        })
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        let cols = cols.max(1);
        let rows = rows.max(1);
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        self.config.cols = cols;
        self.config.rows = rows;
        if let Ok(mut screen) = self.screen.write() {
            screen.resize(cols as usize, rows as usize);
        }
        Ok(())
    }

    pub fn send_input(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn set_cwd(&mut self, new_cwd: PathBuf) -> Result<()> {
        if self.config.cwd == new_cwd {
            return Ok(());
        }
        let command = shell_cd_command(&self.config.shell, &new_cwd)?;
        self.send_input(command.as_bytes())?;
        self.config.cwd = new_cwd;
        Ok(())
    }

    pub fn screen_snapshot(&self) -> ScreenBuffer {
        self.screen
            .read()
            .map(|screen| screen.clone())
            .unwrap_or_else(|_| {
                ScreenBuffer::new(self.config.cols as usize, self.config.rows as usize)
            })
    }

    /// Transfers the terminal output event receiver to the frontend.
    ///
    /// The receiver is intentionally single-consumer: one frontend owns repaint
    /// scheduling while `TerminalSession` continues to own PTY bytes and parser state.
    pub fn take_event_receiver(&mut self) -> Option<Receiver<TerminalEvent>> {
        self.event_rx.take()
    }

    pub fn command_history(&self) -> Vec<CommandBlock> {
        self.history
            .read()
            .map(|history| history.clone())
            .unwrap_or_default()
    }

    pub fn push_command_block(&self, block: CommandBlock) {
        if let Ok(mut history) = self.history.write() {
            history.push(block);
        }
    }

    pub fn try_wait(&mut self) -> Result<Option<portable_pty::ExitStatus>> {
        Ok(self.child.try_wait()?)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.reader_thread.take();
    }
}

fn spawn_reader_thread(
    mut reader: Box<dyn Read + Send>,
    screen: Arc<RwLock<ScreenBuffer>>,
    event_tx: Sender<TerminalEvent>,
) -> Result<JoinHandle<()>> {
    Ok(thread::Builder::new()
        .name("bifur-terminal-reader".into())
        .spawn(move || {
            let mut bytes = [0_u8; 8192];
            loop {
                match reader.read(&mut bytes) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        if let Ok(mut screen) = screen.write() {
                            screen.push_bytes(&bytes[..read]);
                        }
                        if event_tx.send(TerminalEvent::OutputReady).is_err() {
                            break;
                        }
                    }
                }
            }
        })?)
}

fn default_shell() -> String {
    #[cfg(windows)]
    {
        if let Some(shell) = std::env::var("SHELL")
            .ok()
            .filter(|shell| is_native_windows_shell_candidate(shell))
        {
            return shell;
        }

        return std::env::var("COMSPEC")
            .ok()
            .filter(|shell| !shell.trim().is_empty())
            .unwrap_or_else(|| "powershell.exe".into());
    }

    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into())
    }
}

#[cfg(any(windows, test))]
fn is_native_windows_shell_candidate(shell: &str) -> bool {
    let shell = shell.trim();
    !shell.is_empty() && !shell.starts_with('/')
}

#[cfg(test)]
mod shell_tests {
    use super::is_native_windows_shell_candidate;

    #[test]
    fn rejects_msys_posix_shell_paths_for_native_windows_spawn() {
        assert!(!is_native_windows_shell_candidate("/usr/bin/bash"));
        assert!(!is_native_windows_shell_candidate("/bin/zsh"));
    }

    #[test]
    fn accepts_native_windows_shell_names_and_paths() {
        assert!(is_native_windows_shell_candidate("bash.exe"));
        assert!(is_native_windows_shell_candidate(
            r"C:\Program Files\Git\bin\bash.exe"
        ));
    }
}
