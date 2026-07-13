use std::path::PathBuf;

use rfd::FileDialog;
use tao::window::Window;
use wry::WebView;

use crate::document::ActiveDocument;
use crate::file_io;
use crate::workspace::DocumentWorkspace;

use super::workspace_view::{render_status, sync_workspace_state};

pub(super) fn save_document(document: &mut ActiveDocument) -> Result<(), String> {
    if document.is_draft() {
        return Err("Draft documents must choose a save path first.".to_string());
    }
    file_io::save_markdown(document.file_path(), document.markdown())?;
    document.mark_saved();
    Ok(())
}

pub(super) fn save_active_document(
    window: &Window,
    webview: &WebView,
    workspace: &mut DocumentWorkspace,
) -> bool {
    if workspace
        .active_document()
        .map(ActiveDocument::is_draft)
        .unwrap_or(false)
    {
        return save_active_document_as(window, webview, workspace);
    }

    let Some(document) = workspace.active_document_mut() else {
        render_status(webview, "No document to save.", "error");
        return false;
    };
    if let Err(message) = save_document(document) {
        render_status(webview, &message, "error");
        return false;
    }
    sync_workspace_state(window, webview, workspace, "Saved.");
    true
}

pub(super) fn save_active_document_as(
    window: &Window,
    webview: &WebView,
    workspace: &mut DocumentWorkspace,
) -> bool {
    let Some(document) = workspace.active_document() else {
        render_status(webview, "No document to save.", "error");
        return false;
    };

    let snapshot = SaveAsSnapshot::from_document(document);
    let Some(path) = choose_save_as_path(&snapshot) else {
        render_status(webview, "Save As cancelled.", "info");
        return false;
    };
    if workspace
        .find_by_path_excluding(&path, snapshot.document_id)
        .is_some()
    {
        render_status(
            webview,
            "Save As target is already open in another tab.",
            "error",
        );
        return false;
    }
    if let Err(error) = file_io::save_markdown(&path, &snapshot.markdown) {
        render_status(webview, &error, "error");
        return false;
    }
    let Ok(base_url) = file_io::directory_base_url(&path) else {
        render_status(
            webview,
            "Document directory cannot be converted to a file URL.",
            "error",
        );
        return false;
    };
    let Some(document) = workspace.active_document_mut() else {
        render_status(webview, "No document to save.", "error");
        return false;
    };
    document.replace_file_path(path, base_url);
    sync_workspace_state(window, webview, workspace, "Saved to new path.");
    true
}

struct SaveAsSnapshot {
    document_id: u64,
    directory: PathBuf,
    file_name: String,
    markdown: String,
}

impl SaveAsSnapshot {
    fn from_document(document: &ActiveDocument) -> Self {
        Self {
            document_id: document.id(),
            directory: document
                .file_path()
                .parent()
                .unwrap_or(document.file_path())
                .to_path_buf(),
            file_name: document.file_name().to_string(),
            markdown: document.markdown().to_string(),
        }
    }
}

fn choose_save_as_path(snapshot: &SaveAsSnapshot) -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("Markdown", &["md", "markdown"])
        .set_title("Save Markdown File As")
        .set_directory(&snapshot.directory)
        .set_file_name(&snapshot.file_name)
        .save_file()
}
