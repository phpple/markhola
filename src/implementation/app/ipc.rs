use serde_json::Value;
use tao::event_loop::EventLoopProxy;

use super::{UserEvent, dispatch_user_event, log_event, new_action_context};

pub(super) fn handle_ipc_message(proxy: &EventLoopProxy<UserEvent>, payload: String) {
    log_event(
        "ipc.received",
        None,
        "ipc payload received",
        format!("payload={payload}"),
    );
    let Ok(value) = serde_json::from_str::<Value>(&payload) else {
        log_event("ipc.error", None, "ipc payload parsing failed", "");
        return;
    };

    match value.get("kind").and_then(Value::as_str) {
        Some("open-file") => {
            let ctx = new_action_context("ipc-open-file");
            dispatch_user_event(proxy, "ipc", UserEvent::OpenFile(ctx));
        }
        Some("shell-ready") => dispatch_user_event(proxy, "ipc", UserEvent::ShellReady),
        Some("toggle-mode") => dispatch_user_event(proxy, "ipc", UserEvent::ToggleMode),
        Some("close-current-document") => {
            dispatch_user_event(proxy, "ipc", UserEvent::CloseCurrentDocument)
        }
        Some("request-save") => dispatch_user_event(proxy, "ipc", UserEvent::SaveDocument),
        Some("request-save-as") => dispatch_user_event(proxy, "ipc", UserEvent::SaveDocumentAs),
        Some("request-export-pdf") => dispatch_user_event(proxy, "ipc", UserEvent::ExportPdf),
        Some("request-export-html") => dispatch_user_event(proxy, "ipc", UserEvent::ExportHtml),
        Some("request-print") => dispatch_user_event(proxy, "ipc", UserEvent::PrintDocument),
        Some("request-open-find") => dispatch_user_event(proxy, "ipc", UserEvent::OpenFind),
        Some("request-exit") => dispatch_user_event(proxy, "ipc", UserEvent::Exit),
        Some("open-external") => {
            dispatch_string_event(value.get("href"), proxy, UserEvent::OpenExternal)
        }
        Some("activate-document") => {
            dispatch_u64_event(value.get("documentId"), proxy, UserEvent::ActivateDocument)
        }
        Some("close-document") => {
            dispatch_u64_event(value.get("documentId"), proxy, UserEvent::CloseDocument)
        }
        Some("editor-changed") => {
            dispatch_string_event(value.get("markdown"), proxy, UserEvent::EditorChanged)
        }
        _ => {}
    }
}

fn dispatch_string_event(
    value: Option<&Value>,
    proxy: &EventLoopProxy<UserEvent>,
    build: fn(String) -> UserEvent,
) {
    if let Some(value) = value.and_then(Value::as_str) {
        dispatch_user_event(proxy, "ipc", build(value.to_string()));
    }
}

fn dispatch_u64_event(
    value: Option<&Value>,
    proxy: &EventLoopProxy<UserEvent>,
    build: fn(u64) -> UserEvent,
) {
    if let Some(value) = value.and_then(Value::as_u64) {
        dispatch_user_event(proxy, "ipc", build(value));
    }
}
