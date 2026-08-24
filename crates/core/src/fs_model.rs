use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
}

#[derive(Clone, Debug)]
pub struct PaneState {
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected: usize,
}

impl PaneState {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let entries = Self::read_dir(&path);
        Self {
            current_path: path,
            entries,
            selected: 0,
        }
    }

    pub fn read_dir(path: &Path) -> Vec<FileEntry> {
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(path) {
            for entry in read_dir.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    entries.push(FileEntry {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: entry.path(),
                        is_dir: metadata.is_dir(),
                        size: metadata.len(),
                        modified: metadata
                            .modified()
                            .ok()
                            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|duration| duration.as_secs())
                            .unwrap_or(0),
                    });
                }
            }
        }

        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        entries
    }

    pub fn enter(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                self.current_path = entry.path.clone();
                self.entries = Self::read_dir(&self.current_path);
                self.selected = 0;
            }
        }
    }

    pub fn up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.current_path = parent.to_path_buf();
            self.entries = Self::read_dir(&self.current_path);
            self.selected = 0;
        }
    }
}

pub fn batch_rename(paths: Vec<String>, pattern: String) -> Vec<String> {
    let placeholder = regex::Regex::new(r"\{name\}").expect("valid rename placeholder regex");
    paths
        .iter()
        .map(|path| {
            let path = Path::new(path);
            let name = path.file_stem().unwrap_or_default().to_string_lossy();
            placeholder.replace_all(&pattern, name.as_ref()).to_string()
        })
        .collect()
}

#[cfg(all(test, unix))]
mod tests {
    use super::PaneState;
    use std::{ffi::OsString, fs, os::unix::ffi::OsStringExt, time::SystemTime};

    #[test]
    fn entering_directory_preserves_non_utf8_path_bytes() {
        let mut root = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        root.push(format!("bifur-path-{unique}"));
        fs::create_dir_all(&root).unwrap();

        let child_name = OsString::from_vec(vec![b'n', b'o', b'n', 0xff, b'u', b't', b'f', b'8']);
        let child = root.join(child_name);
        fs::create_dir(&child).unwrap();

        let mut pane = PaneState::new(&root);
        pane.selected = pane
            .entries
            .iter()
            .position(|entry| entry.path == child)
            .unwrap();
        pane.enter();

        assert_eq!(pane.current_path, child);
        let _ = fs::remove_dir_all(&root);
    }
}
