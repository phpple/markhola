use std::path::PathBuf;

use super::{ActiveDocument, DocumentMode};

#[test]
fn switching_to_readonly_rerenders_preview() {
    let mut document = ActiveDocument::open_with_id(
        1,
        PathBuf::from("/tmp/demo.md"),
        "# Hello\nworld".to_string(),
        "file:///tmp/".to_string(),
    );

    document.toggle_mode();
    assert_eq!(document.mode(), DocumentMode::Writable);

    document.update_markdown("# Updated\ncontent".to_string());
    document.toggle_mode();

    let snapshot = document.snapshot();
    assert_eq!(snapshot.mode, DocumentMode::Readonly);
    assert!(snapshot.html.contains("Updated"));
    assert!(snapshot.dirty);
}

#[test]
fn dirty_state_clears_after_save() {
    let mut document = ActiveDocument::open_with_id(
        1,
        PathBuf::from("/tmp/demo.md"),
        "# Hello".to_string(),
        "file:///tmp/".to_string(),
    );

    document.update_markdown("# Hello again".to_string());
    assert!(document.is_dirty());

    document.mark_saved();
    let snapshot = document.snapshot();

    assert!(!snapshot.dirty);
    assert_eq!(snapshot.save_status, "Saved");
    assert!(snapshot.html.contains("Hello again"));
}

#[test]
fn reloading_from_disk_replaces_content_and_clears_dirty_state() {
    let mut document = ActiveDocument::open_with_id(
        1,
        PathBuf::from("/tmp/demo.md"),
        "# Hello".to_string(),
        "file:///tmp/".to_string(),
    );

    document.update_markdown("# Unsaved".to_string());
    assert!(document.is_dirty());

    document.reload_from_disk_markdown("# Reloaded".to_string());
    let snapshot = document.snapshot();

    assert_eq!(snapshot.markdown, "# Reloaded");
    assert!(snapshot.html.contains("Reloaded"));
    assert!(!snapshot.dirty);
}

#[test]
fn blank_document_starts_writable_and_unsaved() {
    let document = ActiveDocument::new_blank_with_id(42);
    let snapshot = document.snapshot();

    assert_eq!(document.mode(), DocumentMode::Writable);
    assert!(document.is_draft());
    assert!(document.is_dirty());
    assert_eq!(snapshot.file_name, "Untitled");
    assert_eq!(snapshot.file_path, "Unsaved draft");
    assert_eq!(snapshot.save_status, "Unsaved");
}
