use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tao::dpi::LogicalSize;
use tao::event_loop::{EventLoop, EventLoopBuilder, EventLoopProxy};
use tao::window::WindowBuilder;
use wry::{PageLoadEvent, WebContext, WebView, WebViewBuilder};

use super::runtime::AppRuntime;
use super::shell::{APP_SHELL_URL, app_shell_html, should_dispatch_shell_recovery};
use super::{APP_AUTHOR, UserEvent, WINDOW_TITLE, dispatch_user_event, log_event};
#[cfg(target_os = "macos")]
use super::macos_menu;

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
    let mut web_context = WebContext::new(webview_data_directory().ok());
    let builder = WebViewBuilder::new_with_web_context(&mut web_context);

    #[cfg(target_os = "windows")]
    let builder = builder
        .with_custom_protocol("markhola".into(), |_webview_id, _request| {
            wry::http::Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(std::borrow::Cow::Owned(app_shell_html().into_bytes()))
                .unwrap()
        })
        .with_url(APP_SHELL_URL);

    #[cfg(not(target_os = "windows"))]
    let builder = builder.with_html(app_shell_html());

    builder
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

fn webview_data_directory() -> Result<PathBuf, std::io::Error> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = base.join(APP_AUTHOR).join("MarkHola").join("WebView2");
        std::fs::create_dir_all(&path)?;
        return Ok(path);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let path = std::env::temp_dir().join("markhola").join("webview");
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }
}
