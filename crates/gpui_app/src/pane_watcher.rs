use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneSide {
    Left,
    Right,
}

pub struct PaneWatcher {
    watcher: RecommendedWatcher,
    watched_path: PathBuf,
}

impl PaneWatcher {
    pub fn new(side: PaneSide, path: &Path, sender: Sender<PaneSide>) -> notify::Result<Self> {
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                if result.is_ok() {
                    let _ = sender.send(side);
                }
            })?;
        watcher.watch(path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            watcher,
            watched_path: path.to_path_buf(),
        })
    }

    pub fn watch_path(&mut self, path: &Path) -> notify::Result<()> {
        if path == self.watched_path {
            return Ok(());
        }

        // Watching the new path before releasing the old one avoids a gap where
        // filesystem changes could be missed during pane navigation. Duplicate
        // signals during this short overlap are harmless because the GPUI side
        // always reads the pane's current path and core rejects stale snapshots.
        self.watcher.watch(path, RecursiveMode::NonRecursive)?;
        self.watcher.unwatch(&self.watched_path)?;
        self.watched_path = path.to_path_buf();
        Ok(())
    }
}
