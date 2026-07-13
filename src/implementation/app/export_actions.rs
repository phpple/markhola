use crate::html_export::{self, HtmlExportOutcome};
use crate::pdf_export::{self, PdfExportOutcome};
use crate::printing::{self, PrintOutcome};
use crate::workspace::DocumentWorkspace;
use tao::window::Window;
use wry::WebView;

use super::workspace_view::{render_status, render_status_with_action};

pub(super) fn export_pdf(window: &Window, webview: &WebView, workspace: &DocumentWorkspace) {
    match workspace.active_document() {
        Some(document) => match pdf_export::export_document(window, document) {
            Ok(PdfExportOutcome::Exported(path)) => render_status_with_action(
                webview,
                &format!("Exported PDF: {}", path.display()),
                "info",
                Some(&path.display().to_string()),
                Some("Open"),
            ),
            Ok(PdfExportOutcome::Cancelled) => render_status(webview, "Export cancelled.", "info"),
            Err(message) => render_status(webview, &message, "error"),
        },
        None => render_status(webview, "No document opened.", "error"),
    }
}

pub(super) fn export_html(webview: &WebView, workspace: &DocumentWorkspace) {
    match workspace.active_document() {
        Some(document) => match html_export::export_document(document) {
            Ok(HtmlExportOutcome::Exported(path)) => render_status_with_action(
                webview,
                &format!("Exported HTML: {}", path.display()),
                "info",
                Some(&path.display().to_string()),
                Some("Open"),
            ),
            Ok(HtmlExportOutcome::Cancelled) => render_status(webview, "Export cancelled.", "info"),
            Err(message) => render_status(webview, &message, "error"),
        },
        None => render_status(webview, "No document opened.", "error"),
    }
}

pub(super) fn print_document(window: &Window, webview: &WebView, workspace: &DocumentWorkspace) {
    match workspace.active_document() {
        Some(document) => match printing::print_document(window, document) {
            Ok(PrintOutcome::Started) => render_status(webview, "Print panel opened.", "info"),
            Ok(PrintOutcome::Cancelled) => render_status(webview, "Print cancelled.", "info"),
            Err(message) => render_status(webview, &message, "error"),
        },
        None => render_status(webview, "No document opened.", "error"),
    }
}

pub(super) fn open_find_panel(webview: &WebView, workspace: &DocumentWorkspace) {
    if workspace.active_document().is_some() {
        let _ = webview.evaluate_script("window.openFindPanel();");
    } else {
        render_status(webview, "No document opened.", "error");
    }
}
