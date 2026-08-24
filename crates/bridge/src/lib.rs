use bifur_core::fs_model::{FileEntry, PaneState};

#[flutter_rust_bridge::frb(sync)]
pub fn get_files(path: String) -> Vec<FileEntry> {
    PaneState::read_dir(&std::path::PathBuf::from(path))
}

#[flutter_rust_bridge::frb(sync)]
pub fn batch_rename_preview(paths: Vec<String>, pattern: String) -> Vec<String> {
    bifur_core::fs_model::batch_rename(paths, pattern)
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Logger and other process-wide initialization can be added here later.
}
