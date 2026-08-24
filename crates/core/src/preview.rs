use std::{
    fs::File,
    io::Read,
    path::Path,
};

pub enum PreviewKind {
    Image,
    Text(String),
    Binary,
    Dir,
}

const MAX_PREVIEW_CHARS: usize = 5_000;
const MAX_PREVIEW_BYTES: u64 = (MAX_PREVIEW_CHARS * 4 + 4) as u64;

pub fn get_preview(path: &Path) -> PreviewKind {
    if path.is_dir() {
        return PreviewKind::Dir;
    }

    let mime = mime_guess::from_path(path).first_or_octet_stream();
    if mime.type_() == mime_guess::mime::IMAGE {
        PreviewKind::Image
    } else if mime.type_() == mime_guess::mime::TEXT || mime.subtype().as_str().contains("json") {
        PreviewKind::Text(read_text_prefix(path))
    } else {
        PreviewKind::Binary
    }
}

fn read_text_prefix(path: &Path) -> String {
    let Ok(file) = File::open(path) else {
        return String::new();
    };

    let mut bytes = Vec::with_capacity(MAX_PREVIEW_BYTES as usize);
    if file.take(MAX_PREVIEW_BYTES).read_to_end(&mut bytes).is_err() {
        return String::new();
    }

    String::from_utf8_lossy(&bytes)
        .chars()
        .take(MAX_PREVIEW_CHARS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{read_text_prefix, MAX_PREVIEW_CHARS};
    use std::{fs, time::SystemTime};

    #[test]
    fn limits_preview_to_requested_character_count() {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("bifur-preview-{unique}.txt"));

        fs::write(&path, "ก".repeat(MAX_PREVIEW_CHARS + 100)).unwrap();
        let preview = read_text_prefix(&path);
        let _ = fs::remove_file(&path);

        assert_eq!(preview.chars().count(), MAX_PREVIEW_CHARS);
    }
}
