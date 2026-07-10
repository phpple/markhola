use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use url::Url;

pub fn load_markdown(path: &Path) -> Result<String, String> {
    ensure_supported_markdown_path(path)?;
    fs::read_to_string(path).map_err(|error| format!("Failed to read file: {error}"))
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

    if is_supported_markdown_path(path) {
        Ok(())
    } else {
        Err("Only .md and .markdown files are supported in v0.6.0.".to_string())
    }
}

pub fn is_supported_markdown_path(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    matches!(extension.as_deref(), Some("md") | Some("markdown"))
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

    use super::{
        ensure_supported_markdown_path, is_supported_markdown_path, load_markdown, save_markdown,
    };

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
    fn detects_supported_markdown_extensions_without_requiring_existing_file() {
        assert!(is_supported_markdown_path(&PathBuf::from("技术.MD")));
        assert!(is_supported_markdown_path(&PathBuf::from("notes.markdown")));
        assert!(!is_supported_markdown_path(&PathBuf::from("notes.txt")));
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
}
