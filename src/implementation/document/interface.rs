use std::path::PathBuf;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentMode {
    Readonly,
    Writable,
}

#[derive(Clone, Debug)]
pub struct ActiveDocument {
    pub(super) id: u64,
    pub(super) file_path: PathBuf,
    pub(super) canonical_path: PathBuf,
    pub(super) file_name: String,
    pub(super) title: String,
    pub(super) markdown: String,
    pub(super) saved_markdown: String,
    pub(super) html: String,
    pub(super) base_url: String,
    pub(super) word_count: usize,
    pub(super) line_count: usize,
    pub(super) mode: DocumentMode,
    pub(super) dirty: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocumentSnapshot {
    pub document_id: u64,
    pub file_name: String,
    pub file_path: String,
    pub title: String,
    pub base_url: String,
    pub word_count: usize,
    pub line_count: usize,
    pub html: String,
    pub markdown: String,
    pub mode: DocumentMode,
    pub mode_label: &'static str,
    pub dirty: bool,
    pub save_status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct DocumentTabSnapshot {
    pub document_id: u64,
    pub file_name: String,
    pub title: String,
    pub dirty: bool,
    pub active: bool,
    pub mode: DocumentMode,
}
