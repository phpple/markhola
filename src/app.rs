use std::ffi::{c_char, c_long, CStr};
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use serde::Serialize;
use serde_json::Value;
use tao::dpi::LogicalSize;
use tao::event::{ElementState, Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::keyboard::{KeyCode, ModifiersState};
use tao::window::{Window, WindowBuilder};
use wry::{WebView, WebViewBuilder};

use crate::document::{ActiveDocument, DocumentSnapshot, DocumentMode};
use crate::file_io;

const WINDOW_TITLE: &str = "MarkHola";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_AUTHOR: &str = "Ronnie Deng";
const APP_GITHUB_URL: &str = "https://github.com/phpple/markhola";
const APP_BUILD_TARGET: &str = std::env::consts::ARCH;
const APP_BUILD_PLATFORM: &str = std::env::consts::OS;
const MERMAID_RUNTIME: &str = include_str!("../assets/mermaid/mermaid.min.js");
const MATHJAX_RUNTIME: &str = include_str!("../assets/mathjax/tex-svg-full.js");
const DEBUG_LOG_DIR: &str = "/var/log/markhola";
const DEBUG_LOG_FALLBACK_PATH: &str = "/tmp/markhola.log";
static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);
static PANIC_HOOK_ONCE: Once = Once::new();

#[derive(Clone, Debug)]
enum UserEvent {
    OpenFile(ActionContext),
    OpenPath(OpenPathRequest),
    ShellReady,
    OpenExternal(String),
    SaveDocument,
    ToggleMode,
    EditorChanged(String),
    ShowAbout,
    Exit,
}

#[derive(Clone, Debug)]
enum PendingChangesAction {
    Save,
    Discard,
    Cancel,
}

#[derive(Clone, Debug, Serialize)]
struct StatusPayload<'a> {
    message: &'a str,
    level: &'a str,
}

#[derive(Clone, Debug, Serialize)]
struct DocumentPresentation<'a> {
    #[serde(flatten)]
    document: &'a DocumentSnapshot,
    status_message: &'a str,
}

#[derive(Clone, Debug)]
struct ActionContext {
    event_id: u64,
    source: &'static str,
}

#[derive(Clone, Debug)]
struct OpenPathRequest {
    ctx: ActionContext,
    path: PathBuf,
}

#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

unsafe extern "C" {
    fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
    fn strftime(s: *mut c_char, max: usize, format: *const c_char, tm: *const Tm) -> usize;
}

fn current_date_stamp() -> Option<String> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    let mut tm = Tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };

    // SAFETY: localtime_r and strftime are called with valid pointers and fixed-size buffers.
    unsafe {
        if localtime_r(&seconds, &mut tm).is_null() {
            return None;
        }

        let format = b"%Y%m%d\0";
        let mut buffer = [0 as c_char; 16];
        let written = strftime(buffer.as_mut_ptr(), buffer.len(), format.as_ptr().cast(), &tm);
        if written == 0 {
            return None;
        }

        CStr::from_ptr(buffer.as_ptr())
            .to_str()
            .ok()
            .map(ToOwned::to_owned)
    }
}

fn primary_debug_log_path() -> Option<PathBuf> {
    let date = current_date_stamp()?;
    Some(Path::new(DEBUG_LOG_DIR).join(format!("markholo-{date}.log")))
}

fn current_timestamp() -> Option<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let seconds = now.as_secs() as i64;
    let millis = now.subsec_millis();
    let mut tm = Tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };

    // SAFETY: localtime_r and strftime are called with valid pointers and fixed-size buffers.
    unsafe {
        if localtime_r(&seconds, &mut tm).is_null() {
            return None;
        }

        let format = b"%Y-%m-%dT%H:%M:%S\0";
        let mut buffer = [0 as c_char; 32];
        let written = strftime(buffer.as_mut_ptr(), buffer.len(), format.as_ptr().cast(), &tm);
        if written == 0 {
            return None;
        }

        let base = CStr::from_ptr(buffer.as_ptr()).to_str().ok()?;
        Some(format!("{base}.{millis:03}"))
    }
}

fn append_log_line(path: &Path, line: &str) -> bool {
    if let Some(parent) = path.parent() {
        if create_dir_all(parent).is_err() {
            return false;
        }
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        return file.write_all(line.as_bytes()).is_ok();
    }

    false
}

fn debug_log(message: impl AsRef<str>) {
    let ts = current_timestamp().unwrap_or_else(|| "unknown-ts".to_string());
    let pid = std::process::id();
    let current_thread = thread::current();
    let tid = current_thread.name().unwrap_or("unnamed");
    let line = format!("ts={ts} pid={pid} tid={tid} {}\n", message.as_ref());
    eprint!("{line}");

    let wrote_primary = primary_debug_log_path()
        .as_deref()
        .map(|path| append_log_line(path, &line))
        .unwrap_or(false);

    if !wrote_primary {
        let fallback_notice = format!(
            "ts={ts} pid={pid} tid={tid} stage=logger event_id=system msg=\"primary log path unavailable\" primary_path={} fallback_path={}\n",
            primary_debug_log_path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            DEBUG_LOG_FALLBACK_PATH
        );
        let _ = append_log_line(Path::new(DEBUG_LOG_FALLBACK_PATH), &fallback_notice);
        let _ = append_log_line(Path::new(DEBUG_LOG_FALLBACK_PATH), &line);
    }
}

fn log_event(stage: &str, event_id: Option<u64>, message: &str, extra: impl AsRef<str>) {
    let event_id = event_id
        .map(|id| format!("open-{id}"))
        .unwrap_or_else(|| "system".to_string());
    let extra = extra.as_ref();
    if extra.is_empty() {
        debug_log(format!("stage={stage} event_id={event_id} msg=\"{message}\""));
    } else {
        debug_log(format!("stage={stage} event_id={event_id} msg=\"{message}\" {extra}"));
    }
}

fn next_event_id() -> u64 {
    NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed)
}

fn new_action_context(source: &'static str) -> ActionContext {
    ActionContext {
        event_id: next_event_id(),
        source,
    }
}

fn install_panic_hook() {
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

fn dispatch_user_event(proxy: &EventLoopProxy<UserEvent>, stage_source: &'static str, event: UserEvent) {
    let (event_id, event_name, details) = match &event {
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
        UserEvent::ShellReady => (None, "ShellReady", format!("source={stage_source}")),
        UserEvent::OpenExternal(href) => (None, "OpenExternal", format!("source={} href={href}", stage_source)),
        UserEvent::SaveDocument => (None, "SaveDocument", format!("source={stage_source}")),
        UserEvent::ToggleMode => (None, "ToggleMode", format!("source={stage_source}")),
        UserEvent::EditorChanged(markdown) => (
            None,
            "EditorChanged",
            format!("source={} bytes={}", stage_source, markdown.len()),
        ),
        UserEvent::ShowAbout => (None, "ShowAbout", format!("source={stage_source}")),
        UserEvent::Exit => (None, "Exit", format!("source={stage_source}")),
    };

    log_event("send_event.begin", event_id, event_name, &details);
    match proxy.send_event(event) {
        Ok(()) => log_event("send_event.end", event_id, event_name, format!("{details} result=ok")),
        Err(error) => log_event(
            "send_event.end",
            event_id,
            event_name,
            format!("{details} result=err error={error}"),
        ),
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();
    log_event(
        "app.start",
        None,
        "app run started",
        format!("version={APP_VERSION} platform={APP_BUILD_PLATFORM}/{APP_BUILD_TARGET}"),
    );
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let mut modifiers = ModifiersState::default();
    let mut active_document: Option<ActiveDocument> = None;
    let mut shell_ready = false;
    let mut pending_open_requests: Vec<OpenPathRequest> = Vec::new();

    let window = WindowBuilder::new()
        .with_title(WINDOW_TITLE)
        .with_inner_size(LogicalSize::new(1120.0, 760.0))
        .with_min_inner_size(LogicalSize::new(800.0, 560.0))
        .build(&event_loop)?;

    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new()
        .with_html(app_shell_html())
        .with_ipc_handler(move |request| {
            handle_ipc_message(&ipc_proxy, request.body().to_owned());
        })
        .build(&window)?;

    #[cfg(target_os = "macos")]
    macos_menu::install(&proxy)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                log_event("event_loop.init", None, "event loop init", "");
                render_status(&webview, "Ready. Open a Markdown file or press Command+O.", "info");
            }
            Event::Opened { urls } => {
                log_event("tao.opened.begin", None, "received Event::Opened", format!("urls={urls:?}"));
                if let Some(url) = urls.into_iter().find(|url| url.scheme() == "file") {
                    match url.to_file_path() {
                        Ok(path) => {
                            let ctx = new_action_context("tao-opened");
                            log_event(
                                "tao.opened.path",
                                Some(ctx.event_id),
                                "resolved file path from Event::Opened",
                                format!("path={}", path.display()),
                            );
                            dispatch_user_event(
                                &proxy,
                                "tao-opened",
                                UserEvent::OpenPath(OpenPathRequest { ctx, path }),
                            );
                        }
                        Err(_) => {
                            log_event(
                                "tao.opened.error",
                                None,
                                "failed to convert Event::Opened URL to file path",
                                "",
                            );
                            render_status(&webview, "The requested file path is not valid.", "error");
                        }
                    }
                }
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    log_event("window.close_requested", None, "window close requested", "");
                    if resolve_pending_changes(&window, &webview, &mut active_document) {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                WindowEvent::ModifiersChanged(next_modifiers) => {
                    modifiers = next_modifiers;
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == ElementState::Released && modifiers.super_key() {
                        match event.physical_key {
                            KeyCode::KeyO => {
                                let ctx = new_action_context("keyboard-command-o");
                                log_event(
                                    "keyboard.shortcut",
                                    Some(ctx.event_id),
                                    "keyboard shortcut triggered",
                                    "key=Command+O",
                                );
                                dispatch_user_event(&proxy, "keyboard", UserEvent::OpenFile(ctx));
                            }
                            KeyCode::KeyS => {
                                log_event("keyboard.shortcut", None, "keyboard shortcut triggered", "key=Command+S");
                                dispatch_user_event(&proxy, "keyboard", UserEvent::SaveDocument);
                            }
                            KeyCode::KeyW => {
                                log_event("keyboard.shortcut", None, "keyboard shortcut triggered", "key=Command+W");
                                dispatch_user_event(&proxy, "keyboard", UserEvent::Exit);
                            }
                            KeyCode::Slash => {
                                log_event("keyboard.shortcut", None, "keyboard shortcut triggered", "key=Command+/");
                                dispatch_user_event(&proxy, "keyboard", UserEvent::ToggleMode);
                            }
                            _ => {}
                        }
                    }
                }
                WindowEvent::HoveredFile(path) => {
                    let message = format!("Drop to open: {}", path.display());
                    render_status(&webview, &message, "info");
                }
                WindowEvent::HoveredFileCancelled => {
                    render_status(&webview, "Ready. Open a Markdown file or press Command+O.", "info");
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
                        &proxy,
                        "window-drop",
                        UserEvent::OpenPath(OpenPathRequest { ctx, path }),
                    );
                }
                _ => {}
            },
            Event::UserEvent(UserEvent::OpenFile(ctx)) => {
                log_event(
                    "user_event.received",
                    Some(ctx.event_id),
                    "handling UserEvent::OpenFile",
                    format!("source={}", ctx.source),
                );
                if !resolve_pending_changes(&window, &webview, &mut active_document) {
                    log_event(
                        "user_event.aborted",
                        Some(ctx.event_id),
                        "UserEvent::OpenFile aborted by pending changes dialog",
                        "",
                    );
                    return;
                }

                match open_document_dialog(ctx.event_id) {
                    Some(path) => {
                        log_event(
                            "file_dialog.selected",
                            Some(ctx.event_id),
                            "open dialog returned path",
                            format!("path={}", path.display()),
                        );
                        open_document(&window, &webview, &mut active_document, &path, Some(ctx.event_id))
                    }
                    None => {
                        log_event(
                            "file_dialog.cancelled",
                            Some(ctx.event_id),
                            "open dialog cancelled or returned no file",
                            "",
                        );
                        render_status(&webview, "Open cancelled.", "info")
                    }
                }
            }
            Event::UserEvent(UserEvent::OpenPath(request)) => {
                let OpenPathRequest { ctx, path } = request;
                log_event(
                    "user_event.received",
                    Some(ctx.event_id),
                    "handling UserEvent::OpenPath",
                    format!("source={} path={}", ctx.source, path.display()),
                );
                if !shell_ready {
                    log_event(
                        "user_event.deferred",
                        Some(ctx.event_id),
                        "deferring OpenPath until shell is ready",
                        format!("path={}", path.display()),
                    );
                    pending_open_requests.push(OpenPathRequest { ctx, path });
                    return;
                }
                if !resolve_pending_changes(&window, &webview, &mut active_document) {
                    log_event(
                        "user_event.aborted",
                        Some(ctx.event_id),
                        "UserEvent::OpenPath aborted by pending changes dialog",
                        format!("path={}", path.display()),
                    );
                    return;
                }

                open_document(&window, &webview, &mut active_document, &path, Some(ctx.event_id));
            }
            Event::UserEvent(UserEvent::ShellReady) => {
                shell_ready = true;
                log_event(
                    "shell.ready",
                    None,
                    "webview shell reported ready",
                    format!("pending_open_requests={}", pending_open_requests.len()),
                );
                for request in pending_open_requests.drain(..) {
                    dispatch_user_event(&proxy, "shell-ready-flush", UserEvent::OpenPath(request));
                }
            }
            Event::UserEvent(UserEvent::OpenExternal(href)) => {
                log_event("user_event.received", None, "handling UserEvent::OpenExternal", format!("href={href}"));
                if let Err(error) = open::that(href) {
                    log_event("open_external.error", None, "open external failed", format!("error={error}"));
                    render_status(&webview, &format!("Failed to open link: {error}"), "error");
                }
            }
            Event::UserEvent(UserEvent::SaveDocument) => {
                log_event("user_event.received", None, "handling UserEvent::SaveDocument", "");
                save_active_document(&window, &webview, &mut active_document);
            }
            Event::UserEvent(UserEvent::ToggleMode) => {
                log_event("user_event.received", None, "handling UserEvent::ToggleMode", "");
                if let Some(document) = active_document.as_mut() {
                    document.toggle_mode();
                    let status = match document.mode() {
                        DocumentMode::Readonly => "Readonly preview updated.",
                        DocumentMode::Writable => "Writable mode enabled.",
                    };
                    present_document(&window, &webview, document, status, true);
                } else {
                    render_status(&webview, "No document opened.", "error");
                }
            }
            Event::UserEvent(UserEvent::EditorChanged(markdown)) => {
                log_event(
                    "user_event.received",
                    None,
                    "handling UserEvent::EditorChanged",
                    format!("bytes={}", markdown.len()),
                );
                if let Some(document) = active_document.as_mut() {
                    document.update_markdown(markdown);
                    sync_document_state(&window, &webview, document, "Unsaved changes.");
                }
            }
            Event::UserEvent(UserEvent::ShowAbout) => {
                log_event("user_event.received", None, "handling UserEvent::ShowAbout", "");
                render_about(&webview);
            }
            Event::UserEvent(UserEvent::Exit) => {
                log_event("user_event.received", None, "handling UserEvent::Exit", "");
                if resolve_pending_changes(&window, &webview, &mut active_document) {
                    *control_flow = ControlFlow::Exit;
                } else {
                    return;
                }
            }
            _ => {}
        }
    });

    #[allow(unreachable_code)]
    Ok(())
}

fn handle_ipc_message(proxy: &EventLoopProxy<UserEvent>, payload: String) {
    log_event("ipc.received", None, "ipc payload received", format!("payload={payload}"));
    let Ok(value) = serde_json::from_str::<Value>(&payload) else {
        log_event("ipc.error", None, "ipc payload parsing failed", "");
        return;
    };

    match value.get("kind").and_then(Value::as_str) {
        Some("open-file") => {
            let ctx = new_action_context("ipc-open-file");
            dispatch_user_event(proxy, "ipc", UserEvent::OpenFile(ctx));
        }
        Some("shell-ready") => {
            dispatch_user_event(proxy, "ipc", UserEvent::ShellReady);
        }
        Some("open-external") => {
            if let Some(href) = value.get("href").and_then(Value::as_str) {
                dispatch_user_event(proxy, "ipc", UserEvent::OpenExternal(href.to_string()));
            }
        }
        Some("toggle-mode") => {
            dispatch_user_event(proxy, "ipc", UserEvent::ToggleMode);
        }
        Some("request-save") => {
            dispatch_user_event(proxy, "ipc", UserEvent::SaveDocument);
        }
        Some("request-exit") => {
            dispatch_user_event(proxy, "ipc", UserEvent::Exit);
        }
        Some("editor-changed") => {
            if let Some(markdown) = value.get("markdown").and_then(Value::as_str) {
                dispatch_user_event(proxy, "ipc", UserEvent::EditorChanged(markdown.to_string()));
            }
        }
        _ => {}
    }
}

fn open_document_dialog(event_id: u64) -> Option<PathBuf> {
    let started_at = SystemTime::now();
    log_event("file_dialog.begin", Some(event_id), "opening file dialog", "");
    let result = FileDialog::new()
        .add_filter("Markdown", &["md", "markdown"])
        .set_title("Open Markdown File")
        .pick_file();
    let elapsed_ms = started_at.elapsed().map(|duration| duration.as_millis()).unwrap_or(0);
    log_event(
        "file_dialog.end",
        Some(event_id),
        "file dialog finished",
        format!("selected={} elapsed_ms={elapsed_ms}", result.is_some()),
    );
    result
}

fn open_document(
    window: &Window,
    webview: &WebView,
    active_document: &mut Option<ActiveDocument>,
    path: &PathBuf,
    event_id: Option<u64>,
) {
    log_event(
        "open_document.begin",
        event_id,
        "open_document start",
        format!("path={}", path.display()),
    );
    render_status(webview, "Loading document...", "info");

    match load_document(path) {
        Ok(document) => {
            log_event(
                "open_document.end",
                event_id,
                "open_document success",
                format!("path={}", path.display()),
            );
            *active_document = Some(document);
            if let Some(document) = active_document.as_ref() {
                present_document(window, webview, document, "Document loaded.", true);
            }
        }
        Err(message) => {
            log_event(
                "open_document.end",
                event_id,
                "open_document failed",
                format!("path={} error={message}", path.display()),
            );
            render_status(webview, &message, "error");
        }
    }
}

fn load_document(path: &PathBuf) -> Result<ActiveDocument, String> {
    log_event("load_document.begin", None, "load_document path", format!("path={}", path.display()));
    let markdown = file_io::load_markdown(path)?;
    let base_url = file_io::directory_base_url(path)?;
    Ok(ActiveDocument::open(path.clone(), markdown, base_url))
}

fn save_active_document(window: &Window, webview: &WebView, active_document: &mut Option<ActiveDocument>) -> bool {
    let Some(document) = active_document.as_mut() else {
        render_status(webview, "No document to save.", "error");
        return false;
    };

    if let Err(message) = file_io::save_markdown(document.file_path(), document.markdown()) {
        render_status(webview, &message, "error");
        return false;
    }

    document.mark_saved();
    sync_document_state(window, webview, document, "Saved.");
    true
}

fn resolve_pending_changes(
    window: &Window,
    webview: &WebView,
    active_document: &mut Option<ActiveDocument>,
) -> bool {
    let Some(document) = active_document.as_mut() else {
        return true;
    };

    if !document.is_dirty() {
        return true;
    }

    match ask_pending_changes_action(window, document.file_name()) {
        PendingChangesAction::Save => save_active_document(window, webview, active_document),
        PendingChangesAction::Discard => true,
        PendingChangesAction::Cancel => {
            render_status(webview, "Action cancelled.", "info");
            false
        }
    }
}

fn ask_pending_changes_action(window: &Window, file_name: &str) -> PendingChangesAction {
    let result = MessageDialog::new()
        .set_parent(window)
        .set_level(MessageLevel::Warning)
        .set_title("Unsaved changes")
        .set_description(format!("Save changes to {file_name} before continuing?"))
        .set_buttons(MessageButtons::YesNoCancelCustom(
            "Save".to_string(),
            "Discard".to_string(),
            "Cancel".to_string(),
        ))
        .show();

    match result {
        MessageDialogResult::Custom(choice) if choice == "Save" => PendingChangesAction::Save,
        MessageDialogResult::Custom(choice) if choice == "Discard" => PendingChangesAction::Discard,
        _ => PendingChangesAction::Cancel,
    }
}

fn present_document(window: &Window, webview: &WebView, document: &ActiveDocument, status: &str, full_render: bool) {
    update_window_title(window, Some(document));

    if full_render {
        render_document(webview, document, status);
    } else {
        sync_document_state(window, webview, document, status);
    }
}

fn update_window_title(window: &Window, document: Option<&ActiveDocument>) {
    let title = document
        .map(ActiveDocument::window_title)
        .unwrap_or_else(|| WINDOW_TITLE.to_string());
    window.set_title(&title);
}

fn render_document(webview: &WebView, document: &ActiveDocument, status: &str) {
    let snapshot = document.snapshot();
    let payload = DocumentPresentation {
        document: &snapshot,
        status_message: status,
    };
    let serialized = match serde_json::to_string(&payload) {
        Ok(serialized) => serialized,
        Err(error) => {
            render_status(webview, &format!("Failed to serialize document: {error}"), "error");
            return;
        }
    };

    let script = format!("window.renderDocument({serialized});");
    if let Err(error) = webview.evaluate_script(&script) {
        render_status(webview, &format!("WebView script error: {error}"), "error");
    }
}

fn sync_document_state(window: &Window, webview: &WebView, document: &ActiveDocument, status: &str) {
    update_window_title(window, Some(document));

    let snapshot = document.snapshot();
    let payload = DocumentPresentation {
        document: &snapshot,
        status_message: status,
    };
    let serialized = match serde_json::to_string(&payload) {
        Ok(serialized) => serialized,
        Err(error) => {
            render_status(webview, &format!("Failed to serialize document: {error}"), "error");
            return;
        }
    };

    let script = format!("window.updateDocumentState({serialized});");
    if let Err(error) = webview.evaluate_script(&script) {
        render_status(webview, &format!("WebView script error: {error}"), "error");
    }
}

fn render_status(webview: &WebView, message: &str, level: &str) {
    let payload = StatusPayload { message, level };
    let serialized = match serde_json::to_string(&payload) {
        Ok(serialized) => serialized,
        Err(_) => return,
    };
    let script = format!("window.showStatus({serialized});");
    let _ = webview.evaluate_script(&script);
}

fn render_about(webview: &WebView) {
    let script = format!(
        "window.showAbout({{version:{}, author:{}, githubUrl:{}, buildTarget:{}, buildPlatform:{}}});",
        serde_json::to_string(APP_VERSION).unwrap_or_else(|_| "\"0.6.3\"".to_string()),
        serde_json::to_string(APP_AUTHOR).unwrap_or_else(|_| "\"Ronnie Deng\"".to_string()),
        serde_json::to_string(APP_GITHUB_URL)
            .unwrap_or_else(|_| "\"https://github.com/phpple/markhola\"".to_string()),
        serde_json::to_string(APP_BUILD_TARGET).unwrap_or_else(|_| "\"unknown\"".to_string()),
        serde_json::to_string(APP_BUILD_PLATFORM).unwrap_or_else(|_| "\"unknown\"".to_string())
    );
    let _ = webview.evaluate_script(&script);
}

fn app_shell_html() -> String {
    APP_SHELL_HTML
        .replace("__MERMAID_RUNTIME__", &mermaid_runtime_script())
        .replace("__MATHJAX_RUNTIME__", &mathjax_runtime_script())
}

fn mermaid_runtime_script() -> String {
    MERMAID_RUNTIME.replace("</script", "<\\/script")
}

fn mathjax_runtime_script() -> String {
    MATHJAX_RUNTIME.replace("</script", "<\\/script")
}

const APP_SHELL_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <base id="document-base" href="" />
    <title>MarkHola</title>
    <style>
      :root {
        color-scheme: light;
        --bg: #f5f1e8;
        --panel: rgba(255, 253, 248, 0.88);
        --panel-strong: rgba(255, 251, 243, 0.97);
        --border: rgba(92, 74, 52, 0.16);
        --text: #2b241d;
        --muted: #6f6258;
        --accent: #0f766e;
        --accent-strong: #115e59;
        --warn: #b45309;
        --danger: #b91c1c;
        --shadow: 0 24px 60px rgba(69, 48, 29, 0.12);
        --font-ui: "SF Pro Display", "Helvetica Neue", sans-serif;
        --font-body: "Charter", "Iowan Old Style", Georgia, serif;
        --font-code: "SF Mono", "JetBrains Mono", monospace;
      }

      * {
        box-sizing: border-box;
      }

      html,
      body {
        margin: 0;
        min-height: 100%;
        background:
          radial-gradient(circle at top left, rgba(15, 118, 110, 0.08), transparent 35%),
          linear-gradient(180deg, #f8f5ee 0%, #f2ecdf 100%);
        color: var(--text);
        font-family: var(--font-ui);
      }

      body {
        padding: 18px;
      }

      .app {
        display: grid;
        grid-template-rows: minmax(0, 1fr) auto;
        gap: 14px;
        min-height: calc(100vh - 36px);
      }

      .preview-shell {
        display: grid;
        grid-template-rows: auto 1fr;
        min-height: 0;
        background: var(--panel-strong);
        border: 1px solid var(--border);
        border-radius: 24px;
        overflow: hidden;
        box-shadow: var(--shadow);
      }

      .preview-header {
        display: flex;
        justify-content: space-between;
        gap: 12px;
        padding: 14px 20px;
        border-bottom: 1px solid var(--border);
        background: rgba(255, 253, 248, 0.72);
      }

      .preview-title {
        display: flex;
        flex-direction: column;
        gap: 3px;
        min-width: 0;
      }

      .preview-title strong {
        font-size: 15px;
      }

      .preview-title span {
        color: var(--muted);
        font-size: 12px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .status {
        color: var(--muted);
        font-size: 12px;
        font-weight: 600;
        text-align: right;
      }

      .status[data-level="warning"] {
        color: var(--warn);
      }

      .status[data-level="error"] {
        color: var(--danger);
      }

      .workspace {
        min-height: 0;
      }

      .pane {
        height: 100%;
      }

      .preview {
        overflow: auto;
        padding: 24px;
      }

      .editor-pane {
        min-height: 100%;
        padding: 22px;
      }

      .editor-shell {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        height: 100%;
        border-radius: 18px;
        background: rgba(255, 255, 255, 0.9);
        border: 1px solid rgba(92, 74, 52, 0.16);
        overflow: hidden;
        box-shadow: inset 0 1px 3px rgba(92, 74, 52, 0.06);
      }

      .editor-line-numbers {
        min-width: 56px;
        padding: 18px 10px 18px 14px;
        background: rgba(92, 74, 52, 0.06);
        border-right: 1px solid rgba(92, 74, 52, 0.12);
        color: rgba(111, 98, 88, 0.78);
        font: 15px/1.68 var(--font-code);
        text-align: right;
        user-select: none;
        overflow: hidden;
      }

      .editor-line-number {
        display: block;
      }

      .editor {
        width: 100%;
        border: 0;
        color: var(--text);
        font: 15px/1.68 var(--font-code);
        padding: 18px 20px;
        resize: none;
        outline: none;
        background: transparent;
      }

      .empty-state {
        display: grid;
        place-items: center;
        min-height: 100%;
        padding: 40px 20px;
        text-align: center;
      }

      .empty-card {
        max-width: 440px;
        padding: 32px;
        border-radius: 22px;
        background: rgba(255, 255, 255, 0.72);
        border: 1px solid rgba(92, 74, 52, 0.1);
      }

      .empty-card h2 {
        margin: 0 0 10px;
        font-size: 28px;
      }

      .bottom-bar {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto auto auto auto;
        gap: 16px;
        align-items: center;
        padding: 12px 16px;
        background: var(--panel);
        border: 1px solid var(--border);
        border-radius: 16px;
        box-shadow: var(--shadow);
        backdrop-filter: blur(14px);
        font-size: 12px;
        color: var(--muted);
      }

      .bottom-item {
        min-width: 0;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .bottom-item strong {
        color: var(--text);
        font-weight: 600;
      }

      .markdown-body {
        max-width: 860px;
        margin: 0 auto;
        color: var(--text);
        font-family: var(--font-body);
        line-height: 1.72;
        font-size: 17px;
      }

      .markdown-body h1,
      .markdown-body h2,
      .markdown-body h3,
      .markdown-body h4,
      .markdown-body h5,
      .markdown-body h6 {
        margin-top: 1.7em;
        margin-bottom: 0.55em;
        line-height: 1.2;
        font-family: var(--font-ui);
      }

      .markdown-body h1 {
        font-size: 2.3rem;
      }

      .markdown-body h2 {
        font-size: 1.72rem;
      }

      .markdown-body h3 {
        font-size: 1.35rem;
      }

      .markdown-body p,
      .markdown-body ul,
      .markdown-body ol,
      .markdown-body table,
      .markdown-body blockquote {
        margin: 1em 0;
      }

      .markdown-body a {
        color: var(--accent);
      }

      .markdown-body img {
        display: block;
        max-width: 100%;
        height: auto;
        margin: 1.4em auto;
        border-radius: 16px;
        box-shadow: 0 12px 34px rgba(56, 41, 28, 0.14);
      }

      .markdown-body table {
        width: 100%;
        border-collapse: collapse;
        background: rgba(255, 255, 255, 0.66);
        overflow: hidden;
        border-radius: 14px;
      }

      .markdown-body th,
      .markdown-body td {
        padding: 10px 12px;
        border: 1px solid rgba(92, 74, 52, 0.14);
        text-align: left;
        vertical-align: top;
      }

      .markdown-body thead {
        background: rgba(15, 118, 110, 0.08);
        font-family: var(--font-ui);
      }

      .markdown-body code {
        font-family: var(--font-code);
        font-size: 0.92em;
        background: rgba(43, 36, 29, 0.08);
        padding: 0.15em 0.35em;
        border-radius: 6px;
      }

      .markdown-body pre {
        overflow: auto;
        padding: 16px;
        border-radius: 16px;
        background: #666666;
        color: #f8f5ee;
      }

      .markdown-body pre code {
        display: block;
        background: transparent;
        padding: 0;
        border-radius: 0;
        font-size: inherit;
      }

      .markdown-body .code-block {
        position: relative;
        margin: 1.2em 0;
        border-radius: 18px;
        background: #666666;
        box-shadow: 0 12px 34px rgba(56, 41, 28, 0.16);
        overflow: hidden;
        --code-line-height: 1.65;
        --code-font-size: 14px;
      }

      .markdown-body .code-block__badge {
        position: absolute;
        top: 12px;
        right: 14px;
        z-index: 1;
        padding: 4px 10px;
        border-radius: 999px;
        background: rgba(255, 255, 255, 0.18);
        color: rgba(248, 245, 238, 0.92);
        font: 11px/1.2 var(--font-ui);
        letter-spacing: 0.04em;
        text-transform: uppercase;
        opacity: 0;
        transition: opacity 140ms ease;
        pointer-events: none;
      }

      .markdown-body .code-block:hover .code-block__badge {
        opacity: 1;
      }

      .markdown-body .code-block__body {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        align-items: stretch;
        overflow: auto;
      }

      .markdown-body .code-block__line-numbers {
        display: grid;
        align-content: start;
        padding: 18px 0 18px 14px;
        background: rgba(255, 255, 255, 0.1);
        border-right: 1px solid rgba(255, 255, 255, 0.14);
        color: rgba(248, 245, 238, 0.72);
        font: var(--code-font-size)/var(--code-line-height) var(--font-code);
        user-select: none;
      }

      .markdown-body .code-block__line-number {
        display: block;
        min-width: 2.4em;
        padding-right: 12px;
        text-align: right;
      }

      .markdown-body .code-block__pre {
        margin: 0;
        min-width: 100%;
        overflow: visible;
        padding: 18px 18px 18px 16px;
        border-radius: 0;
        background: transparent;
        font: var(--code-font-size)/var(--code-line-height) var(--font-code);
      }

      .markdown-body .code-block__code {
        white-space: pre;
        font: inherit;
        line-height: inherit;
      }

      .markdown-body .code-block__line {
        display: block;
        line-height: var(--code-line-height);
      }

      .markdown-body .mermaid-block {
        margin: 1.2em 0;
        border-radius: 18px;
        background: rgba(255, 255, 255, 0.9);
        border: 1px solid rgba(92, 74, 52, 0.12);
        box-shadow: 0 12px 34px rgba(56, 41, 28, 0.08);
        overflow: auto;
      }

      .markdown-body .mermaid-block__status {
        padding: 12px 16px 0;
        color: var(--muted);
        font: 12px/1.4 var(--font-ui);
      }

      .markdown-body .mermaid-block__diagram {
        min-width: min-content;
        padding: 10px 16px 16px;
      }

      .markdown-body .mermaid-block__diagram svg {
        display: block;
        max-width: none;
        height: auto;
      }

      .markdown-body .mermaid-block__error {
        margin: 0;
        padding: 12px 16px 16px;
        color: #8b1e1e;
        font: 13px/1.6 var(--font-code);
        white-space: pre-wrap;
      }

      .markdown-body .math.math-inline {
        display: inline-flex;
        align-items: center;
        vertical-align: middle;
        max-width: 100%;
      }

      .markdown-body .math.math-display {
        display: block;
        margin: 1.25em 0;
        overflow-x: auto;
        overflow-y: hidden;
        text-align: center;
      }

      .markdown-body .math svg,
      .markdown-body .math-block__formula svg {
        display: inline-block;
        max-width: 100%;
        height: auto;
      }

      .markdown-body .math-block {
        margin: 1.2em 0;
        border-radius: 18px;
        background: rgba(255, 255, 255, 0.9);
        border: 1px solid rgba(92, 74, 52, 0.12);
        box-shadow: 0 12px 34px rgba(56, 41, 28, 0.08);
        overflow-x: auto;
      }

      .markdown-body .math-block__status {
        padding: 12px 16px 0;
        color: var(--muted);
        font: 12px/1.4 var(--font-ui);
      }

      .markdown-body .math-block__formula {
        min-width: min-content;
        padding: 10px 16px 16px;
        text-align: center;
      }

      .markdown-body .math-block__error {
        margin: 0;
        padding: 12px 16px 16px;
        color: #8b1e1e;
        font: 13px/1.6 var(--font-code);
        white-space: pre-wrap;
        text-align: left;
      }

      .markdown-body blockquote {
        padding: 2px 0 2px 18px;
        border-left: 4px solid rgba(15, 118, 110, 0.3);
        color: #51453b;
      }

      .markdown-body .image-error {
        padding: 12px 14px;
        border-radius: 12px;
        background: rgba(183, 28, 28, 0.08);
        border: 1px solid rgba(183, 28, 28, 0.16);
        color: #8b1e1e;
        font-family: var(--font-ui);
        font-size: 14px;
      }

      .hidden {
        display: none !important;
      }

      .about-overlay {
        position: fixed;
        inset: 0;
        display: grid;
        place-items: center;
        padding: 24px;
        background: rgba(34, 27, 20, 0.28);
        backdrop-filter: blur(12px);
        z-index: 50;
      }

      .about-dialog {
        width: min(560px, 100%);
        padding: 30px;
        border-radius: 30px;
        background:
          linear-gradient(180deg, rgba(255, 252, 246, 0.99) 0%, rgba(252, 247, 239, 0.99) 100%);
        border: 1px solid rgba(92, 74, 52, 0.12);
        box-shadow: 0 24px 80px rgba(50, 35, 22, 0.18);
      }

      .about-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
        margin-bottom: 18px;
      }

      .about-header h2 {
        margin: 0;
        font-size: 20px;
      }

      .about-close {
        border: 0;
        background: transparent;
        color: var(--muted);
        font: inherit;
        font-size: 26px;
        line-height: 1;
        cursor: pointer;
        padding: 0;
      }

      .about-brand {
        display: grid;
        justify-items: center;
        gap: 14px;
        margin-bottom: 22px;
      }

      .about-logo {
        width: min(300px, 100%);
        height: auto;
      }

      .about-product {
        display: grid;
        gap: 4px;
        justify-items: center;
      }

      .about-product h3 {
        margin: 0;
        font-size: 24px;
        letter-spacing: 0.02em;
      }

      .about-product p {
        margin: 0;
        color: var(--muted);
        font-size: 13px;
      }

      .about-meta {
        display: grid;
        gap: 10px;
        color: var(--muted);
        font-size: 14px;
      }

      .about-meta-row {
        display: grid;
        grid-template-columns: 96px minmax(0, 1fr) auto;
        gap: 12px;
        align-items: center;
        padding: 10px 12px;
        border-radius: 14px;
        background: rgba(255, 255, 255, 0.54);
        border: 1px solid rgba(92, 74, 52, 0.08);
      }

      .about-meta strong {
        color: var(--text);
        font-weight: 600;
      }

      .about-value {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .about-copy {
        appearance: none;
        border: 0;
        border-radius: 999px;
        padding: 7px 12px;
        font: inherit;
        font-size: 12px;
        font-weight: 600;
        color: white;
        background: linear-gradient(135deg, var(--accent) 0%, var(--accent-strong) 100%);
        cursor: pointer;
        box-shadow: 0 8px 20px rgba(15, 118, 110, 0.22);
      }

      .about-footer {
        margin-top: 18px;
        color: var(--muted);
        font-size: 12px;
        text-align: center;
      }
    </style>
  </head>
  <body>
    <div class="app">
      <section class="preview-shell">
        <div class="preview-header">
          <div class="preview-title">
            <strong id="documentTitle">Preview</strong>
            <span id="documentSubtitle">Use File > Open, Command+O, or drag a Markdown file into the window.</span>
          </div>
          <span id="status" class="status" data-level="info">Ready.</span>
        </div>
        <div class="workspace">
          <div class="empty-state pane" id="emptyState">
            <div class="empty-card">
              <h2>No document opened</h2>
              <p>Open, drag, or drop a Markdown file to preview or edit the current Markdown source.</p>
            </div>
          </div>
          <div class="preview pane hidden" id="previewPane">
            <article class="markdown-body" id="content"></article>
          </div>
          <div class="editor-pane pane hidden" id="editorPane">
            <div class="editor-shell" id="editorShell">
              <div id="editorLineNumbers" class="editor-line-numbers" aria-hidden="true">1</div>
              <textarea id="editor" class="editor" spellcheck="false" aria-label="Markdown editor"></textarea>
            </div>
          </div>
        </div>
      </section>

      <footer class="bottom-bar">
        <div class="bottom-item" id="filePath">Path: No file opened</div>
        <div class="bottom-item" id="wordCount"><strong>Words</strong> 0</div>
        <div class="bottom-item" id="lineCount"><strong>Lines</strong> 0</div>
        <div class="bottom-item" id="modeState"><strong>Mode</strong> Readonly</div>
        <div class="bottom-item" id="saveState"><strong>Status</strong> Ready.</div>
      </footer>
    </div>

    <div class="about-overlay hidden" id="aboutOverlay">
      <div class="about-dialog" role="dialog" aria-modal="true" aria-labelledby="aboutTitle">
        <div class="about-header">
          <h2 id="aboutTitle">About MarkHola</h2>
          <button class="about-close" id="aboutClose" type="button" aria-label="Close About">&times;</button>
        </div>
        <div class="about-brand">
          <svg class="about-logo" viewBox="0 0 1600 640" aria-hidden="true">
            <rect width="1600" height="640" rx="72" fill="#FCFBF8"/>
            <g transform="translate(72 78)">
              <rect x="0" y="0" width="484" height="484" rx="120" fill="url(#aboutBadge)"/>
              <rect x="46" y="46" width="392" height="392" rx="98" fill="#FFF9F0" fill-opacity="0.96"/>
              <path d="M120 338V140H160L242 272L324 140H364V338H321V218L255 320H229L163 218V338H120Z" fill="#FF8A00"/>
              <path d="M198 262H286V306H198V262Z" fill="#35D67C"/>
              <circle cx="242" cy="284" r="74" stroke="#35D67C" stroke-width="18" stroke-dasharray="14 18"/>
            </g>
            <g transform="translate(632 162)">
              <path d="M0 244V20H40L134 179L228 20H268V244H222V110L146 210H122L46 110V244H0Z" fill="#FF8A00"/>
              <path d="M309 244V83C309 45 338 22 380 22H487V64H388C367 64 356 72 356 87V106H473C514 106 543 131 543 169V181C543 219 514 244 473 244H309ZM356 202H465C485 202 497 193 497 178V172C497 157 485 148 465 148H356V202Z" fill="#FF8A00"/>
              <path d="M582 244V83C582 45 611 22 653 22H770V64H661C640 64 629 72 629 87V244H582Z" fill="#FF8A00"/>
              <path d="M807 244V0H854V244H807Z" fill="#FF8A00"/>
              <path d="M899 132L1010 22H1071L957 132L1082 244H1017L899 145V244H852V22H899V132Z" fill="#FF8A00"/>
              <path d="M585 496V276H632V365H754V276H801V496H754V407H632V496H585Z" fill="#35D67C"/>
              <path d="M835 408V365C835 314 872 276 925 276H1034C1087 276 1124 314 1124 365V408C1124 458 1087 496 1034 496H925C872 496 835 458 835 408ZM882 401C882 429 899 454 928 454H1031C1060 454 1077 429 1077 401V372C1077 343 1060 318 1031 318H928C899 318 882 343 882 372V401Z" fill="#35D67C"/>
              <path d="M1156 496V315C1156 292 1173 276 1197 276H1239V318H1206C1200 318 1196 322 1196 329V496H1156Z" fill="#35D67C"/>
              <path d="M1270 408V365C1270 314 1308 276 1361 276H1470C1523 276 1560 314 1560 365V408C1560 458 1523 496 1470 496H1361C1308 496 1270 458 1270 408ZM1317 401C1317 429 1334 454 1364 454H1467C1496 454 1513 429 1513 401V372C1513 343 1496 318 1467 318H1364C1334 318 1317 343 1317 372V401Z" fill="#35D67C"/>
            </g>
            <defs>
              <linearGradient id="aboutBadge" x1="0" y1="0" x2="484" y2="484" gradientUnits="userSpaceOnUse">
                <stop stop-color="#FFF2D9"/>
                <stop offset="1" stop-color="#EFFFF5"/>
              </linearGradient>
            </defs>
          </svg>
          <div class="about-product">
            <h3>MarkHola</h3>
            <p>Markdown reader and editor</p>
          </div>
        </div>
        <div class="about-meta">
          <div class="about-meta-row">
            <strong>Version</strong>
            <span class="about-value" id="aboutVersion">0.6.3</span>
            <span></span>
          </div>
          <div class="about-meta-row">
            <strong>Author</strong>
            <span class="about-value" id="aboutAuthor">Ronnie Deng</span>
            <span></span>
          </div>
          <div class="about-meta-row">
            <strong>Build</strong>
            <span class="about-value" id="aboutBuild">apple / arm64</span>
            <span></span>
          </div>
          <div class="about-meta-row">
            <strong>GitHub</strong>
            <a class="about-value" id="aboutGithub" href="https://github.com/phpple/markhola">https://github.com/phpple/markhola</a>
            <button class="about-copy" id="aboutCopy" type="button">Copy</button>
          </div>
        </div>
        <div class="about-footer">Built for local Markdown reading and writing on Apple Silicon.</div>
      </div>
    </div>

    <script>
      window.MathJax = {
        startup: { typeset: false },
        svg: { fontCache: "none" }
      };
    </script>
    <script>__MERMAID_RUNTIME__</script>
    <script>__MATHJAX_RUNTIME__</script>
    <script>
      const status = document.getElementById("status");
      const documentTitle = document.getElementById("documentTitle");
      const documentSubtitle = document.getElementById("documentSubtitle");
      const emptyState = document.getElementById("emptyState");
      const previewPane = document.getElementById("previewPane");
      const editorPane = document.getElementById("editorPane");
      const editorLineNumbers = document.getElementById("editorLineNumbers");
      const editor = document.getElementById("editor");
      const content = document.getElementById("content");
      const documentBase = document.getElementById("document-base");
      const filePath = document.getElementById("filePath");
      const wordCount = document.getElementById("wordCount");
      const lineCount = document.getElementById("lineCount");
      const modeState = document.getElementById("modeState");
      const saveState = document.getElementById("saveState");
      const aboutOverlay = document.getElementById("aboutOverlay");
      const aboutClose = document.getElementById("aboutClose");
      const aboutVersion = document.getElementById("aboutVersion");
      const aboutAuthor = document.getElementById("aboutAuthor");
      const aboutBuild = document.getElementById("aboutBuild");
      const aboutGithub = document.getElementById("aboutGithub");
      const aboutCopy = document.getElementById("aboutCopy");
      let mermaidInitialized = false;
      let mathJaxReadyPromise = null;

      const hideAbout = () => {
        aboutOverlay.classList.add("hidden");
      };

      const EDITOR_INDENT = "    ";

      const insertIndent = () => {
        const start = editor.selectionStart;
        const end = editor.selectionEnd;
        editor.setRangeText(EDITOR_INDENT, start, end, "end");
        editor.dispatchEvent(new Event("input", { bubbles: true }));
      };

      const isWritableMode = () => !editorPane.classList.contains("hidden");

      const selectAllEditorText = () => {
        editor.focus();
        editor.selectionStart = 0;
        editor.selectionEnd = editor.value.length;
        editor.setSelectionRange(0, editor.value.length);
      };

      const updateEditorLineNumbers = () => {
        const totalLines = Math.max(1, editor.value.split("\n").length);
        editorLineNumbers.innerHTML = Array.from(
          { length: totalLines },
          (_, index) => `<span class="editor-line-number">${index + 1}</span>`
        ).join("");
      };

      const syncEditorScroll = () => {
        editorLineNumbers.scrollTop = editor.scrollTop;
      };

      const moveCaretToLineBoundary = (boundary) => {
        const cursor = editor.selectionStart;
        const value = editor.value;
        const lineStart = value.lastIndexOf("\n", Math.max(0, cursor - 1)) + 1;
        const nextBreak = value.indexOf("\n", cursor);
        const lineEnd = nextBreak === -1 ? value.length : nextBreak;
        const target = boundary === "start" ? lineStart : lineEnd;
        editor.focus();
        editor.setSelectionRange(target, target);
      };

      const lineRangeForSelection = () => {
        const value = editor.value;
        const start = editor.selectionStart;
        const end = editor.selectionEnd;
        const effectiveEnd = end > start && value[end - 1] === "\n" ? end - 1 : end;
        const blockStart = value.lastIndexOf("\n", Math.max(0, start - 1)) + 1;
        let blockEnd = effectiveEnd;

        while (blockEnd < value.length && value[blockEnd] !== "\n") {
          blockEnd += 1;
        }

        return { start, end, blockStart, blockEnd };
      };

      const indentSelectedLines = () => {
        const { start, end, blockStart, blockEnd } = lineRangeForSelection();
        const value = editor.value;
        const block = value.slice(blockStart, blockEnd);

        if (start === end && !block.includes("\n")) {
          insertIndent();
          return;
        }

        const lines = block.split("\n");
        const indented = lines.map((line) => `${EDITOR_INDENT}${line}`).join("\n");
        editor.setRangeText(indented, blockStart, blockEnd, "preserve");

        const nextStart = start + EDITOR_INDENT.length;
        const nextEnd = end + EDITOR_INDENT.length * lines.length;
        editor.setSelectionRange(nextStart, nextEnd);
        editor.dispatchEvent(new Event("input", { bubbles: true }));
      };

      const outdentSelectedLines = () => {
        const { start, end, blockStart, blockEnd } = lineRangeForSelection();
        const value = editor.value;
        const block = value.slice(blockStart, blockEnd);
        const lines = block.split("\n");
        const removedPerLine = lines.map((line) => {
          const match = line.match(/^ {1,4}/);
          return match ? match[0].length : 0;
        });

        if (removedPerLine.every((count) => count === 0)) {
          return;
        }

        const outdented = lines
          .map((line, index) => line.slice(removedPerLine[index]))
          .join("\n");

        editor.setRangeText(outdented, blockStart, blockEnd, "preserve");

        const firstLineRemoved = removedPerLine[0];
        const removedBeforeSelectionEnd = removedPerLine.reduce(
          (total, count) => total + count,
          0
        );
        const nextStart = Math.max(blockStart, start - firstLineRemoved);
        const nextEnd = Math.max(nextStart, end - removedBeforeSelectionEnd);
        editor.setSelectionRange(nextStart, nextEnd);
        editor.dispatchEvent(new Event("input", { bubbles: true }));
      };

      const runEditorCommand = (command) => {
        editor.focus();
        document.execCommand(command);
      };

      const showPaneForMode = (mode) => {
        const hasDocument = mode === "readonly" || mode === "writable";
        emptyState.classList.toggle("hidden", hasDocument);
        previewPane.classList.toggle("hidden", mode !== "readonly");
        editorPane.classList.toggle("hidden", mode !== "writable");
      };

      const applyDocumentChrome = (payload) => {
        document.title = `${payload.file_name}${payload.dirty ? " *" : ""} - MarkHola`;
        documentTitle.textContent = payload.title;
        documentSubtitle.textContent = payload.file_name;
        filePath.textContent = `Path: ${payload.file_path}`;
        wordCount.innerHTML = `<strong>Words</strong> ${payload.word_count}`;
        lineCount.innerHTML = `<strong>Lines</strong> ${payload.line_count}`;
        modeState.innerHTML = `<strong>Mode</strong> ${payload.mode_label}`;
        saveState.innerHTML = `<strong>Status</strong> ${payload.save_status}`;
        documentBase.setAttribute("href", payload.base_url);
        showPaneForMode(payload.mode);
        window.showStatus({ message: payload.status_message, level: payload.dirty ? "warning" : "info" });
      };

      const escapeHtml = (value) =>
        value
          .replaceAll("&", "&amp;")
          .replaceAll("<", "&lt;")
          .replaceAll(">", "&gt;");

      const ensureMermaidInitialized = () => {
        if (mermaidInitialized || !window.mermaid) return;

        window.mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: "default"
        });
        mermaidInitialized = true;
      };

      const renderMermaidDiagrams = async () => {
        ensureMermaidInitialized();
        if (!window.mermaid) return;

        const blocks = document.querySelectorAll(".mermaid-block");
        for (const [index, block] of blocks.entries()) {
          const statusNode = block.querySelector(".mermaid-block__status");
          const sourceNode = block.querySelector(".mermaid-block__source");
          const diagramNode = block.querySelector(".mermaid-block__diagram");
          const source = sourceNode?.textContent || "";

          if (!diagramNode) continue;

          diagramNode.innerHTML = "";
          if (statusNode) {
            statusNode.textContent = "Rendering diagram...";
            statusNode.classList.remove("hidden");
          }

          try {
            const { svg } = await window.mermaid.render(
              `mermaid-diagram-${index}-${Date.now()}`,
              source
            );
            diagramNode.innerHTML = svg;
            statusNode?.classList.add("hidden");
          } catch (error) {
            const message =
              error && typeof error === "object" && "message" in error
                ? String(error.message)
                : String(error || "Unknown Mermaid error");
            if (statusNode) {
              statusNode.textContent = "Mermaid render failed.";
              statusNode.classList.remove("hidden");
            }
            diagramNode.innerHTML =
              `<pre class="mermaid-block__error">${escapeHtml(message)}\n\n${escapeHtml(source)}</pre>`;
          }
        }
      };

      const ensureMathJaxReady = () => {
        if (!window.MathJax || !window.MathJax.startup) return null;
        if (!mathJaxReadyPromise) {
          mathJaxReadyPromise = window.MathJax.startup.promise;
        }
        return mathJaxReadyPromise;
      };

      const extractRenderedMathNode = (rendered) =>
        rendered.querySelector("mjx-container") || rendered.firstElementChild || rendered;

      const renderMathSource = async (node, source, display) => {
        const ready = ensureMathJaxReady();
        if (!ready) return false;

        await ready;
        const rendered = await window.MathJax.tex2svgPromise(source, { display });
        const mathNode = extractRenderedMathNode(rendered);
        node.replaceChildren(mathNode.cloneNode(true));
        return true;
      };

      const renderMathExpressions = async () => {
        if (!window.MathJax) return;

        const mathNodes = content.querySelectorAll(".math.math-inline, .math.math-display");
        for (const node of mathNodes) {
          const source = node.textContent || "";
          const display = node.classList.contains("math-display");

          try {
            await renderMathSource(node, source, display);
          } catch (error) {
            const message =
              error && typeof error === "object" && "message" in error
                ? String(error.message)
                : String(error || "Unknown math error");
            node.innerHTML = `<code>${escapeHtml(`Math render failed: ${message}\n\n${source}`)}</code>`;
          }
        }

        const blocks = content.querySelectorAll(".math-block");
        for (const block of blocks) {
          const statusNode = block.querySelector(".math-block__status");
          const sourceNode = block.querySelector(".math-block__source");
          const formulaNode = block.querySelector(".math-block__formula");
          const source = sourceNode?.textContent || "";

          if (!formulaNode) continue;

          formulaNode.innerHTML = "";
          if (statusNode) {
            statusNode.textContent = "Rendering formula...";
            statusNode.classList.remove("hidden");
          }

          try {
            await renderMathSource(formulaNode, source, true);
            statusNode?.classList.add("hidden");
          } catch (error) {
            const message =
              error && typeof error === "object" && "message" in error
                ? String(error.message)
                : String(error || "Unknown math error");
            if (statusNode) {
              statusNode.textContent = "Math render failed.";
              statusNode.classList.remove("hidden");
            }
            formulaNode.innerHTML =
              `<pre class="math-block__error">${escapeHtml(message)}\n\n${escapeHtml(source)}</pre>`;
          }
        }
      };

      const renderReadonlyEnhancements = async () => {
        await renderMermaidDiagrams();
        await renderMathExpressions();
      };

      aboutClose.addEventListener("click", hideAbout);
      aboutOverlay.addEventListener("click", (event) => {
        if (event.target === aboutOverlay) hideAbout();
      });

      aboutCopy.addEventListener("click", async () => {
        const url = aboutGithub.getAttribute("href") || "";
        if (!url) return;

        try {
          await navigator.clipboard.writeText(url);
          aboutCopy.textContent = "Copied";
          setTimeout(() => {
            aboutCopy.textContent = "Copy";
          }, 1200);
        } catch {
          aboutCopy.textContent = "Failed";
          setTimeout(() => {
            aboutCopy.textContent = "Copy";
          }, 1200);
        }
      });

      editor.addEventListener("input", () => {
        updateEditorLineNumbers();
        window.ipc.postMessage(JSON.stringify({ kind: "editor-changed", markdown: editor.value }));
      });

      editor.addEventListener("scroll", syncEditorScroll);

      document.addEventListener("keydown", (event) => {
        if (event.key === "Escape" && !aboutOverlay.classList.contains("hidden")) {
          hideAbout();
          return;
        }

        if (event.target === editor && event.key === "Tab" && !event.metaKey && !event.ctrlKey) {
          event.preventDefault();
          if (event.shiftKey) {
            outdentSelectedLines();
          } else {
            indentSelectedLines();
          }
          return;
        }

        if (event.target === editor && event.ctrlKey && !event.metaKey && !event.altKey) {
          if (event.key.toLowerCase() === "a") {
            event.preventDefault();
            moveCaretToLineBoundary("start");
            return;
          }

          if (event.key.toLowerCase() === "e") {
            event.preventDefault();
            moveCaretToLineBoundary("end");
            return;
          }
        }

        if (!event.metaKey || event.ctrlKey || event.altKey) {
          return;
        }

        if (event.key.toLowerCase() === "z" && isWritableMode()) {
          if (document.activeElement !== editor) {
            event.preventDefault();
            runEditorCommand("undo");
          }
        } else if (event.key.toLowerCase() === "r" && isWritableMode()) {
          event.preventDefault();
          runEditorCommand("redo");
        } else if (event.key.toLowerCase() === "s") {
          event.preventDefault();
          window.ipc.postMessage(JSON.stringify({ kind: "request-save" }));
        } else if (event.key.toLowerCase() === "w") {
          event.preventDefault();
          window.ipc.postMessage(JSON.stringify({ kind: "request-exit" }));
        } else if (event.key.toLowerCase() === "a" && isWritableMode()) {
          event.preventDefault();
          selectAllEditorText();
        } else if (event.key === "/") {
          event.preventDefault();
          window.ipc.postMessage(JSON.stringify({ kind: "toggle-mode" }));
        }
      });

      document.addEventListener("click", (event) => {
        const link = event.target.closest("a[href]");
        if (!link) return;

        const href = link.getAttribute("href") || "";
        if (href.startsWith("http://") || href.startsWith("https://")) {
          event.preventDefault();
          window.ipc.postMessage(JSON.stringify({ kind: "open-external", href }));
        }
      });

      document.addEventListener(
        "error",
        (event) => {
          const target = event.target;
          if (!(target instanceof HTMLImageElement)) return;

          const fallback = document.createElement("p");
          fallback.className = "image-error";
          fallback.textContent = `Image failed to load: ${target.getAttribute("src") || "unknown source"}`;
          target.replaceWith(fallback);
        },
        true
      );

      window.showStatus = (payload) => {
        status.textContent = payload.message;
        status.dataset.level = payload.level || "info";
      };

      window.renderDocument = (payload) => {
        applyDocumentChrome(payload);
        content.innerHTML = payload.html;
        editor.value = payload.markdown;
        updateEditorLineNumbers();
        syncEditorScroll();
        void renderReadonlyEnhancements();
      };

      window.updateDocumentState = (payload) => {
        applyDocumentChrome(payload);
        if (payload.mode === "readonly") {
          content.innerHTML = payload.html;
          void renderReadonlyEnhancements();
        }
      };

      updateEditorLineNumbers();

      window.showAbout = (payload) => {
        aboutVersion.textContent = payload.version;
        aboutAuthor.textContent = payload.author;
        aboutBuild.textContent = `${payload.buildPlatform} / ${payload.buildTarget}`;
        aboutGithub.textContent = payload.githubUrl;
        aboutGithub.setAttribute("href", payload.githubUrl);
        aboutCopy.textContent = "Copy";
        aboutOverlay.classList.remove("hidden");
      };

      window.ipc.postMessage(JSON.stringify({ kind: "shell-ready" }));
    </script>
  </body>
</html>
"##;

#[cfg(target_os = "macos")]
mod macos_menu {
    use std::error::Error;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{DefinedClass, MainThreadOnly, define_class, sel};
    use objc2_app_kit::{NSApp, NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
    use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, ns_string};
    use tao::event_loop::EventLoopProxy;

    use super::UserEvent;

    #[derive(Debug)]
    struct ProxyIvars {
        proxy: EventLoopProxy<UserEvent>,
    }

    define_class!(
        #[unsafe(super = NSObject)]
        #[thread_kind = MainThreadOnly]
        #[ivars = ProxyIvars]
        struct MenuTarget;

    unsafe impl NSObjectProtocol for MenuTarget {}

        impl MenuTarget {
            #[unsafe(method(openMenuDocument:))]
            fn open_menu_document(&self, _sender: Option<&AnyObject>) {
                let ctx = super::new_action_context("macos-menu-open");
                super::log_event(
                    "macos.menu.action",
                    Some(ctx.event_id),
                    "macOS menu action openMenuDocument:",
                    "",
                );
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::OpenFile(ctx));
            }

            #[unsafe(method(saveMenuDocument:))]
            fn save_menu_document(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action saveMenuDocument:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::SaveDocument);
            }

            #[unsafe(method(toggleDocumentMode:))]
            fn toggle_document_mode(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action toggleDocumentMode:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::ToggleMode);
            }

            #[unsafe(method(showAboutPanel:))]
            fn show_about_panel(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action showAboutPanel:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::ShowAbout);
            }

            #[unsafe(method(exitApplication:))]
            fn exit_application(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action exitApplication:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::Exit);
            }
        }
    );

    impl MenuTarget {
        fn new(mtm: MainThreadMarker, proxy: EventLoopProxy<UserEvent>) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(ProxyIvars { proxy });
            unsafe { objc2::msg_send![super(this), init] }
        }
    }

    pub fn install(proxy: &EventLoopProxy<UserEvent>) -> Result<(), Box<dyn Error>> {
        let mtm = MainThreadMarker::new().ok_or("menu setup must run on main thread")?;
        let app = NSApplication::sharedApplication(mtm);
        // AppKit menu targets and delegates are not retained for us here,
        // so keep them alive for the app lifetime.
        let target = Box::leak(Box::new(MenuTarget::new(mtm, proxy.clone())));

        let main_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("MainMenu"));

        let app_menu_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("MarkHola"),
                None,
                ns_string!(""),
            )
        };
        main_menu.addItem(&app_menu_item);

        let app_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("MarkHola"));
        let about_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("About MarkHola"),
                Some(sel!(showAboutPanel:)),
                ns_string!(""),
            )
        };
        unsafe { about_item.setTarget(Some((&**target).as_ref())) };
        app_menu.addItem(&about_item);
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));
        let quit_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Exit"),
                Some(sel!(exitApplication:)),
                ns_string!("q"),
            )
        };
        unsafe { quit_item.setTarget(Some((&**target).as_ref())) };
        quit_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        app_menu.addItem(&quit_item);
        app_menu_item.setSubmenu(Some(&app_menu));

        let file_menu_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("File"),
                None,
                ns_string!(""),
            )
        };
        main_menu.addItem(&file_menu_item);

        let file_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("File"));
        let open_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Open"),
                Some(sel!(openMenuDocument:)),
                ns_string!("o"),
            )
        };
        unsafe { open_item.setTarget(Some((&**target).as_ref())) };
        open_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        file_menu.addItem(&open_item);

        let save_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Save"),
                Some(sel!(saveMenuDocument:)),
                ns_string!("s"),
            )
        };
        unsafe { save_item.setTarget(Some((&**target).as_ref())) };
        save_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        file_menu.addItem(&save_item);

        let toggle_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Toggle Mode"),
                Some(sel!(toggleDocumentMode:)),
                ns_string!("/"),
            )
        };
        unsafe { toggle_item.setTarget(Some((&**target).as_ref())) };
        toggle_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        file_menu.addItem(&toggle_item);

        file_menu.addItem(&NSMenuItem::separatorItem(mtm));
        let close_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Close"),
                Some(sel!(exitApplication:)),
                ns_string!("w"),
            )
        };
        unsafe { close_item.setTarget(Some((&**target).as_ref())) };
        close_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        file_menu.addItem(&close_item);

        let exit_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Exit"),
                Some(sel!(exitApplication:)),
                ns_string!("q"),
            )
        };
        unsafe { exit_item.setTarget(Some((&**target).as_ref())) };
        exit_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        file_menu.addItem(&exit_item);
        file_menu_item.setSubmenu(Some(&file_menu));

        let edit_menu_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Edit"),
                None,
                ns_string!(""),
            )
        };
        main_menu.addItem(&edit_menu_item);

        let edit_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Edit"));

        let undo_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Undo"),
                Some(sel!(undo:)),
                ns_string!("z"),
            )
        };
        undo_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        edit_menu.addItem(&undo_item);

        let redo_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Redo"),
                Some(sel!(redo:)),
                ns_string!("r"),
            )
        };
        redo_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        edit_menu.addItem(&redo_item);

        edit_menu.addItem(&NSMenuItem::separatorItem(mtm));

        let cut_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Cut"),
                Some(sel!(cut:)),
                ns_string!("x"),
            )
        };
        cut_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        edit_menu.addItem(&cut_item);

        let copy_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Copy"),
                Some(sel!(copy:)),
                ns_string!("c"),
            )
        };
        copy_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        edit_menu.addItem(&copy_item);

        let paste_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Paste"),
                Some(sel!(paste:)),
                ns_string!("v"),
            )
        };
        paste_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        edit_menu.addItem(&paste_item);

        edit_menu.addItem(&NSMenuItem::separatorItem(mtm));

        let select_all_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Select All"),
                Some(sel!(selectAll:)),
                ns_string!("a"),
            )
        };
        select_all_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        edit_menu.addItem(&select_all_item);

        edit_menu_item.setSubmenu(Some(&edit_menu));

        app.setMainMenu(Some(&main_menu));

        let _ = NSApp(mtm);
        Ok(())
    }
}
