use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::markdown;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentMode {
    Readonly,
    Writable,
}

impl DocumentMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Readonly => Self::Writable,
            Self::Writable => Self::Readonly,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Readonly => "Readonly",
            Self::Writable => "Writable",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActiveDocument {
    id: u64,
    file_path: PathBuf,
    canonical_path: PathBuf,
    file_name: String,
    title: String,
    markdown: String,
    saved_markdown: String,
    html: String,
    base_url: String,
    word_count: usize,
    line_count: usize,
    mode: DocumentMode,
    dirty: bool,
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

impl ActiveDocument {
    pub fn open_with_id(id: u64, path: PathBuf, markdown: String, base_url: String) -> Self {
        let file_name = file_name(&path).unwrap_or_else(|| "Untitled".to_string());
        let canonical_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let title = markdown::extract_title(&markdown).unwrap_or_else(|| file_name.clone());
        let html = markdown::render_html(&markdown);
        let (word_count, line_count) = text_metrics(&markdown);

        Self {
            id,
            file_path: path,
            canonical_path,
            file_name,
            title,
            saved_markdown: markdown.clone(),
            markdown,
            html,
            base_url,
            word_count,
            line_count,
            mode: DocumentMode::Readonly,
            dirty: false,
        }
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            document_id: self.id,
            file_name: self.file_name.clone(),
            file_path: self.file_path.display().to_string(),
            title: self.title.clone(),
            base_url: self.base_url.clone(),
            word_count: self.word_count,
            line_count: self.line_count,
            html: self.html.clone(),
            markdown: self.markdown.clone(),
            mode: self.mode,
            mode_label: self.mode.label(),
            dirty: self.dirty,
            save_status: if self.dirty { "Unsaved" } else { "Saved" },
        }
    }

    pub fn tab_snapshot(&self, active: bool) -> DocumentTabSnapshot {
        DocumentTabSnapshot {
            document_id: self.id,
            file_name: self.file_name.clone(),
            title: self.title.clone(),
            dirty: self.dirty,
            active,
            mode: self.mode,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn mode(&self) -> DocumentMode {
        self.mode
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn suggested_pdf_export_path(&self) -> PathBuf {
        suggested_pdf_export_path(self.file_path())
    }

    pub fn update_markdown(&mut self, markdown: String) {
        self.markdown = markdown;
        self.refresh_metadata();
        self.dirty = self.markdown != self.saved_markdown;
    }

    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.toggle();
        if self.mode == DocumentMode::Readonly {
            self.refresh_html();
        }
    }

    pub fn mark_saved(&mut self) {
        self.saved_markdown = self.markdown.clone();
        self.refresh_metadata();
        self.refresh_html();
        self.dirty = false;
    }

    pub fn reload_from_disk_markdown(&mut self, markdown: String) {
        self.saved_markdown = markdown.clone();
        self.markdown = markdown;
        self.refresh_metadata();
        self.refresh_html();
        self.dirty = false;
    }

    pub fn window_title(&self) -> String {
        let dirty_marker = if self.dirty { " *" } else { "" };
        format!("{}{} - MarkHola", self.file_name(), dirty_marker)
    }

    fn refresh_metadata(&mut self) {
        self.title = markdown::extract_title(&self.markdown).unwrap_or_else(|| self.file_name.clone());
        let (word_count, line_count) = text_metrics(&self.markdown);
        self.word_count = word_count;
        self.line_count = line_count;
    }

    fn refresh_html(&mut self) {
        self.html = markdown::render_html(&self.markdown);
    }
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
}

pub fn suggested_pdf_export_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .or_else(|| path.file_name().and_then(|value| value.to_str()).filter(|value| !value.is_empty()))
        .unwrap_or("document");

    path.with_file_name(format!("{stem}.pdf"))
}

fn text_metrics(markdown: &str) -> (usize, usize) {
    (markdown.split_whitespace().count(), markdown.lines().count())
}

#[cfg(test)]
mod tests {
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
}
