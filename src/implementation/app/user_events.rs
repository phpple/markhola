use tao::event_loop::ControlFlow;

use crate::document::DocumentMode;

use super::close_actions::close_document_tab;
use super::document_actions::{open_document, open_document_dialog};
use super::export_actions;
use super::navigation_actions::{
    activate_document, close_all_documents, close_current_document, close_other_documents,
    exit_application, switch_document,
};
use super::runtime::AppRuntime;
use super::save_actions::{save_active_document, save_active_document_as};
use super::shell_events::{handle_shell_ready, open_documentation, recover_shell};
use super::workspace_view::{present_workspace, render_about, render_status, sync_workspace_state};
use super::{OpenPathRequest, UserEvent, log_event};

pub(super) fn handle_user_event(
    user_event: UserEvent,
    runtime: &mut AppRuntime,
    control_flow: &mut ControlFlow,
) {
    match user_event {
        UserEvent::OpenFile(ctx) => handle_open_file(ctx, runtime),
        UserEvent::OpenPath(request) => handle_open_path(request, runtime),
        UserEvent::ActivateDocument(document_id) => activate_document(document_id, runtime),
        UserEvent::ActivateNextDocument => switch_document(runtime, true),
        UserEvent::ActivatePreviousDocument => switch_document(runtime, false),
        UserEvent::CloseDocument(document_id) => {
            close_document_tab(
                &runtime.window,
                &runtime.webview,
                &mut runtime.workspace,
                document_id,
                "Document closed.",
            );
        }
        UserEvent::CloseCurrentDocument => close_current_document(runtime, control_flow),
        UserEvent::CloseOtherDocuments => close_other_documents(runtime),
        UserEvent::CloseAllDocuments => close_all_documents(runtime),
        UserEvent::ShellReady => handle_shell_ready(runtime),
        UserEvent::RecoverShell(url) => recover_shell(url, runtime),
        UserEvent::OpenExternal(href) => open_external_link(&href, runtime),
        UserEvent::SaveDocument => {
            save_active_document(&runtime.window, &runtime.webview, &mut runtime.workspace);
        }
        UserEvent::SaveDocumentAs => {
            save_active_document_as(&runtime.window, &runtime.webview, &mut runtime.workspace);
        }
        UserEvent::ExportPdf => export_actions::export_pdf(&runtime.webview, &runtime.workspace),
        UserEvent::ExportHtml => export_actions::export_html(&runtime.webview, &runtime.workspace),
        UserEvent::PrintDocument => {
            export_actions::print_document(&runtime.webview, &runtime.workspace)
        }
        UserEvent::OpenFind => {
            export_actions::open_find_panel(&runtime.webview, &runtime.workspace)
        }
        UserEvent::ToggleMode => toggle_mode(runtime),
        UserEvent::EditorChanged(markdown) => editor_changed(markdown, runtime),
        UserEvent::ShowAbout => render_about(&runtime.webview),
        UserEvent::OpenDocumentation => open_documentation(runtime),
        UserEvent::Exit => exit_application(runtime, control_flow),
    }
}

fn handle_open_file(ctx: super::ActionContext, runtime: &mut AppRuntime) {
    log_event(
        "user_event.received",
        Some(ctx.event_id),
        "handling UserEvent::OpenFile",
        format!("source={}", ctx.source),
    );
    match open_document_dialog(ctx.event_id) {
        Some(path) => open_document(
            &runtime.window,
            &runtime.webview,
            &mut runtime.workspace,
            &path,
            Some(ctx.event_id),
        ),
        None => render_status(&runtime.webview, "Open cancelled.", "info"),
    }
}

fn handle_open_path(request: OpenPathRequest, runtime: &mut AppRuntime) {
    let OpenPathRequest { ctx, path } = request;
    log_event(
        "user_event.received",
        Some(ctx.event_id),
        "handling UserEvent::OpenPath",
        format!("source={} path={}", ctx.source, path.display()),
    );
    if !runtime.shell.ready {
        runtime
            .shell
            .pending_open_requests
            .push(OpenPathRequest { ctx, path });
        return;
    }
    open_document(
        &runtime.window,
        &runtime.webview,
        &mut runtime.workspace,
        &path,
        Some(ctx.event_id),
    );
}

fn toggle_mode(runtime: &mut AppRuntime) {
    let status = runtime.workspace.active_document_mut().map(|document| {
        document.toggle_mode();
        match document.mode() {
            DocumentMode::Readonly => "Readonly preview updated.",
            DocumentMode::Writable => "Writable mode enabled.",
        }
    });
    match status {
        Some(status) => present_workspace(
            &runtime.window,
            &runtime.webview,
            &runtime.workspace,
            status,
            true,
        ),
        None => render_status(&runtime.webview, "No document opened.", "error"),
    }
}

fn editor_changed(markdown: String, runtime: &mut AppRuntime) {
    if let Some(document) = runtime.workspace.active_document_mut() {
        document.update_markdown(markdown);
        sync_workspace_state(
            &runtime.window,
            &runtime.webview,
            &runtime.workspace,
            "Unsaved changes.",
        );
    }
}

fn open_external_link(href: &str, runtime: &AppRuntime) {
    if let Err(error) = open::that(href) {
        log_event(
            "open_external.error",
            None,
            "open external failed",
            format!("error={error}"),
        );
        render_status(
            &runtime.webview,
            &format!("Failed to open link: {error}"),
            "error",
        );
    }
}
