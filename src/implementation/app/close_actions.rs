use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use tao::window::Window;
use wry::WebView;

use crate::document::ActiveDocument;
use crate::workspace::DocumentWorkspace;

use super::PendingChangesAction;
use super::asset_access::{AssetAccessRegistry, unregister_document};
use super::native_footer::NativeFooter;
use super::save_actions::save_document;
use super::workspace_view::{render_status, sync_workspace_state};

pub(super) fn resolve_all_pending_changes(
    window: &Window,
    webview: &WebView,
    workspace: &mut DocumentWorkspace,
) -> bool {
    let document_ids = workspace
        .tab_snapshots()
        .into_iter()
        .map(|tab| tab.document_id)
        .collect::<Vec<_>>();
    for document_id in document_ids {
        let Some(document) = workspace.document_by_id_mut(document_id) else {
            continue;
        };
        if !resolve_document_pending_changes(window, webview, document) {
            return false;
        }
    }
    true
}

pub(super) fn close_document_tab(
    window: &Window,
    webview: &WebView,
    native_footer: &NativeFooter,
    workspace: &mut DocumentWorkspace,
    document_id: u64,
    status: &str,
    asset_access: &AssetAccessRegistry,
) -> bool {
    let Some(document) = workspace.document_by_id_mut(document_id) else {
        render_status(webview, "Document tab no longer exists.", "error");
        return false;
    };
    if !resolve_document_pending_changes(window, webview, document) {
        return false;
    }
    workspace.close_document(document_id);
    unregister_document(asset_access, document_id);
    sync_workspace_state(window, webview, native_footer, workspace, status);
    true
}

pub(super) fn close_document_tabs(
    window: &Window,
    webview: &WebView,
    native_footer: &NativeFooter,
    workspace: &mut DocumentWorkspace,
    document_ids: &[u64],
    status: &str,
    asset_access: &AssetAccessRegistry,
) -> bool {
    for document_id in document_ids {
        let Some(document) = workspace.document_by_id_mut(*document_id) else {
            continue;
        };
        if !resolve_document_pending_changes(window, webview, document) {
            return false;
        }
    }
    for document_id in document_ids {
        workspace.close_document(*document_id);
        unregister_document(asset_access, *document_id);
    }
    sync_workspace_state(window, webview, native_footer, workspace, status);
    true
}

pub(super) fn resolve_document_pending_changes(
    window: &Window,
    webview: &WebView,
    document: &mut ActiveDocument,
) -> bool {
    if !document.is_dirty() {
        return true;
    }
    match ask_pending_changes_action(window, document.file_name()) {
        PendingChangesAction::Save => match save_document(document) {
            Ok(()) => true,
            Err(message) => {
                render_status(webview, &message, "error");
                false
            }
        },
        PendingChangesAction::Discard => true,
        PendingChangesAction::Cancel => {
            render_status(webview, "Action cancelled.", "info");
            false
        }
    }
}

fn ask_pending_changes_action(window: &Window, file_name: &str) -> PendingChangesAction {
    let result = MessageDialog::new()
        .set_parent(window)
        .set_level(MessageLevel::Warning)
        .set_title("Unsaved changes")
        .set_description(format!("Save changes to {file_name} before continuing?"))
        .set_buttons(MessageButtons::YesNoCancelCustom(
            "Save".to_string(),
            "Discard".to_string(),
            "Cancel".to_string(),
        ))
        .show();

    match result {
        MessageDialogResult::Custom(choice) if choice == "Save" => PendingChangesAction::Save,
        MessageDialogResult::Custom(choice) if choice == "Discard" => PendingChangesAction::Discard,
        _ => PendingChangesAction::Cancel,
    }
}
