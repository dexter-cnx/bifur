pub mod fs_model;
pub mod preview;
pub mod terminal;

pub mod file_ops {
    use std::path::Path;

    pub fn delete(path: &Path) -> std::io::Result<()> {
        if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        }
    }
}
