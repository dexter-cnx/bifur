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
            notify::recommended_watcher(move |_result: notify::Result<notify::Event>| {
                // A watcher error can mean events were dropped (for example an
                // overflow). Refreshing from the current pane path is the safest
                // recovery because the consumer rebuilds an authoritative snapshot.
                let _ = sender.send(side);
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

        let old_path = std::mem::replace(&mut self.watched_path, path.to_path_buf());
        // Backends can drop a watch themselves when a directory disappears. At
        // that point unwatch may fail even though the new watch is already valid.
        // Keep the committed new state; a lingering old watch can only cause an
        // extra refresh signal, which is safe because refresh reads current_path.
        let _ = self.watcher.unwatch(&old_path);
        Ok(())
    }
}
