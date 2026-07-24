use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tao::dpi::LogicalSize;
use tao::event_loop::{EventLoop, EventLoopBuilder, EventLoopProxy};
use tao::window::WindowBuilder;
use url::Url;
use wry::{DragDropEvent, PageLoadEvent, WebView, WebViewBuilder};
use wry::http::{Response, header};

use crate::app::AppTheme;

use super::runtime::AppRuntime;
use super::asset_access::{AssetAccessRegistry, new_registry, resolve_asset};
use super::native_footer::NativeFooter;
use super::shell::{app_shell_html, should_dispatch_shell_recovery};
use super::theme_preferences;
use super::{UserEvent, WINDOW_TITLE, dispatch_user_event, log_event, macos_menu};

fn is_markdown_path(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown")
}

fn local_asset_content_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()).map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub(super) fn build_runtime() -> Result<(EventLoop<UserEvent>, AppRuntime), Box<dyn Error>> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let suppress_blank_recovery = Arc::new(AtomicBool::new(true));
    let asset_access = new_registry();
    let selected_theme = theme_preferences::load_selected_theme();

    let window = WindowBuilder::new()
        .with_title(WINDOW_TITLE)
        .with_inner_size(LogicalSize::new(1120.0, 760.0))
        .with_min_inner_size(LogicalSize::new(800.0, 560.0))
        .build(&event_loop)?;
    let webview = build_webview(
        &window,
        &proxy,
        Arc::clone(&suppress_blank_recovery),
        Arc::clone(&asset_access),
        selected_theme,
    )?;
    let native_footer = NativeFooter::install(&window, &webview, selected_theme);

    #[cfg(target_os = "macos")]
    macos_menu::install(&proxy)?;

    let runtime = AppRuntime::new(
        proxy,
        window,
        webview,
        suppress_blank_recovery,
        asset_access,
        native_footer,
        selected_theme,
    );
    runtime.native_footer.sync(&runtime.workspace, "Ready.");
    Ok((event_loop, runtime))
}

fn build_webview(
    window: &tao::window::Window,
    proxy: &EventLoopProxy<UserEvent>,
    suppress_blank_recovery: Arc<AtomicBool>,
    asset_access: AssetAccessRegistry,
    selected_theme: AppTheme,
) -> Result<WebView, wry::Error> {
    let ipc_proxy = proxy.clone();
    let page_load_proxy = proxy.clone();
    let navigation_proxy = proxy.clone();
    let drag_drop_proxy = proxy.clone();

    WebViewBuilder::new()
        .with_html(app_shell_html(selected_theme))
        .with_devtools(true)
        .with_drag_drop_handler(move |event| {
            match event {
                DragDropEvent::Enter { paths, .. } => {
                    log_event(
                        "webview.drag_drop.enter",
                        None,
                        "files entered webview",
                        format!("paths={paths:?}"),
                    );
                }
                DragDropEvent::Drop { paths, .. } => {
                    log_event(
                        "webview.drag_drop.drop",
                        None,
                        "files dropped on webview",
                        format!("paths={paths:?}"),
                    );
                    for path in paths {
                        let ctx = super::new_action_context("webview-drop");
                        dispatch_user_event(
                            &drag_drop_proxy,
                            "webview-drop",
                            UserEvent::OpenPath(super::OpenPathRequest { ctx, path }),
                        );
                    }
                }
                DragDropEvent::Leave | DragDropEvent::Over { .. } => {}
                _ => {}
            }

            true
        })
        .with_custom_protocol("markhola-file".to_string(), move |_id, request| {
            let uri = request.uri().to_string();
            log_event(
                "webview.protocol.markhola_file.request",
                None,
                "markhola-file protocol request",
                uri.as_str(),
            );
            let Ok(parsed) = Url::parse(&uri) else {
                log_event(
                    "webview.protocol.markhola_file",
                    None,
                    "failed to parse markhola-file URL",
                    uri.as_str(),
                );
                return Response::builder()
                    .status(400)
                    .body(std::borrow::Cow::Borrowed(b"bad url" as &[u8]))
                    .unwrap();
            };
            let Some(document_id) = parsed
                .host_str()
                .filter(|host| *host == "asset")
                .and_then(|_| parsed.path_segments())
                .and_then(|mut segments| segments.next())
                .and_then(|segment| segment.parse::<u64>().ok())
            else {
                return Response::builder().status(403).body(std::borrow::Cow::Borrowed(b"forbidden" as &[u8])).unwrap();
            };
            let raw_relative = parsed.path().strip_prefix(&format!("/{document_id}/")).unwrap_or("");
            let relative = match percent_encoding::percent_decode_str(raw_relative).decode_utf8() {
                Ok(value) => value,
                Err(_) => return Response::builder().status(400).body(std::borrow::Cow::Borrowed(b"bad path" as &[u8])).unwrap(),
            };
            match resolve_asset(&asset_access, document_id, &relative) {
                Ok(path) => match std::fs::read(&path) {
                Ok(bytes) => {
                    log_event(
                        "webview.protocol.markhola_file.response",
                        None,
                        "served local asset",
                        format!(
                            "uri={uri} path={} bytes={}",
                            path.display(),
                            bytes.len()
                        ),
                    );
                    Response::builder()
                        .status(200)
                        .header(header::CONTENT_TYPE, local_asset_content_type(&path))
                        .body(std::borrow::Cow::Owned(bytes))
                        .unwrap()
                }
                Err(error) => {
                    log_event(
                        "webview.protocol.markhola_file",
                        None,
                        "failed to load local asset",
                        format!("uri={uri} path={} error={error}", path.display()),
                    );
                    log_event(
                        "webview.protocol.markhola_file.response",
                        None,
                        "local asset not found",
                        format!("uri={uri} path={} status=404", path.display()),
                    );
                    Response::builder()
                        .status(404)
                        .body(std::borrow::Cow::Borrowed(b"not found" as &[u8]))
                        .unwrap()
                }
            },
                Err(error) => Response::builder()
                    .status(error.status_code())
                    .body(std::borrow::Cow::Borrowed(b"forbidden" as &[u8]))
                    .unwrap(),
            }
        })
        .with_navigation_handler(move |url| {
            if url.starts_with("file://") && is_markdown_path(&url) {
                log_event(
                    "webview.navigation.blocked",
                    None,
                    "blocked WKWebView navigation to markdown file URL",
                    url.as_str(),
                );
                let ctx = super::new_action_context("webview-navigation");
                if let Ok(parsed) = Url::parse(&url) {
                    if let Ok(path) = parsed.to_file_path() {
                        dispatch_user_event(
                            &navigation_proxy,
                            "webview-navigation",
                            UserEvent::OpenPath(super::OpenPathRequest { ctx, path }),
                        );
                        return false;
                    }
                }
                if let Some(path) = url
                    .strip_prefix("file://")
                    .and_then(|rest| rest.split('?').next())
                    .filter(|value| !value.is_empty())
                {
                    dispatch_user_event(
                        &navigation_proxy,
                        "webview-navigation",
                        UserEvent::OpenPath(super::OpenPathRequest {
                            ctx,
                            path: std::path::PathBuf::from(path),
                        }),
                    );
                }
                return false;
            }
            true
        })
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
