use tao::event::{Event, StartCause};
use tao::event_loop::ControlFlow;

use super::runtime::AppRuntime;
use super::window_events::handle_window_event;
use super::workspace_view::render_status;
use super::{UserEvent, log_event};

pub(super) fn handle_event(
    event: Event<'_, UserEvent>,
    runtime: &mut AppRuntime,
    control_flow: &mut ControlFlow,
) {
    *control_flow = ControlFlow::Wait;

    match event {
        Event::NewEvents(StartCause::Init) => {
            log_event("event_loop.init", None, "event loop init", "");
            render_status(
                &runtime.webview,
                "Ready. Open a Markdown file or press Command+O.",
                "info",
            );
        }
        Event::Opened { urls } => handle_opened_urls(urls, runtime),
        Event::WindowEvent { event, .. } => handle_window_event(event, runtime, control_flow),
        Event::UserEvent(user_event) => {
            super::user_events::handle_user_event(user_event, runtime, control_flow)
        }
        _ => {}
    }
}

fn handle_opened_urls(urls: Vec<url::Url>, runtime: &mut AppRuntime) {
    log_event(
        "tao.opened.begin",
        None,
        "received Event::Opened",
        format!("urls={urls:?}"),
    );
    if let Some(url) = urls.into_iter().find(|url| url.scheme() == "file") {
        match url.to_file_path() {
            Ok(path) => {
                let ctx = super::new_action_context("tao-opened");
                log_event(
                    "tao.opened.path",
                    Some(ctx.event_id),
                    "resolved file path from Event::Opened",
                    format!("path={}", path.display()),
                );
                super::dispatch_user_event(
                    &runtime.proxy,
                    "tao-opened",
                    UserEvent::OpenPath(super::OpenPathRequest { ctx, path }),
                );
            }
            Err(_) => {
                log_event(
                    "tao.opened.error",
                    None,
                    "failed to convert Event::Opened URL to file path",
                    "",
                );
                render_status(
                    &runtime.webview,
                    "The requested file path is not valid.",
                    "error",
                );
            }
        }
    }
}
