use std::path::Path;

pub enum PreviewKind {
    Image,
    Text(String),
    Binary,
    Dir,
}

pub fn get_preview(path: &Path) -> PreviewKind {
    if path.is_dir() {
        return PreviewKind::Dir;
    }

    let mime = mime_guess::from_path(path).first_or_octet_stream();
    if mime.type_() == mime_guess::mime::IMAGE {
        PreviewKind::Image
    } else if mime.type_() == mime_guess::mime::TEXT || mime.subtype().as_str().contains("json") {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        PreviewKind::Text(content.chars().take(5000).collect())
    } else {
        PreviewKind::Binary
    }
}
