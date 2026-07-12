use tao::event::{ElementState, WindowEvent};
use tao::event_loop::ControlFlow;

use super::close_actions::resolve_all_pending_changes;
use super::runtime::AppRuntime;
use super::shortcuts::handle_command_shortcut;
use super::workspace_view::render_status;
use super::{UserEvent, dispatch_user_event, log_event, new_action_context};

pub(super) fn handle_window_event(
    event: WindowEvent,
    runtime: &mut AppRuntime,
    control_flow: &mut ControlFlow,
) {
    match event {
        WindowEvent::CloseRequested => handle_close_requested(runtime, control_flow),
        WindowEvent::ModifiersChanged(next_modifiers) => runtime.modifiers = next_modifiers,
        WindowEvent::KeyboardInput { event, .. } => {
            if event.state == ElementState::Released && runtime.modifiers.super_key() {
                handle_command_shortcut(&runtime.proxy, event.physical_key);
            }
        }
        WindowEvent::HoveredFile(path) => {
            render_status(
                &runtime.webview,
                &format!("Drop to open: {}", path.display()),
                "info",
            );
        }
        WindowEvent::HoveredFileCancelled => {
            render_status(
                &runtime.webview,
                "Ready. Open a Markdown file or press Command+O.",
                "info",
            );
        }
        WindowEvent::DroppedFile(path) => {
            let ctx = new_action_context("window-dropped-file");
            log_event(
                "window.dropped_file",
                Some(ctx.event_id),
                "window dropped file",
                format!("path={}", path.display()),
            );
            dispatch_user_event(
                &runtime.proxy,
                "window-drop",
                UserEvent::OpenPath(super::OpenPathRequest { ctx, path }),
            );
        }
        _ => {}
    }
}

fn handle_close_requested(runtime: &mut AppRuntime, control_flow: &mut ControlFlow) {
    log_event("window.close_requested", None, "window close requested", "");
    if resolve_all_pending_changes(&runtime.window, &runtime.webview, &mut runtime.workspace) {
        *control_flow = ControlFlow::Exit;
    }
}
