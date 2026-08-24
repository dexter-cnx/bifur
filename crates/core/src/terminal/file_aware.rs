use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// Contract used by frontends that keep the terminal synchronized with the
/// active file pane.
pub trait FileAwareTerminal {
    fn on_pane_changed(&mut self, new_path: PathBuf) -> anyhow::Result<()>;
    fn on_file_selected(&mut self, path: PathBuf);
}

/// Build a shell-specific cwd command without silently corrupting paths.
///
/// Shell command input is textual, so paths that cannot be represented as
/// Unicode must be reported as unsupported rather than converted with
/// `to_string_lossy()`. That keeps `TerminalSession::config.cwd` truthful: the
/// session only records a cwd after a command can actually be sent.
pub fn shell_cd_command(shell: &str, path: &Path) -> Result<String> {
    let shell_name = Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(shell)
        .to_ascii_lowercase();
    let Some(path) = path.to_str() else {
        bail!("terminal cwd cannot be represented losslessly as shell text");
    };

    if shell_name.contains("powershell") || shell_name == "pwsh" || shell_name == "pwsh.exe" {
        let escaped = path.replace('\'', "''");
        Ok(format!("Set-Location -LiteralPath '{escaped}'\r\n"))
    } else if shell_name == "cmd" || shell_name == "cmd.exe" {
        let escaped = path.replace('"', "\"\"");
        Ok(format!("cd /d \"{escaped}\"\r\n"))
    } else {
        let escaped = path.replace('\'', "'\\''");
        Ok(format!("cd '{escaped}'\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::shell_cd_command;
    use std::path::Path;

    #[test]
    fn posix_path_is_shell_escaped() {
        assert_eq!(
            shell_cd_command("/bin/zsh", Path::new("/tmp/a'b")).unwrap(),
            "cd '/tmp/a'\\''b'\n"
        );
    }

    #[test]
    fn powershell_path_uses_literal_path() {
        assert_eq!(
            shell_cd_command("pwsh.exe", Path::new("C:\\A'B")).unwrap(),
            "Set-Location -LiteralPath 'C:\\A''B'\r\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_is_rejected_instead_of_lossily_changed() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

        let path = PathBuf::from(OsString::from_vec(vec![b'/', b't', b'm', b'p', b'/', 0xff]));
        assert!(shell_cd_command("/bin/zsh", &path).is_err());
    }
}
