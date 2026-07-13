use std::path::{Path, PathBuf};

use crate::markdown;

use super::{ActiveDocument, DocumentMode, DocumentSnapshot, DocumentTabSnapshot};

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

impl ActiveDocument {
    pub fn open_with_id(id: u64, path: PathBuf, markdown: String, base_url: String) -> Self {
        let file_name = file_name(&path).unwrap_or_else(|| "Untitled".to_string());
        let canonical_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let title = markdown::extract_title(&markdown).unwrap_or_else(|| file_name.clone());
        let html = markdown::render_html(&markdown);
        let (word_count, line_count) = text_metrics(&markdown);

        Self {
            id,
            draft: false,
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

    pub fn new_blank_with_id(id: u64) -> Self {
        let draft_path = std::env::current_dir()
            .unwrap_or_else(|_| std::env::temp_dir())
            .join(format!(".markhola-draft-{id}.md"));
        let base_url = file_io_base_url(&draft_path);
        let markdown = String::new();

        Self {
            id,
            draft: true,
            file_path: draft_path.clone(),
            canonical_path: draft_path,
            file_name: "Untitled".to_string(),
            title: "Untitled".to_string(),
            saved_markdown: markdown.clone(),
            markdown,
            html: String::new(),
            base_url,
            word_count: 0,
            line_count: 0,
            mode: DocumentMode::Writable,
            dirty: false,
        }
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            document_id: self.id,
            file_name: self.file_name.clone(),
            file_path: self.display_path(),
            title: self.title.clone(),
            base_url: self.base_url.clone(),
            word_count: self.word_count,
            line_count: self.line_count,
            html: self.html.clone(),
            markdown: self.markdown.clone(),
            mode: self.mode,
            mode_label: self.mode.label(),
            dirty: self.needs_save(),
            save_status: if self.needs_save() { "Unsaved" } else { "Saved" },
        }
    }

    pub fn tab_snapshot(&self, active: bool) -> DocumentTabSnapshot {
        DocumentTabSnapshot {
            document_id: self.id,
            file_name: self.file_name.clone(),
            title: self.title.clone(),
            dirty: self.needs_save(),
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
        self.needs_save()
    }

    pub fn is_draft(&self) -> bool {
        self.draft
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

    pub fn replace_file_path(&mut self, path: PathBuf, base_url: String) {
        self.draft = false;
        self.file_name = file_name(&path).unwrap_or_else(|| "Untitled".to_string());
        self.canonical_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        self.file_path = path;
        self.base_url = base_url;
        self.mark_saved();
    }

    pub fn window_title(&self) -> String {
        let dirty_marker = if self.needs_save() { " *" } else { "" };
        format!("{}{} - MarkHola", self.file_name(), dirty_marker)
    }

    fn refresh_metadata(&mut self) {
        self.title =
            markdown::extract_title(&self.markdown).unwrap_or_else(|| self.file_name.clone());
        let (word_count, line_count) = text_metrics(&self.markdown);
        self.word_count = word_count;
        self.line_count = line_count;
    }

    fn refresh_html(&mut self) {
        self.html = markdown::render_html(&self.markdown);
    }

    fn needs_save(&self) -> bool {
        self.draft || self.dirty
    }

    fn display_path(&self) -> String {
        if self.draft {
            "Unsaved draft".to_string()
        } else {
            self.file_path.display().to_string()
        }
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
        .or_else(|| {
            path.file_name()
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("document");

    path.with_file_name(format!("{stem}.pdf"))
}

fn text_metrics(markdown: &str) -> (usize, usize) {
    (
        markdown.split_whitespace().count(),
        markdown.lines().count(),
    )
}

fn file_io_base_url(path: &Path) -> String {
    path.parent()
        .and_then(|directory| url::Url::from_directory_path(directory).ok())
        .map(|url| url.to_string())
        .unwrap_or_default()
}
