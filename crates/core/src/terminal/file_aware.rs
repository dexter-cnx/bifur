use std::path::{Path, PathBuf};

/// Contract used by frontends that keep the terminal synchronized with the
/// active file pane.
pub trait FileAwareTerminal {
    fn on_pane_changed(&mut self, new_path: PathBuf) -> anyhow::Result<()>;
    fn on_file_selected(&mut self, path: PathBuf);
}

pub fn shell_cd_command(shell: &str, path: &Path) -> String {
    let shell_name = Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(shell)
        .to_ascii_lowercase();

    if shell_name.contains("powershell") || shell_name == "pwsh" || shell_name == "pwsh.exe" {
        let escaped = path.to_string_lossy().replace('\'', "''");
        format!("Set-Location -LiteralPath '{escaped}'\r\n")
    } else if shell_name == "cmd" || shell_name == "cmd.exe" {
        let escaped = path.to_string_lossy().replace('"', "\"\"");
        format!("cd /d \"{escaped}\"\r\n")
    } else {
        let escaped = path.to_string_lossy().replace('\'', "'\\''");
        format!("cd '{escaped}'\n")
    }
}

#[cfg(test)]
mod tests {
    use super::shell_cd_command;
    use std::path::Path;

    #[test]
    fn posix_path_is_shell_escaped() {
        assert_eq!(
            shell_cd_command("/bin/zsh", Path::new("/tmp/a'b")),
            "cd '/tmp/a'\\''b'\n"
        );
    }

    #[test]
    fn powershell_path_uses_literal_path() {
        assert_eq!(
            shell_cd_command("pwsh.exe", Path::new("C:\\A'B")),
            "Set-Location -LiteralPath 'C:\\A''B'\r\n"
        );
    }
}
