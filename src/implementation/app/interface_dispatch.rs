use std::sync::atomic::Ordering;

use tao::event_loop::EventLoopProxy;

use super::interface_constants::{NEXT_EVENT_ID, PANIC_HOOK_ONCE};
use super::interface_types::{ActionContext, UserEvent};
use super::log_event;

pub(crate) fn next_event_id() -> u64 {
    NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed)
}

pub(crate) fn new_action_context(source: &'static str) -> ActionContext {
    ActionContext {
        event_id: next_event_id(),
        source,
    }
}

pub(crate) fn install_panic_hook() {
    PANIC_HOOK_ONCE.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let location = info
                .location()
                .map(|location| format!("{}:{}", location.file(), location.line()))
                .unwrap_or_else(|| "<unknown>".to_string());
            let payload = info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("<non-string panic>");
            log_event(
                "panic",
                None,
                "application panic",
                format!("location={location} payload={payload:?}"),
            );
        }));
    });
}

pub(crate) fn dispatch_user_event(
    proxy: &EventLoopProxy<UserEvent>,
    stage_source: &'static str,
    event: UserEvent,
) {
    let (event_id, event_name, details) = describe_user_event(stage_source, &event);
    log_event("send_event.begin", event_id, event_name, &details);
    match proxy.send_event(event) {
        Ok(()) => log_event(
            "send_event.end",
            event_id,
            event_name,
            format!("{details} result=ok"),
        ),
        Err(error) => log_event(
            "send_event.end",
            event_id,
            event_name,
            format!("{details} result=err error={error}"),
        ),
    }
}

fn describe_user_event(
    stage_source: &'static str,
    event: &UserEvent,
) -> (Option<u64>, &'static str, String) {
    match event {
        UserEvent::NewDocument => (None, "NewDocument", format!("source={stage_source}")),
        UserEvent::OpenFile(ctx) => (
            Some(ctx.event_id),
            "OpenFile",
            format!("source={} origin={}", stage_source, ctx.source),
        ),
        UserEvent::OpenPath(request) => (
            Some(request.ctx.event_id),
            "OpenPath",
            format!(
                "source={} origin={} path={}",
                stage_source,
                request.ctx.source,
                request.path.display()
            ),
        ),
        UserEvent::ActivateDocument(document_id) => (
            None,
            "ActivateDocument",
            format!("source={} document_id={document_id}", stage_source),
        ),
        UserEvent::ActivateNextDocument => (
            None,
            "ActivateNextDocument",
            format!("source={stage_source}"),
        ),
        UserEvent::ActivatePreviousDocument => (
            None,
            "ActivatePreviousDocument",
            format!("source={stage_source}"),
        ),
        UserEvent::CloseDocument(document_id) => (
            None,
            "CloseDocument",
            format!("source={} document_id={document_id}", stage_source),
        ),
        UserEvent::CloseCurrentDocument => (
            None,
            "CloseCurrentDocument",
            format!("source={stage_source}"),
        ),
        UserEvent::CloseOtherDocuments => (
            None,
            "CloseOtherDocuments",
            format!("source={stage_source}"),
        ),
        UserEvent::CloseAllDocuments => {
            (None, "CloseAllDocuments", format!("source={stage_source}"))
        }
        UserEvent::ShellReady => (None, "ShellReady", format!("source={stage_source}")),
        UserEvent::RecoverShell(url) => (
            None,
            "RecoverShell",
            format!("source={} url={url}", stage_source),
        ),
        UserEvent::OpenExternal(href) => (
            None,
            "OpenExternal",
            format!("source={} href={href}", stage_source),
        ),
        UserEvent::SaveDocument => (None, "SaveDocument", format!("source={stage_source}")),
        UserEvent::SaveDocumentAs => (None, "SaveDocumentAs", format!("source={stage_source}")),
        UserEvent::ExportPdf => (None, "ExportPdf", format!("source={stage_source}")),
        UserEvent::ExportHtml => (None, "ExportHtml", format!("source={stage_source}")),
        UserEvent::PrintDocument => (None, "PrintDocument", format!("source={stage_source}")),
        UserEvent::OpenFind => (None, "OpenFind", format!("source={stage_source}")),
        UserEvent::ToggleMode => (None, "ToggleMode", format!("source={stage_source}")),
        UserEvent::EditorChanged(markdown) => (
            None,
            "EditorChanged",
            format!("source={} bytes={}", stage_source, markdown.len()),
        ),
        UserEvent::ShowAbout => (None, "ShowAbout", format!("source={stage_source}")),
        UserEvent::OpenDocumentation => {
            (None, "OpenDocumentation", format!("source={stage_source}"))
        }
        UserEvent::Exit => (None, "Exit", format!("source={stage_source}")),
    }
}
