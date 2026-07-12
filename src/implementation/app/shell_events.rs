use std::sync::atomic::Ordering;

use super::document_actions::{open_document, reload_workspace_documents_from_disk};
use super::runtime::AppRuntime;
use super::workspace_view::{present_workspace, render_status};
use super::{UserEvent, dispatch_user_event, log_event};

pub(super) fn handle_shell_ready(runtime: &mut AppRuntime) {
    let shell_was_ready = runtime.shell.ready;
    runtime.shell.ready = true;

    if (shell_was_ready || runtime.shell.recovery_pending)
        && runtime.workspace.active_document().is_some()
    {
        runtime.shell.recovery_pending = false;
        let status = reload_workspace_documents_from_disk(&mut runtime.workspace)
            .unwrap_or_else(|message| message);
        present_workspace(
            &runtime.window,
            &runtime.webview,
            &runtime.workspace,
            &status,
            true,
        );
    }

    for request in runtime.shell.pending_open_requests.drain(..) {
        dispatch_user_event(
            &runtime.proxy,
            "shell-ready-flush",
            UserEvent::OpenPath(request),
        );
    }
}

pub(super) fn recover_shell(url: String, runtime: &mut AppRuntime) {
    log_event(
        "user_event.received",
        None,
        "handling UserEvent::RecoverShell",
        format!("url={url}"),
    );
    runtime.shell.ready = false;
    runtime.shell.recovery_pending = true;
    runtime
        .shell
        .suppress_blank_recovery
        .store(true, Ordering::SeqCst);

    if let Err(error) = runtime.webview.load_html(&super::shell::app_shell_html()) {
        runtime.shell.recovery_pending = false;
        runtime
            .shell
            .suppress_blank_recovery
            .store(false, Ordering::SeqCst);
        render_status(
            &runtime.webview,
            &format!("Failed to recover the document view: {error}"),
            "error",
        );
    }
}

pub(super) fn open_documentation(runtime: &mut AppRuntime) {
    match super::documentation::documentation_markdown_path() {
        Some(path) => open_document(
            &runtime.window,
            &runtime.webview,
            &mut runtime.workspace,
            &path,
            None,
        ),
        None => render_status(&runtime.webview, "Documentation file not found.", "error"),
    }
}
