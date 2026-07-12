use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rfd::FileDialog;
use tao::window::Window;
use wry::WebView;

use crate::document::ActiveDocument;
use crate::file_io;
use crate::workspace::{DocumentWorkspace, WorkspaceOpenResult};

use super::log_event;
use super::workspace_view::{present_workspace, render_status, sync_workspace_state};

pub(super) fn open_document_dialog(event_id: u64) -> Option<PathBuf> {
    let started_at = SystemTime::now();
    log_event(
        "file_dialog.begin",
        Some(event_id),
        "opening file dialog",
        "",
    );
    let result = FileDialog::new()
        .add_filter("Markdown", &["md", "markdown"])
        .set_title("Open Markdown File")
        .pick_file();
    let elapsed_ms = started_at
        .elapsed()
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    log_event(
        "file_dialog.end",
        Some(event_id),
        "file dialog finished",
        format!("selected={} elapsed_ms={elapsed_ms}", result.is_some()),
    );
    result
}

pub(super) fn open_document(
    window: &Window,
    webview: &WebView,
    workspace: &mut DocumentWorkspace,
    path: &PathBuf,
    event_id: Option<u64>,
) {
    log_event(
        "open_document.begin",
        event_id,
        "open_document start",
        format!("path={}", path.display()),
    );
    render_status(webview, "Loading document...", "info");

    if let Some(document_id) = workspace.find_by_path(path) {
        workspace.activate_document(document_id);
        sync_workspace_state(
            window,
            webview,
            workspace,
            "Document already opened. Switched to tab.",
        );
        return;
    }

    match load_document(workspace.next_document_id(), path) {
        Ok(document) => open_loaded_document(window, webview, workspace, path, event_id, document),
        Err(message) => {
            log_event(
                "open_document.end",
                event_id,
                "open_document failed",
                format!("path={} error={message}", path.display()),
            );
            render_status(webview, &message, "error");
        }
    }
}

pub(crate) fn load_document(document_id: u64, path: &PathBuf) -> Result<ActiveDocument, String> {
    log_event(
        "load_document.begin",
        None,
        "load_document path",
        format!("path={}", path.display()),
    );
    let markdown = file_io::load_markdown(path)?;
    let base_url = file_io::directory_base_url(path)?;
    Ok(ActiveDocument::open_with_id(
        document_id,
        path.clone(),
        markdown,
        base_url,
    ))
}

pub(crate) fn reload_workspace_documents_from_disk(
    workspace: &mut DocumentWorkspace,
) -> Result<String, String> {
    let document_ids = workspace.document_ids();
    let mut reloaded = 0usize;
    let mut skipped_dirty = 0usize;
    let mut failures = Vec::new();

    for document_id in document_ids {
        let Some(document) = workspace.document_by_id_mut(document_id) else {
            continue;
        };
        if document.is_dirty() {
            skipped_dirty += 1;
            continue;
        }
        let path = document.file_path().to_path_buf();
        match file_io::load_markdown(&path) {
            Ok(markdown) => {
                document.reload_from_disk_markdown(markdown);
                reloaded += 1;
            }
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    failures.first().map_or_else(
        || Ok(reload_status_message(reloaded, skipped_dirty)),
        |failure| Err(format!("Reload failed: {failure}")),
    )
}

fn open_loaded_document(
    window: &Window,
    webview: &WebView,
    workspace: &mut DocumentWorkspace,
    path: &Path,
    event_id: Option<u64>,
    document: ActiveDocument,
) {
    log_event(
        "open_document.end",
        event_id,
        "open_document success",
        format!("path={}", path.display()),
    );
    match workspace.open_document(document) {
        WorkspaceOpenResult::OpenedNew(_) => {
            present_workspace(window, webview, workspace, "Document loaded.", true)
        }
        WorkspaceOpenResult::ActivatedExisting(_) => sync_workspace_state(
            window,
            webview,
            workspace,
            "Document already opened. Switched to tab.",
        ),
    }
}

fn reload_status_message(reloaded: usize, skipped_dirty: usize) -> String {
    match (reloaded, skipped_dirty) {
        (0, 0) | (_, 0) => "Document reloaded.".to_string(),
        (_, 1) => "Document reloaded. One unsaved tab was kept as-is.".to_string(),
        (_, count) => format!("Document reloaded. {count} unsaved tabs were kept as-is."),
    }
}
