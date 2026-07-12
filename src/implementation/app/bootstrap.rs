use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tao::dpi::LogicalSize;
use tao::event_loop::{EventLoop, EventLoopBuilder, EventLoopProxy};
use tao::window::WindowBuilder;
use wry::{PageLoadEvent, WebView, WebViewBuilder};

use super::runtime::AppRuntime;
use super::shell::{app_shell_html, should_dispatch_shell_recovery};
use super::{UserEvent, WINDOW_TITLE, dispatch_user_event, log_event, macos_menu};

pub(super) fn build_runtime() -> Result<(EventLoop<UserEvent>, AppRuntime), Box<dyn Error>> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let suppress_blank_recovery = Arc::new(AtomicBool::new(true));

    let window = WindowBuilder::new()
        .with_title(WINDOW_TITLE)
        .with_inner_size(LogicalSize::new(1120.0, 760.0))
        .with_min_inner_size(LogicalSize::new(800.0, 560.0))
        .build(&event_loop)?;
    let webview = build_webview(&window, &proxy, Arc::clone(&suppress_blank_recovery))?;

    #[cfg(target_os = "macos")]
    macos_menu::install(&proxy)?;

    let runtime = AppRuntime::new(proxy, window, webview, suppress_blank_recovery);
    Ok((event_loop, runtime))
}

fn build_webview(
    window: &tao::window::Window,
    proxy: &EventLoopProxy<UserEvent>,
    suppress_blank_recovery: Arc<AtomicBool>,
) -> Result<WebView, wry::Error> {
    let ipc_proxy = proxy.clone();
    let page_load_proxy = proxy.clone();

    WebViewBuilder::new()
        .with_html(app_shell_html())
        .with_devtools(true)
        .with_ipc_handler(move |request| {
            super::ipc::handle_ipc_message(&ipc_proxy, request.body().to_owned());
        })
        .with_on_page_load_handler(move |event, url| {
            let event_name = match event {
                PageLoadEvent::Started => "started",
                PageLoadEvent::Finished => "finished",
            };
            log_event(
                "webview.page_load",
                None,
                "webview page load event",
                format!("event={event_name} url={url}"),
            );
            if matches!(event, PageLoadEvent::Finished)
                && should_dispatch_shell_recovery(&url, &suppress_blank_recovery)
            {
                dispatch_user_event(&page_load_proxy, "page-load", UserEvent::RecoverShell(url));
            }
        })
        .build(window)
}
