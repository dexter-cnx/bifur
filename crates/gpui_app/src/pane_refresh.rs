use crate::pane_watcher::{PaneSide, PaneWatcher};
use bifur_core::fs_model::{FileEntry, PaneState};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

#[derive(Clone, Debug)]
pub struct PaneRefreshRequest {
    pub side: PaneSide,
    pub source_path: PathBuf,
}

impl PaneRefreshRequest {
    pub fn new(side: PaneSide, source_path: PathBuf) -> Self {
        Self { side, source_path }
    }

    pub fn read(self) -> PaneRefreshSnapshot {
        let entries = PaneState::read_dir(&self.source_path);
        PaneRefreshSnapshot {
            side: self.side,
            source_path: self.source_path,
            entries,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaneRefreshSnapshot {
    pub side: PaneSide,
    pub source_path: PathBuf,
    pub entries: Vec<FileEntry>,
}

pub struct PaneRefreshCoordinator {
    left: PaneWatcher,
    right: PaneWatcher,
    receiver: Option<Receiver<PaneSide>>,
}

impl PaneRefreshCoordinator {
    pub fn new(left_path: &Path, right_path: &Path) -> notify::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let left = PaneWatcher::new(PaneSide::Left, left_path, sender.clone())?;
        let right = PaneWatcher::new(PaneSide::Right, right_path, sender)?;

        Ok(Self {
            left,
            right,
            receiver: Some(receiver),
        })
    }

    pub fn watch_path(&mut self, side: PaneSide, path: &Path) -> notify::Result<()> {
        match side {
            PaneSide::Left => self.left.watch_path(path),
            PaneSide::Right => self.right.watch_path(path),
        }
    }

    pub fn take_receiver(&mut self) -> Option<Receiver<PaneSide>> {
        self.receiver.take()
    }
}

#[cfg(test)]
mod tests {
    use super::PaneRefreshRequest;
    use crate::pane_watcher::PaneSide;
    use std::{fs, time::SystemTime};

    #[test]
    fn background_snapshot_retains_source_path_and_side() {
        let mut root = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        root.push(format!("bifur-refresh-request-{unique}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("item.txt"), "item").unwrap();

        let snapshot = PaneRefreshRequest::new(PaneSide::Right, root.clone()).read();

        assert_eq!(snapshot.side, PaneSide::Right);
        assert_eq!(snapshot.source_path, root);
        assert!(snapshot
            .entries
            .iter()
            .any(|entry| entry.path == snapshot.source_path.join("item.txt")));

        let _ = fs::remove_dir_all(snapshot.source_path);
    }
}
