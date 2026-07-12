use std::ffi::{c_char, c_long, CStr};
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Once;
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
use wry::{PageLoadEvent, WebView, WebViewBuilder};

use crate::document::{ActiveDocument, DocumentSnapshot, DocumentTabSnapshot, DocumentMode};
use crate::file_io;
use crate::html_export::{self, HtmlExportOutcome};
use crate::pdf_export::{self, PdfExportOutcome};
use crate::printing::{self, PrintOutcome};
use crate::render_assets;
use crate::workspace::{DocumentWorkspace, WorkspaceOpenResult};

const WINDOW_TITLE: &str = "MarkHola";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_AUTHOR: &str = "Ronnie Deng";
const APP_GITHUB_URL: &str = "https://github.com/phpple/markhola";
const APP_BUILD_TARGET: &str = std::env::consts::ARCH;
const APP_BUILD_PLATFORM: &str = std::env::consts::OS;
const DEBUG_LOG_DIR: &str = "/var/log/markhola";
const DEBUG_LOG_FALLBACK_PATH: &str = "/tmp/markhola.log";
static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);
static PANIC_HOOK_ONCE: Once = Once::new();

#[derive(Clone, Debug)]
enum UserEvent {
    OpenFile(ActionContext),
    OpenPath(OpenPathRequest),
    ActivateDocument(u64),
    ActivateNextDocument,
    ActivatePreviousDocument,
    CloseDocument(u64),
    CloseCurrentDocument,
    CloseOtherDocuments,
    CloseAllDocuments,
    ShellReady,
    RecoverShell(String),
    OpenExternal(String),
    SaveDocument,
    SaveDocumentAs,
    ExportPdf,
    ExportHtml,
    PrintDocument,
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
    action_path: Option<&'a str>,
    action_label: Option<&'a str>,
}

#[derive(Clone, Debug, Serialize)]
struct WorkspacePresentation {
    tabs: Vec<DocumentTabSnapshot>,
    active_document: Option<DocumentSnapshot>,
    status_message: String,
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

pub(crate) fn log_event(stage: &str, event_id: Option<u64>, message: &str, extra: impl AsRef<str>) {
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
        UserEvent::ActivateDocument(document_id) => (
            None,
            "ActivateDocument",
            format!("source={} document_id={document_id}", stage_source),
        ),
        UserEvent::ActivateNextDocument => (None, "ActivateNextDocument", format!("source={stage_source}")),
        UserEvent::ActivatePreviousDocument => (None, "ActivatePreviousDocument", format!("source={stage_source}")),
        UserEvent::CloseDocument(document_id) => (
            None,
            "CloseDocument",
            format!("source={} document_id={document_id}", stage_source),
        ),
        UserEvent::CloseCurrentDocument => (None, "CloseCurrentDocument", format!("source={stage_source}")),
        UserEvent::CloseOtherDocuments => (None, "CloseOtherDocuments", format!("source={stage_source}")),
        UserEvent::CloseAllDocuments => (None, "CloseAllDocuments", format!("source={stage_source}")),
        UserEvent::ShellReady => (None, "ShellReady", format!("source={stage_source}")),
        UserEvent::RecoverShell(url) => (None, "RecoverShell", format!("source={} url={url}", stage_source)),
        UserEvent::OpenExternal(href) => (None, "OpenExternal", format!("source={} href={href}", stage_source)),
        UserEvent::SaveDocument => (None, "SaveDocument", format!("source={stage_source}")),
        UserEvent::SaveDocumentAs => (None, "SaveDocumentAs", format!("source={stage_source}")),
        UserEvent::ExportPdf => (None, "ExportPdf", format!("source={stage_source}")),
        UserEvent::ExportHtml => (None, "ExportHtml", format!("source={stage_source}")),
        UserEvent::PrintDocument => (None, "PrintDocument", format!("source={stage_source}")),
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
    let mut workspace = DocumentWorkspace::new();
    let mut shell_ready = false;
    let mut shell_recovery_pending = false;
    let mut pending_open_requests: Vec<OpenPathRequest> = Vec::new();

    let window = WindowBuilder::new()
        .with_title(WINDOW_TITLE)
        .with_inner_size(LogicalSize::new(1120.0, 760.0))
        .with_min_inner_size(LogicalSize::new(800.0, 560.0))
        .build(&event_loop)?;

    let ipc_proxy = proxy.clone();
    let page_load_proxy = proxy.clone();
    let suppress_blank_shell_recovery = Arc::new(AtomicBool::new(true));
    let page_load_recovery_guard = Arc::clone(&suppress_blank_shell_recovery);
    let webview = WebViewBuilder::new()
        .with_html(app_shell_html())
        .with_devtools(true)
        .with_ipc_handler(move |request| {
            handle_ipc_message(&ipc_proxy, request.body().to_owned());
        })
        .with_on_page_load_handler(move |event, url| {
            let event_name = match event {
                PageLoadEvent::Started => "started",
                PageLoadEvent::Finished => "finished",
            };
            log_event("webview.page_load", None, "webview page load event", format!("event={event_name} url={url}"));
            if matches!(event, PageLoadEvent::Finished)
                && should_dispatch_shell_recovery(&url, &page_load_recovery_guard)
            {
                dispatch_user_event(&page_load_proxy, "page-load", UserEvent::RecoverShell(url));
            }
        })
        .build(&window)?;

    #[cfg(target_os = "macos")]
    macos_menu::install(&proxy)?;
    sync_native_menu_state(&workspace);

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
                    if resolve_all_pending_changes(&window, &webview, &mut workspace) {
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
                            KeyCode::KeyP => {
                                log_event("keyboard.shortcut", None, "keyboard shortcut triggered", "key=Command+P");
                                dispatch_user_event(&proxy, "keyboard", UserEvent::PrintDocument);
                            }
                            KeyCode::KeyW => {
                                log_event("keyboard.shortcut", None, "keyboard shortcut triggered", "key=Command+W");
                                dispatch_user_event(&proxy, "keyboard", UserEvent::CloseCurrentDocument);
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
                match open_document_dialog(ctx.event_id) {
                    Some(path) => {
                        log_event(
                            "file_dialog.selected",
                            Some(ctx.event_id),
                            "open dialog returned path",
                            format!("path={}", path.display()),
                        );
                        open_document(&window, &webview, &mut workspace, &path, Some(ctx.event_id))
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
                open_document(&window, &webview, &mut workspace, &path, Some(ctx.event_id));
            }
            Event::UserEvent(UserEvent::ActivateDocument(document_id)) => {
                log_event(
                    "user_event.received",
                    None,
                    "handling UserEvent::ActivateDocument",
                    format!("document_id={document_id}"),
                );
                if workspace.activate_document(document_id) {
                    sync_workspace_state(&window, &webview, &workspace, "Document switched.");
                } else {
                    render_status(&webview, "Document tab no longer exists.", "error");
                }
            }
            Event::UserEvent(UserEvent::ActivateNextDocument) => {
                log_event("user_event.received", None, "handling UserEvent::ActivateNextDocument", "");
                if workspace.activate_next_document() {
                    sync_workspace_state(&window, &webview, &workspace, "Switched to next tab.");
                }
            }
            Event::UserEvent(UserEvent::ActivatePreviousDocument) => {
                log_event("user_event.received", None, "handling UserEvent::ActivatePreviousDocument", "");
                if workspace.activate_previous_document() {
                    sync_workspace_state(&window, &webview, &workspace, "Switched to previous tab.");
                }
            }
            Event::UserEvent(UserEvent::CloseDocument(document_id)) => {
                log_event(
                    "user_event.received",
                    None,
                    "handling UserEvent::CloseDocument",
                    format!("document_id={document_id}"),
                );
                close_document_tab(&window, &webview, &mut workspace, document_id, "Document closed.");
            }
            Event::UserEvent(UserEvent::CloseCurrentDocument) => {
                log_event("user_event.received", None, "handling UserEvent::CloseCurrentDocument", "");
                if let Some(document_id) = workspace.active_document_id() {
                    close_document_tab(&window, &webview, &mut workspace, document_id, "Document closed.");
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(UserEvent::CloseOtherDocuments) => {
                log_event("user_event.received", None, "handling UserEvent::CloseOtherDocuments", "");
                if let Some(active_document_id) = workspace.active_document_id() {
                    let document_ids = workspace.other_document_ids(active_document_id);
                    if document_ids.is_empty() {
                        render_status(&webview, "No other tabs to close.", "info");
                    } else {
                        close_document_tabs(&window, &webview, &mut workspace, &document_ids, "Other tabs closed.");
                    }
                } else {
                    render_status(&webview, "No document opened.", "info");
                }
            }
            Event::UserEvent(UserEvent::CloseAllDocuments) => {
                log_event("user_event.received", None, "handling UserEvent::CloseAllDocuments", "");
                let document_ids = workspace.document_ids();
                if document_ids.is_empty() {
                    render_status(&webview, "No document opened.", "info");
                } else {
                    close_document_tabs(&window, &webview, &mut workspace, &document_ids, "All tabs closed.");
                }
            }
            Event::UserEvent(UserEvent::ShellReady) => {
                let shell_was_ready = shell_ready;
                shell_ready = true;
                log_event(
                    "shell.ready",
                    None,
                    "webview shell reported ready",
                    format!("pending_open_requests={}", pending_open_requests.len()),
                );

                if (shell_was_ready || shell_recovery_pending) && workspace.active_document().is_some() {
                    shell_recovery_pending = false;
                    match reload_workspace_documents_from_disk(&mut workspace) {
                        Ok(reload_status) => {
                            present_workspace(&window, &webview, &workspace, &reload_status, true);
                        }
                        Err(message) => {
                            present_workspace(&window, &webview, &workspace, &message, true);
                        }
                    }
                }

                for request in pending_open_requests.drain(..) {
                    dispatch_user_event(&proxy, "shell-ready-flush", UserEvent::OpenPath(request));
                }
            }
            Event::UserEvent(UserEvent::RecoverShell(url)) => {
                log_event("user_event.received", None, "handling UserEvent::RecoverShell", format!("url={url}"));
                shell_ready = false;
                shell_recovery_pending = true;
                suppress_blank_shell_recovery.store(true, Ordering::SeqCst);
                if let Err(error) = webview.load_html(&app_shell_html()) {
                    shell_recovery_pending = false;
                    suppress_blank_shell_recovery.store(false, Ordering::SeqCst);
                    render_status(&webview, &format!("Failed to recover the document view: {error}"), "error");
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
                save_active_document(&window, &webview, &mut workspace);
            }
            Event::UserEvent(UserEvent::SaveDocumentAs) => {
                log_event("user_event.received", None, "handling UserEvent::SaveDocumentAs", "");
                save_active_document_as(&window, &webview, &mut workspace);
            }
            Event::UserEvent(UserEvent::ExportPdf) => {
                log_event("user_event.received", None, "handling UserEvent::ExportPdf", "");
                match workspace.active_document() {
                    Some(document) => match pdf_export::export_document(document) {
                        Ok(PdfExportOutcome::Exported(path)) => {
                            render_status_with_action(
                                &webview,
                                &format!("Exported PDF: {}", path.display()),
                                "info",
                                Some(&path.display().to_string()),
                                Some("Open"),
                            );
                        }
                        Ok(PdfExportOutcome::Cancelled) => {
                            render_status(&webview, "Export cancelled.", "info");
                        }
                        Err(message) => {
                            render_status(&webview, &message, "error");
                        }
                    },
                    None => {
                        render_status(&webview, "No document opened.", "error");
                    }
                }
            }
            Event::UserEvent(UserEvent::ExportHtml) => {
                log_event("user_event.received", None, "handling UserEvent::ExportHtml", "");
                match workspace.active_document() {
                    Some(document) => match html_export::export_document(document) {
                        Ok(HtmlExportOutcome::Exported(path)) => {
                            render_status_with_action(
                                &webview,
                                &format!("Exported HTML: {}", path.display()),
                                "info",
                                Some(&path.display().to_string()),
                                Some("Open"),
                            );
                        }
                        Ok(HtmlExportOutcome::Cancelled) => {
                            render_status(&webview, "Export cancelled.", "info");
                        }
                        Err(message) => {
                            render_status(&webview, &message, "error");
                        }
                    },
                    None => {
                        render_status(&webview, "No document opened.", "error");
                    }
                }
            }
            Event::UserEvent(UserEvent::PrintDocument) => {
                log_event("user_event.received", None, "handling UserEvent::PrintDocument", "");
                match workspace.active_document() {
                    Some(document) => match printing::print_document(document) {
                        Ok(PrintOutcome::Started) => {
                            render_status(&webview, "Print panel opened.", "info");
                        }
                        Ok(PrintOutcome::Cancelled) => {
                            render_status(&webview, "Print cancelled.", "info");
                        }
                        Err(message) => {
                            render_status(&webview, &message, "error");
                        }
                    },
                    None => {
                        render_status(&webview, "No document opened.", "error");
                    }
                }
            }
            Event::UserEvent(UserEvent::ToggleMode) => {
                log_event("user_event.received", None, "handling UserEvent::ToggleMode", "");
                let status = if let Some(document) = workspace.active_document_mut() {
                    document.toggle_mode();
                    Some(match document.mode() {
                        DocumentMode::Readonly => "Readonly preview updated.",
                        DocumentMode::Writable => "Writable mode enabled.",
                    })
                } else {
                    None
                };

                if let Some(status) = status {
                    present_workspace(&window, &webview, &workspace, status, true);
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
                let has_active_document = if let Some(document) = workspace.active_document_mut() {
                    document.update_markdown(markdown);
                    true
                } else {
                    false
                };

                if has_active_document {
                    sync_workspace_state(&window, &webview, &workspace, "Unsaved changes.");
                }
            }
            Event::UserEvent(UserEvent::ShowAbout) => {
                log_event("user_event.received", None, "handling UserEvent::ShowAbout", "");
                render_about(&webview);
            }
            Event::UserEvent(UserEvent::Exit) => {
                log_event("user_event.received", None, "handling UserEvent::Exit", "");
                if resolve_all_pending_changes(&window, &webview, &mut workspace) {
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
        Some("activate-document") => {
            if let Some(document_id) = value.get("documentId").and_then(Value::as_u64) {
                dispatch_user_event(proxy, "ipc", UserEvent::ActivateDocument(document_id));
            }
        }
        Some("close-document") => {
            if let Some(document_id) = value.get("documentId").and_then(Value::as_u64) {
                dispatch_user_event(proxy, "ipc", UserEvent::CloseDocument(document_id));
            }
        }
        Some("close-current-document") => {
            dispatch_user_event(proxy, "ipc", UserEvent::CloseCurrentDocument);
        }
        Some("request-save") => {
            dispatch_user_event(proxy, "ipc", UserEvent::SaveDocument);
        }
        Some("request-save-as") => {
            dispatch_user_event(proxy, "ipc", UserEvent::SaveDocumentAs);
        }
        Some("request-export-pdf") => {
            dispatch_user_event(proxy, "ipc", UserEvent::ExportPdf);
        }
        Some("request-export-html") => {
            dispatch_user_event(proxy, "ipc", UserEvent::ExportHtml);
        }
        Some("request-print") => {
            dispatch_user_event(proxy, "ipc", UserEvent::PrintDocument);
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
    workspace: &mut DocumentWorkspace,
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

    if let Some(document_id) = workspace.find_by_path(path) {
        workspace.activate_document(document_id);
        sync_workspace_state(window, webview, workspace, "Document already opened. Switched to tab.");
        return;
    }

    match load_document(workspace.next_document_id(), path) {
        Ok(document) => {
            log_event(
                "open_document.end",
                event_id,
                "open_document success",
                format!("path={}", path.display()),
            );
            match workspace.open_document(document) {
                WorkspaceOpenResult::OpenedNew(_) => {
                    present_workspace(window, webview, workspace, "Document loaded.", true);
                }
                WorkspaceOpenResult::ActivatedExisting(_) => {
                    sync_workspace_state(window, webview, workspace, "Document already opened. Switched to tab.");
                }
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

fn load_document(document_id: u64, path: &PathBuf) -> Result<ActiveDocument, String> {
    log_event("load_document.begin", None, "load_document path", format!("path={}", path.display()));
    let markdown = file_io::load_markdown(path)?;
    let base_url = file_io::directory_base_url(path)?;
    Ok(ActiveDocument::open_with_id(document_id, path.clone(), markdown, base_url))
}

fn reload_workspace_documents_from_disk(workspace: &mut DocumentWorkspace) -> Result<String, String> {
    let document_ids = workspace.document_ids();
    let mut reloaded = 0usize;
    let mut skipped_dirty = 0usize;
    let mut failures = Vec::new();

    for document_id in document_ids {
        let Some(document) = workspace.document_by_id_mut(document_id) else {
            continue;
        };

        if document.is_dirty() {
            skipped_dirty += 1;
            continue;
        }

        let path = document.file_path().to_path_buf();
        match file_io::load_markdown(&path) {
            Ok(markdown) => {
                document.reload_from_disk_markdown(markdown);
                reloaded += 1;
            }
            Err(error) => failures.push(format!("{}: {error}", path.display())),
        }
    }

    if let Some(first_failure) = failures.first() {
        return Err(format!("Reload failed: {first_failure}"));
    }

    Ok(match (reloaded, skipped_dirty) {
        (0, 0) => "Document reloaded.".to_string(),
        (_, 0) => "Document reloaded.".to_string(),
        (_, 1) => "Document reloaded. One unsaved tab was kept as-is.".to_string(),
        (_, count) => format!("Document reloaded. {count} unsaved tabs were kept as-is."),
    })
}

fn save_document(document: &mut ActiveDocument) -> Result<(), String> {
    file_io::save_markdown(document.file_path(), document.markdown())?;
    document.mark_saved();
    Ok(())
}

fn save_active_document(window: &Window, webview: &WebView, workspace: &mut DocumentWorkspace) -> bool {
    let Some(document) = workspace.active_document_mut() else {
        render_status(webview, "No document to save.", "error");
        return false;
    };

    if let Err(message) = save_document(document) {
        render_status(webview, &message, "error");
        return false;
    }

    sync_workspace_state(window, webview, workspace, "Saved.");
    true
}

fn save_active_document_as(window: &Window, webview: &WebView, workspace: &mut DocumentWorkspace) -> bool {
    let Some(document) = workspace.active_document() else {
        render_status(webview, "No document to save.", "error");
        return false;
    };

    let document_id = document.id();
    let document_directory = document
        .file_path()
        .parent()
        .unwrap_or(document.file_path())
        .to_path_buf();
    let document_name = document.file_name().to_string();
    let document_markdown = document.markdown().to_string();

    let Some(path) = FileDialog::new()
        .add_filter("Markdown", &["md", "markdown"])
        .set_title("Save Markdown File As")
        .set_directory(&document_directory)
        .set_file_name(&document_name)
        .save_file()
    else {
        render_status(webview, "Save As cancelled.", "info");
        return false;
    };

    if workspace
        .find_by_path_excluding(&path, document_id)
        .is_some()
    {
        render_status(
            webview,
            "Save As target is already open in another tab.",
            "error",
        );
        return false;
    }

    if let Err(error) = file_io::save_markdown(&path, &document_markdown) {
        render_status(webview, &error, "error");
        return false;
    }

    let base_url = match file_io::directory_base_url(&path) {
        Ok(base_url) => base_url,
        Err(error) => {
            render_status(webview, &error, "error");
            return false;
        }
    };

    let Some(document) = workspace.active_document_mut() else {
        render_status(webview, "No document to save.", "error");
        return false;
    };
    document.replace_file_path(path.clone(), base_url);
    sync_workspace_state(window, webview, workspace, "Saved to new path.");
    true
}

fn resolve_document_pending_changes(window: &Window, webview: &WebView, document: &mut ActiveDocument) -> bool {
    if !document.is_dirty() {
        return true;
    }

    match ask_pending_changes_action(window, document.file_name()) {
        PendingChangesAction::Save => match save_document(document) {
            Ok(()) => true,
            Err(message) => {
                render_status(webview, &message, "error");
                false
            }
        },
        PendingChangesAction::Discard => true,
        PendingChangesAction::Cancel => {
            render_status(webview, "Action cancelled.", "info");
            false
        }
    }
}

fn resolve_all_pending_changes(window: &Window, webview: &WebView, workspace: &mut DocumentWorkspace) -> bool {
    let document_ids = workspace
        .tab_snapshots()
        .into_iter()
        .map(|tab| tab.document_id)
        .collect::<Vec<_>>();

    for document_id in document_ids {
        let Some(document) = workspace.document_by_id_mut(document_id) else {
            continue;
        };

        if !resolve_document_pending_changes(window, webview, document) {
            return false;
        }
    }

    true
}

fn close_document_tab(
    window: &Window,
    webview: &WebView,
    workspace: &mut DocumentWorkspace,
    document_id: u64,
    status: &str,
) -> bool {
    let Some(document) = workspace.document_by_id_mut(document_id) else {
        render_status(webview, "Document tab no longer exists.", "error");
        return false;
    };

    if !resolve_document_pending_changes(window, webview, document) {
        return false;
    }

    workspace.close_document(document_id);
    sync_workspace_state(window, webview, workspace, status);
    true
}

fn close_document_tabs(
    window: &Window,
    webview: &WebView,
    workspace: &mut DocumentWorkspace,
    document_ids: &[u64],
    status: &str,
) -> bool {
    for document_id in document_ids {
        let Some(document) = workspace.document_by_id_mut(*document_id) else {
            continue;
        };

        if !resolve_document_pending_changes(window, webview, document) {
            return false;
        }
    }

    for document_id in document_ids {
        workspace.close_document(*document_id);
    }

    sync_workspace_state(window, webview, workspace, status);
    true
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

fn present_workspace(
    window: &Window,
    webview: &WebView,
    workspace: &DocumentWorkspace,
    status: &str,
    full_render: bool,
) {
    update_window_title(window, workspace.active_window_title().as_deref());
    sync_native_menu_state(workspace);

    if full_render {
        render_workspace(webview, workspace, status);
    } else {
        sync_workspace_state(window, webview, workspace, status);
    }
}

fn update_window_title(window: &Window, title: Option<&str>) {
    let title = title.unwrap_or(WINDOW_TITLE);
    window.set_title(&title);
}

fn sync_native_menu_state(workspace: &DocumentWorkspace) {
    #[cfg(target_os = "macos")]
    macos_menu::set_document_output_enabled(workspace.active_document().is_some());
}

fn workspace_presentation(workspace: &DocumentWorkspace, status: &str) -> WorkspacePresentation {
    WorkspacePresentation {
        tabs: workspace.tab_snapshots(),
        active_document: workspace.active_document_snapshot(),
        status_message: status.to_string(),
    }
}

fn render_workspace(webview: &WebView, workspace: &DocumentWorkspace, status: &str) {
    let payload = workspace_presentation(workspace, status);
    let serialized = match serde_json::to_string(&payload) {
        Ok(serialized) => serialized,
        Err(error) => {
            render_status(webview, &format!("Failed to serialize workspace: {error}"), "error");
            return;
        }
    };

    let script = format!("window.renderWorkspace({serialized});");
    if let Err(error) = webview.evaluate_script(&script) {
        render_status(webview, &format!("WebView script error: {error}"), "error");
    }
}

fn sync_workspace_state(window: &Window, webview: &WebView, workspace: &DocumentWorkspace, status: &str) {
    update_window_title(window, workspace.active_window_title().as_deref());

    let payload = workspace_presentation(workspace, status);
    let serialized = match serde_json::to_string(&payload) {
        Ok(serialized) => serialized,
        Err(error) => {
            render_status(webview, &format!("Failed to serialize workspace: {error}"), "error");
            return;
        }
    };

    let script = format!("window.updateWorkspaceState({serialized});");
    if let Err(error) = webview.evaluate_script(&script) {
        render_status(webview, &format!("WebView script error: {error}"), "error");
    }
}

fn render_status(webview: &WebView, message: &str, level: &str) {
    render_status_with_action(webview, message, level, None, None);
}

fn render_status_with_action(
    webview: &WebView,
    message: &str,
    level: &str,
    action_path: Option<&str>,
    action_label: Option<&str>,
) {
    let payload = StatusPayload {
        message,
        level,
        action_path,
        action_label,
    };
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
        serde_json::to_string(APP_VERSION).unwrap_or_else(|_| "\"0.7.0\"".to_string()),
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
        .replace("__APP_THEME__", &render_assets::load_app_theme_css_for_inline_style())
        .replace(
            "__MERMAID_RUNTIME__",
            &render_assets::mermaid_runtime_for_inline_script(),
        )
        .replace(
            "__MATHJAX_RUNTIME__",
            &render_assets::mathjax_runtime_for_inline_script(),
        )
}

fn should_recover_shell_on_page_load(url: &str) -> bool {
    let trimmed = url.trim();
    trimmed.is_empty() || trimmed == "about:blank"
}

fn should_dispatch_shell_recovery(url: &str, suppress_once: &AtomicBool) -> bool {
    should_recover_shell_on_page_load(url) && !suppress_once.swap(false, Ordering::SeqCst)
}

const APP_SHELL_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <base id="document-base" href="" />
    <title>MarkHola</title>
    <style>__APP_THEME__</style>
  </head>
  <body>
        <div class="app">
          <div class="tabs-bar hidden" id="tabsBar"></div>
          <section class="preview-shell">
            <div class="workspace">
              <div class="empty-state pane" id="emptyState">
                <div class="empty-card">
                  <h2>No document opened</h2>
                  <p>Open, drag, or drop a Markdown file to preview or edit the current Markdown source.</p>
                </div>
              </div>
              <div class="preview pane hidden" id="previewPane">
                <div class="preview-header" id="previewHeader">
                  <div class="preview-title">
                    <strong id="documentTitle">Preview</strong>
                    <span id="documentSubtitle">Use File > Open, Command+O, or drag a Markdown file into the window.</span>
                  </div>
                  <span id="status" class="status" data-level="info">Ready.</span>
                </div>
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
            <span class="about-value" id="aboutVersion">0.7.0</span>
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
      const previewHeader = document.getElementById("previewHeader");
      const tabsBar = document.getElementById("tabsBar");
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
      let currentDocumentId = null;

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

      const attachHeaderForMode = (mode) => {
        if (mode === "writable") {
          if (editorPane.firstElementChild !== previewHeader) {
            editorPane.insertBefore(previewHeader, editorPane.firstChild);
          }
          return;
        }

        if (previewPane.firstElementChild !== previewHeader) {
          previewPane.insertBefore(previewHeader, previewPane.firstChild);
        }
      };

      const showPaneForMode = (mode) => {
        attachHeaderForMode(mode);
        const hasDocument = mode === "readonly" || mode === "writable";
        emptyState.classList.toggle("hidden", hasDocument);
        previewPane.classList.toggle("hidden", mode !== "readonly");
        editorPane.classList.toggle("hidden", mode !== "writable");
      };

      const renderTabs = (tabs) => {
        if (!tabs.length) {
          tabsBar.classList.add("hidden");
          tabsBar.innerHTML = "";
          return;
        }

        tabsBar.classList.remove("hidden");
        tabsBar.innerHTML = tabs
          .map((tab) => {
            const activeClass = tab.active ? " active" : "";
            const dirty = tab.dirty ? `<span class="document-tab__dirty" aria-hidden="true"></span>` : "";
            return `
              <div class="document-tab${activeClass}" data-document-id="${tab.document_id}" title="${escapeHtml(tab.title)}">
                <span class="document-tab__name">${escapeHtml(tab.file_name)}</span>
                ${dirty}
                <button class="document-tab__close" type="button" data-close-document="${tab.document_id}" aria-label="Close ${escapeHtml(tab.file_name)}">&times;</button>
              </div>
            `;
          })
          .join("");
      };

      const resetWorkspaceChrome = (statusMessage) => {
        document.title = "MarkHola";
        documentTitle.textContent = "Preview";
        documentSubtitle.textContent = "Use File > Open, Command+O, or drag a Markdown file into the window.";
        filePath.textContent = "Path: No file opened";
        wordCount.innerHTML = "<strong>Words</strong> 0";
        lineCount.innerHTML = "<strong>Lines</strong> 0";
        modeState.innerHTML = "<strong>Mode</strong> Readonly";
        saveState.innerHTML = "<strong>Status</strong> Ready.";
        documentBase.setAttribute("href", "");
        showPaneForMode(null);
        window.showStatus({ message: statusMessage || "Ready.", level: "info" });
      };

      const applyWorkspaceChrome = (payload) => {
        renderTabs(payload.tabs || []);
        const active = payload.active_document;

        if (!active) {
          resetWorkspaceChrome(payload.status_message);
          return;
        }

        document.title = `${active.file_name}${active.dirty ? " *" : ""} - MarkHola`;
        documentTitle.textContent = active.title;
        documentSubtitle.textContent = active.file_name;
        filePath.textContent = `Path: ${active.file_path}`;
        wordCount.innerHTML = `<strong>Words</strong> ${active.word_count}`;
        lineCount.innerHTML = `<strong>Lines</strong> ${active.line_count}`;
        modeState.innerHTML = `<strong>Mode</strong> ${active.mode_label}`;
        saveState.innerHTML = `<strong>Status</strong> ${active.save_status}`;
        documentBase.setAttribute("href", active.base_url);
        showPaneForMode(active.mode);
        window.showStatus({ message: payload.status_message, level: active.dirty ? "warning" : "info" });
      };

      const escapeHtml = (value) =>
        value
          .replaceAll("&", "&amp;")
          .replaceAll("<", "&lt;")
          .replaceAll(">", "&gt;")
          .replaceAll('"', "&quot;")
          .replaceAll("'", "&#39;");

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
        } else if (event.key.toLowerCase() === "p") {
          event.preventDefault();
          window.ipc.postMessage(JSON.stringify({ kind: "request-print" }));
        } else if (event.key.toLowerCase() === "w") {
          event.preventDefault();
          window.ipc.postMessage(JSON.stringify({ kind: "close-current-document" }));
        } else if (event.key.toLowerCase() === "a" && isWritableMode()) {
          event.preventDefault();
          selectAllEditorText();
        } else if (event.key === "/") {
          event.preventDefault();
          window.ipc.postMessage(JSON.stringify({ kind: "toggle-mode" }));
        }
      });

      document.addEventListener("click", (event) => {
        const statusAction = event.target.closest("[data-open-path]");
        if (statusAction) {
          event.preventDefault();
          const path = statusAction.getAttribute("data-open-path") || "";
          if (path) {
            window.ipc.postMessage(JSON.stringify({ kind: "open-external", href: path }));
          }
          return;
        }

        const closeButton = event.target.closest("[data-close-document]");
        if (closeButton) {
          event.preventDefault();
          event.stopPropagation();
          window.ipc.postMessage(
            JSON.stringify({
              kind: "close-document",
              documentId: Number(closeButton.getAttribute("data-close-document"))
            })
          );
          return;
        }

        const tab = event.target.closest("[data-document-id]");
        if (tab) {
          const documentId = Number(tab.getAttribute("data-document-id"));
          if (Number.isFinite(documentId)) {
            window.ipc.postMessage(JSON.stringify({ kind: "activate-document", documentId }));
          }
          return;
        }

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
        const actionPath = payload.action_path || "";
        const actionLabel = payload.action_label || "";
        if (actionPath && actionLabel) {
          status.innerHTML = `${escapeHtml(payload.message)} <a href=\"#\" class=\"status__action\" data-open-path=\"${escapeHtml(actionPath)}\">${escapeHtml(actionLabel)}</a>`;
        } else {
          status.textContent = payload.message;
        }
        status.dataset.level = payload.level || "info";
      };

      const applyWorkspacePayload = (payload, forceRefresh) => {
        applyWorkspaceChrome(payload);
        const active = payload.active_document;
        const nextDocumentId = active ? active.document_id : null;
        const documentChanged = nextDocumentId !== currentDocumentId;

        if (!active) {
          currentDocumentId = null;
          content.innerHTML = "";
          editor.value = "";
          updateEditorLineNumbers();
          syncEditorScroll();
          return;
        }

        if (forceRefresh || documentChanged || active.mode === "readonly") {
          content.innerHTML = active.html;
        }

        if (forceRefresh || documentChanged) {
          editor.value = active.markdown;
          updateEditorLineNumbers();
          syncEditorScroll();
        }

        currentDocumentId = nextDocumentId;

        if (forceRefresh || documentChanged || active.mode === "readonly") {
          void renderReadonlyEnhancements();
        }
      };

      window.renderWorkspace = (payload) => {
        applyWorkspacePayload(payload, true);
      };

      window.updateWorkspaceState = (payload) => {
        applyWorkspacePayload(payload, false);
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::workspace::DocumentWorkspace;

    use std::sync::atomic::AtomicBool;

    use super::{
        load_document, reload_workspace_documents_from_disk, should_dispatch_shell_recovery,
        should_recover_shell_on_page_load,
    };

    fn temp_markdown_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("markhola-reload-{name}-{stamp}.md"))
    }

    #[test]
    fn reload_workspace_refreshes_active_document_from_disk() {
        let path = temp_markdown_path("reload");
        fs::write(&path, "# Before\nold content").unwrap();

        let mut workspace = DocumentWorkspace::new();
        let document = load_document(1, &path).unwrap();
        workspace.open_document(document);

        fs::write(&path, "# After\nnew content").unwrap();

        let status = reload_workspace_documents_from_disk(&mut workspace).unwrap();
        let snapshot = workspace.active_document_snapshot().unwrap();

        assert_eq!(status, "Document reloaded.");
        assert_eq!(snapshot.markdown, "# After\nnew content");
        assert!(snapshot.html.contains("After"));
        assert!(snapshot.html.contains("new content"));
        assert!(!snapshot.dirty);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn recovers_shell_when_page_load_finishes_on_blank_url() {
        assert!(should_recover_shell_on_page_load("about:blank"));
        assert!(should_recover_shell_on_page_load(""));
        assert!(!should_recover_shell_on_page_load("file:///tmp/demo.md"));
        assert!(!should_recover_shell_on_page_load("data:text/html,hello"));
    }

    #[test]
    fn suppresses_the_expected_blank_finish_once_before_recovering_again() {
        let suppress_once = AtomicBool::new(true);

        assert!(!should_dispatch_shell_recovery("about:blank", &suppress_once));
        assert!(should_dispatch_shell_recovery("about:blank", &suppress_once));
        assert!(!should_dispatch_shell_recovery("file:///tmp/demo.md", &suppress_once));
    }
}

#[cfg(target_os = "macos")]
mod macos_menu {
    use std::cell::RefCell;
    use std::error::Error;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{DefinedClass, MainThreadOnly, define_class, sel};
    use objc2_app_kit::{NSApp, NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
    use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, ns_string};
    use tao::event_loop::EventLoopProxy;

    use super::UserEvent;

    thread_local! {
        static EXPORT_PDF_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
        static EXPORT_HTML_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
        static SAVE_AS_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
        static PRINT_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    }

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

            #[unsafe(method(saveMenuDocumentAs:))]
            fn save_menu_document_as(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action saveMenuDocumentAs:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::SaveDocumentAs);
            }

            #[unsafe(method(exportPdfDocument:))]
            fn export_pdf_document(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action exportPdfDocument:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::ExportPdf);
            }

            #[unsafe(method(exportHtmlDocument:))]
            fn export_html_document(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action exportHtmlDocument:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::ExportHtml);
            }

            #[unsafe(method(printDocument:))]
            fn print_document(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action printDocument:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::PrintDocument);
            }

            #[unsafe(method(toggleDocumentMode:))]
            fn toggle_document_mode(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action toggleDocumentMode:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::ToggleMode);
            }

            #[unsafe(method(closeCurrentDocument:))]
            fn close_current_document(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action closeCurrentDocument:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::CloseCurrentDocument);
            }

            #[unsafe(method(activateNextDocument:))]
            fn activate_next_document(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action activateNextDocument:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::ActivateNextDocument);
            }

            #[unsafe(method(activatePreviousDocument:))]
            fn activate_previous_document(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action activatePreviousDocument:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::ActivatePreviousDocument);
            }

            #[unsafe(method(closeOtherDocuments:))]
            fn close_other_documents(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action closeOtherDocuments:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::CloseOtherDocuments);
            }

            #[unsafe(method(closeAllDocuments:))]
            fn close_all_documents(&self, _sender: Option<&AnyObject>) {
                super::log_event("macos.menu.action", None, "macOS menu action closeAllDocuments:", "");
                super::dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::CloseAllDocuments);
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

        let save_as_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Save As"),
                Some(sel!(saveMenuDocumentAs:)),
                ns_string!("S"),
            )
        };
        unsafe { save_as_item.setTarget(Some((&**target).as_ref())) };
        save_as_item.setKeyEquivalentModifierMask(
            NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
        );
        file_menu.addItem(&save_as_item);
        SAVE_AS_ITEM.with(|slot| {
            *slot.borrow_mut() = Some(save_as_item.clone());
        });

        let print_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Print"),
                Some(sel!(printDocument:)),
                ns_string!("p"),
            )
        };
        unsafe { print_item.setTarget(Some((&**target).as_ref())) };
        print_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        print_item.setEnabled(false);
        file_menu.addItem(&print_item);
        PRINT_ITEM.with(|slot| {
            *slot.borrow_mut() = Some(print_item.clone());
        });

        let export_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Export"),
                None,
                ns_string!(""),
            )
        };
        file_menu.addItem(&export_item);

        let export_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Export"));
        let export_pdf_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("PDF"),
                Some(sel!(exportPdfDocument:)),
                ns_string!(""),
            )
        };
        unsafe { export_pdf_item.setTarget(Some((&**target).as_ref())) };
        export_pdf_item.setEnabled(false);
        export_menu.addItem(&export_pdf_item);
        EXPORT_PDF_ITEM.with(|slot| {
            *slot.borrow_mut() = Some(export_pdf_item.clone());
        });

        let export_html_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("HTML"),
                Some(sel!(exportHtmlDocument:)),
                ns_string!(""),
            )
        };
        unsafe { export_html_item.setTarget(Some((&**target).as_ref())) };
        export_html_item.setEnabled(false);
        export_menu.addItem(&export_html_item);
        EXPORT_HTML_ITEM.with(|slot| {
            *slot.borrow_mut() = Some(export_html_item.clone());
        });
        export_item.setSubmenu(Some(&export_menu));

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
                Some(sel!(closeCurrentDocument:)),
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

        let tab_menu_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Tab"),
                None,
                ns_string!(""),
            )
        };
        main_menu.addItem(&tab_menu_item);

        let tab_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Tab"));

        let next_tab_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Next Tab"),
                Some(sel!(activateNextDocument:)),
                ns_string!("]"),
            )
        };
        unsafe { next_tab_item.setTarget(Some((&**target).as_ref())) };
        next_tab_item.setKeyEquivalentModifierMask(
            NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
        );
        tab_menu.addItem(&next_tab_item);

        let previous_tab_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Previous Tab"),
                Some(sel!(activatePreviousDocument:)),
                ns_string!("["),
            )
        };
        unsafe { previous_tab_item.setTarget(Some((&**target).as_ref())) };
        previous_tab_item.setKeyEquivalentModifierMask(
            NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
        );
        tab_menu.addItem(&previous_tab_item);

        tab_menu.addItem(&NSMenuItem::separatorItem(mtm));

        let close_tab_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Close Tab"),
                Some(sel!(closeCurrentDocument:)),
                ns_string!("w"),
            )
        };
        unsafe { close_tab_item.setTarget(Some((&**target).as_ref())) };
        close_tab_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        tab_menu.addItem(&close_tab_item);

        let close_other_tabs_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Close Other Tabs"),
                Some(sel!(closeOtherDocuments:)),
                ns_string!("w"),
            )
        };
        unsafe { close_other_tabs_item.setTarget(Some((&**target).as_ref())) };
        close_other_tabs_item.setKeyEquivalentModifierMask(
            NSEventModifierFlags::Command | NSEventModifierFlags::Option,
        );
        tab_menu.addItem(&close_other_tabs_item);

        let close_all_tabs_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Close All Tabs"),
                Some(sel!(closeAllDocuments:)),
                ns_string!("w"),
            )
        };
        unsafe { close_all_tabs_item.setTarget(Some((&**target).as_ref())) };
        close_all_tabs_item.setKeyEquivalentModifierMask(
            NSEventModifierFlags::Command | NSEventModifierFlags::Option | NSEventModifierFlags::Shift,
        );
        tab_menu.addItem(&close_all_tabs_item);

        tab_menu_item.setSubmenu(Some(&tab_menu));

        app.setMainMenu(Some(&main_menu));
        let _ = NSApp(mtm);
        Ok(())
    }
    pub fn set_document_output_enabled(enabled: bool) {
        SAVE_AS_ITEM.with(|slot| {
            if let Some(item) = slot.borrow().as_ref() {
                item.setEnabled(enabled);
            }
        });
        PRINT_ITEM.with(|slot| {
            if let Some(item) = slot.borrow().as_ref() {
                item.setEnabled(enabled);
            }
        });
        EXPORT_PDF_ITEM.with(|slot| {
            if let Some(item) = slot.borrow().as_ref() {
                item.setEnabled(enabled);
            }
        });
        EXPORT_HTML_ITEM.with(|slot| {
            if let Some(item) = slot.borrow().as_ref() {
                item.setEnabled(enabled);
            }
        });
    }
}
