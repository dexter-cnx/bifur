use crate::pane_watcher::{PaneSide, PaneWatcher};
use std::{
    path::Path,
    sync::mpsc::{self, Receiver},
};

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
