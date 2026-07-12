use tao::event_loop::EventLoopProxy;
use tao::keyboard::KeyCode;

use super::{UserEvent, dispatch_user_event, log_event, new_action_context};

pub(super) fn handle_command_shortcut(proxy: &EventLoopProxy<UserEvent>, key: KeyCode) {
    match key {
        KeyCode::KeyO => {
            let ctx = new_action_context("keyboard-command-o");
            log_event(
                "keyboard.shortcut",
                Some(ctx.event_id),
                "keyboard shortcut triggered",
                "key=Command+O",
            );
            dispatch_user_event(proxy, "keyboard", UserEvent::OpenFile(ctx));
        }
        KeyCode::KeyS => emit_shortcut(proxy, UserEvent::SaveDocument, "Command+S"),
        KeyCode::KeyP => emit_shortcut(proxy, UserEvent::PrintDocument, "Command+P"),
        KeyCode::KeyF => emit_shortcut(proxy, UserEvent::OpenFind, "Command+F"),
        KeyCode::KeyW => emit_shortcut(proxy, UserEvent::CloseCurrentDocument, "Command+W"),
        KeyCode::Slash => emit_shortcut(proxy, UserEvent::ToggleMode, "Command+/"),
        _ => {}
    }
}

fn emit_shortcut(proxy: &EventLoopProxy<UserEvent>, event: UserEvent, key: &str) {
    log_event(
        "keyboard.shortcut",
        None,
        "keyboard shortcut triggered",
        format!("key={key}"),
    );
    dispatch_user_event(proxy, "keyboard", event);
}
