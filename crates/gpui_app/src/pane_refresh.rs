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

impl PaneRefreshSnapshot {
    pub fn apply(self, left: &mut PaneState, right: &mut PaneState) -> bool {
        let pane = match self.side {
            PaneSide::Left => left,
            PaneSide::Right => right,
        };
        pane.replace_entries(&self.source_path, self.entries)
    }
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
    use bifur_core::fs_model::PaneState;
    use std::{fs, time::SystemTime};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let mut root = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        root.push(format!("bifur-refresh-{label}-{unique}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn background_snapshot_retains_source_path_and_side() {
        let root = temp_dir("request");
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

    #[test]
    fn snapshot_applies_only_to_target_pane() {
        let left_root = temp_dir("left");
        let right_root = temp_dir("right");
        fs::write(left_root.join("left.txt"), "left").unwrap();
        fs::write(right_root.join("right.txt"), "right").unwrap();

        let mut left = PaneState::new(&left_root);
        let mut right = PaneState::new(&right_root);
        fs::write(right_root.join("new.txt"), "new").unwrap();

        let snapshot = PaneRefreshRequest::new(PaneSide::Right, right_root.clone()).read();
        assert!(snapshot.apply(&mut left, &mut right));
        assert_eq!(left.entries.len(), 1);
        assert!(right
            .entries
            .iter()
            .any(|entry| entry.path == right_root.join("new.txt")));

        let _ = fs::remove_dir_all(left_root);
        let _ = fs::remove_dir_all(right_root);
    }

    #[test]
    fn stale_snapshot_is_rejected_by_core_pane_state() {
        let root = temp_dir("stale");
        fs::create_dir_all(root.join("child")).unwrap();
        let mut left = PaneState::new(&root);
        let mut right = PaneState::new(&root);
        let stale = PaneRefreshRequest::new(PaneSide::Left, root.clone()).read();

        left.selected = left
            .entries
            .iter()
            .position(|entry| entry.path == root.join("child"))
            .unwrap();
        assert!(left.enter());
        assert!(!stale.apply(&mut left, &mut right));
        assert_eq!(left.current_path, root.join("child"));

        let _ = fs::remove_dir_all(root);
    }
}
