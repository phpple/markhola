use tao::event_loop::ControlFlow;
use tao::window::Fullscreen;

use crate::app::AppTheme;
use crate::document::DocumentMode;
use crate::render_assets;

use super::close_actions::close_document_tab;
use super::document_actions::{create_blank_document, open_document, open_documents_dialog};
use super::export_actions;
use super::navigation_actions::{
    activate_document, close_all_documents, close_current_document, close_other_documents,
    exit_application, switch_document,
};
use super::runtime::AppRuntime;
use super::save_actions::{save_active_document, save_active_document_as};
use super::shell_events::{handle_shell_ready, open_documentation, recover_shell};
use super::theme_preferences;
use super::workspace_view::{
    present_workspace, render_about, render_status, sync_native_theme_state, sync_workspace_state,
};
use super::{OpenPathRequest, UserEvent, log_event};

pub(super) fn handle_user_event(
    user_event: UserEvent,
    runtime: &mut AppRuntime,
    control_flow: &mut ControlFlow,
) {
    match user_event {
        UserEvent::NewDocument => {
            create_blank_document(
                &runtime.window,
                &runtime.webview,
                &runtime.native_footer,
                &mut runtime.workspace,
            );
        }
        UserEvent::OpenFile(ctx) => handle_open_file(ctx, runtime),
        UserEvent::OpenPath(request) => handle_open_path(request, runtime),
        UserEvent::ActivateDocument(document_id) => activate_document(document_id, runtime),
        UserEvent::ActivateNextDocument => switch_document(runtime, true),
        UserEvent::ActivatePreviousDocument => switch_document(runtime, false),
        UserEvent::CloseDocument(document_id) => {
            close_document_tab(
                &runtime.window,
                &runtime.webview,
                &runtime.native_footer,
                &mut runtime.workspace,
                document_id,
                "Document closed.",
                &runtime.asset_access,
            );
        }
        UserEvent::CloseCurrentDocument => close_current_document(runtime, control_flow),
        UserEvent::CloseOtherDocuments => close_other_documents(runtime),
        UserEvent::CloseAllDocuments => close_all_documents(runtime),
        UserEvent::ShellReady => handle_shell_ready(runtime),
        UserEvent::RecoverShell(url) => recover_shell(url, runtime),
        UserEvent::OpenExternal(href) => open_external_link(&href, runtime),
        UserEvent::SaveDocument => {
            save_active_document(
                &runtime.window,
                &runtime.webview,
                &runtime.native_footer,
                &mut runtime.workspace,
                &runtime.asset_access,
            );
        }
        UserEvent::SaveDocumentAs => {
            save_active_document_as(
                &runtime.window,
                &runtime.webview,
                &runtime.native_footer,
                &mut runtime.workspace,
                &runtime.asset_access,
            );
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
        UserEvent::SelectTheme(theme) => select_theme(theme, runtime),
        UserEvent::ToggleFullscreen => toggle_fullscreen(runtime),
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
    match open_documents_dialog(ctx.event_id) {
        Some(paths) => {
            let mut failures = 0usize;
            for path in paths {
                let before_active = runtime.workspace.active_document_id();
                open_document(
                    &runtime.window,
                    &runtime.webview,
                    &runtime.native_footer,
                    &mut runtime.workspace,
                    &path,
                    Some(ctx.event_id),
                    &runtime.asset_access,
                );
                let after_active = runtime.workspace.active_document_id();
                if after_active == before_active && runtime.workspace.find_by_path(&path).is_none() {
                    failures += 1;
                }
            }
            if failures > 0 {
                render_status(&runtime.webview, "Some files failed to open.", "error");
            }
        }
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
        &runtime.native_footer,
        &mut runtime.workspace,
        &path,
        Some(ctx.event_id),
        &runtime.asset_access,
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
            &runtime.native_footer,
            &runtime.workspace,
            status,
            true,
        ),
        None => render_status(&runtime.webview, "No document opened.", "error"),
    }
}

fn select_theme(theme: AppTheme, runtime: &mut AppRuntime) {
    runtime.selected_theme = theme;
    theme_preferences::save_selected_theme(theme);
    sync_native_theme_state(theme);
    let css = render_assets::load_app_theme_css_for_inline_style(theme.key());
    match serde_json::to_string(&css) {
        Ok(serialized_css) => {
            let script = format!("window.applyAppTheme({serialized_css});");
            if let Err(error) = runtime.webview.evaluate_script(&script) {
                render_status(
                    &runtime.webview,
                    &format!("Failed to apply theme: {error}"),
                    "error",
                );
                return;
            }
            render_status(
                &runtime.webview,
                &format!("Theme switched to {}.", theme.label()),
                "info",
            );
        }
        Err(error) => render_status(
            &runtime.webview,
            &format!("Failed to serialize theme CSS: {error}"),
            "error",
        ),
    }
}

fn toggle_fullscreen(runtime: &mut AppRuntime) {
    let next_state = if runtime.window.fullscreen().is_some() {
        None
    } else {
        Some(Fullscreen::Borderless(None))
    };
    runtime.window.set_fullscreen(next_state);
    let message = if runtime.window.fullscreen().is_some() {
        "Entered fullscreen."
    } else {
        "Exited fullscreen."
    };
    render_status(&runtime.webview, message, "info");
}

fn editor_changed(markdown: String, runtime: &mut AppRuntime) {
    if let Some(document) = runtime.workspace.active_document_mut() {
        document.update_markdown(markdown);
        sync_workspace_state(
            &runtime.window,
            &runtime.webview,
            &runtime.native_footer,
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
