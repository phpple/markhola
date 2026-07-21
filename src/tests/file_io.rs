use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    directory_base_url, ensure_supported_markdown_extension, ensure_supported_markdown_path,
    load_markdown, save_markdown,
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

#[test]
fn save_as_allows_new_markdown_path() {
    let path = temp_file("save-as", "md");

    assert!(ensure_supported_markdown_extension(&path).is_ok());
}

#[test]
fn directory_base_url_ends_with_trailing_slash() {
    let path = temp_file("base-url", "md");
    fs::write(&path, "# base").unwrap();

    let base = directory_base_url(&path).unwrap();
    assert!(
        base.ends_with('/'),
        "base url should end with '/': {base}"
    );

    let _ = fs::remove_file(&path);
}
