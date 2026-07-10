use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chardetng::EncodingDetector;
use url::Url;

pub fn load_markdown(path: &Path) -> Result<String, String> {
    ensure_supported_markdown_path(path)?;
    let bytes = fs::read(path).map_err(|error| format!("Failed to read file: {error}"))?;
    decode_markdown(&bytes)
}

fn decode_markdown(bytes: &[u8]) -> Result<String, String> {
    if let Some(content) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8(content.to_vec())
            .map_err(|error| format!("Failed to decode UTF-8 file: {error}"));
    }

    if let Some(content) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(content, u16::from_le_bytes);
    }

    if let Some(content) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(content, u16::from_be_bytes);
    }

    if let Ok(content) = std::str::from_utf8(bytes) {
        return Ok(content.to_string());
    }

    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);
    let (content, _, had_errors) = encoding.decode(bytes);

    if had_errors {
        return Err(format!(
            "Failed to decode file using the detected {} encoding.",
            encoding.name()
        ));
    }

    Ok(content.into_owned())
}

fn decode_utf16(bytes: &[u8], decode_unit: fn([u8; 2]) -> u16) -> Result<String, String> {
    if !bytes.len().is_multiple_of(2) {
        return Err("Failed to decode UTF-16 file with an incomplete code unit.".to_string());
    }

    let units = bytes
        .chunks_exact(2)
        .map(|chunk| decode_unit([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();

    String::from_utf16(&units).map_err(|error| format!("Failed to decode UTF-16 file: {error}"))
}

pub fn save_markdown(path: &Path, markdown: &str) -> Result<(), String> {
    ensure_supported_markdown_path(path)?;

    let parent = path
        .parent()
        .ok_or_else(|| format!("File does not have a parent directory: {}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("File name is not valid UTF-8: {}", path.display()))?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("System clock error: {error}"))?
        .as_nanos();
    let temp_path = parent.join(format!(".{file_name}.markhola-{stamp}.tmp"));

    let write_result = fs::write(&temp_path, markdown);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Failed to write temporary file: {error}"));
    }

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Failed to replace file: {error}"));
    }

    Ok(())
}

pub fn ensure_supported_markdown_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("md") | Some("markdown") => Ok(()),
        _ => Err("Only .md and .markdown files are supported in v0.6.0.".to_string()),
    }
}

pub fn directory_base_url(path: &Path) -> Result<String, String> {
    let directory = path
        .parent()
        .ok_or_else(|| "Document path does not have a parent directory.".to_string())?;
    let url = Url::from_directory_path(directory)
        .map_err(|_| "Document directory cannot be converted to a file URL.".to_string())?;
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ensure_supported_markdown_path, load_markdown, save_markdown};

    fn temp_file(name: &str, extension: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("markhola-{name}-{stamp}.{extension}"))
    }

    #[test]
    fn accepts_markdown_extensions() {
        let md_path = temp_file("accepts", "md");
        fs::write(&md_path, "# hello").unwrap();
        assert!(ensure_supported_markdown_path(&md_path).is_ok());
        let _ = fs::remove_file(&md_path);

        let markdown_path = temp_file("accepts", "markdown");
        fs::write(&markdown_path, "# hello").unwrap();
        assert!(ensure_supported_markdown_path(&markdown_path).is_ok());
        let _ = fs::remove_file(&markdown_path);
    }

    #[test]
    fn rejects_non_markdown_extensions() {
        let temp_path = temp_file("rejects", "txt");
        fs::write(&temp_path, "hello").unwrap();

        let error = ensure_supported_markdown_path(&temp_path).unwrap_err();
        assert!(error.contains("Only .md and .markdown files are supported"));
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn saves_without_losing_content() {
        let path = temp_file("save", "md");
        fs::write(&path, "# before").unwrap();

        save_markdown(&path, "# after\nnext line").unwrap();
        let content = load_markdown(&path).unwrap();

        assert_eq!(content, "# after\nnext line");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn loads_utf8_bom_content() {
        let path = temp_file("utf8-bom", "md");
        fs::write(&path, b"\xEF\xBB\xBF# hello").unwrap();

        let content = load_markdown(&path).unwrap();

        assert_eq!(content, "# hello");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn loads_utf16_little_endian_content() {
        let path = temp_file("utf16-le", "md");
        fs::write(&path, [0xFF, 0xFE, 0x4C, 0x68, 0x62, 0x97]).unwrap();

        let content = load_markdown(&path).unwrap();

        assert_eq!(content, "\u{684c}\u{9762}");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn loads_gb18030_content() {
        let path = temp_file("gb18030", "md");
        fs::write(&path, [0xD7, 0xC0, 0xC3, 0xE6, 0xBC, 0xBC, 0xCA, 0xF5]).unwrap();

        let content = load_markdown(&path).unwrap();

        assert_eq!(content, "\u{684c}\u{9762}\u{6280}\u{672f}");
        let _ = fs::remove_file(&path);
    }
}
