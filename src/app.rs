use std::fs;
use std::path::{Path, PathBuf};

use rfd::FileDialog;
use tao::dpi::LogicalSize;
use tao::event::{ElementState, Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::keyboard::{KeyCode, ModifiersState};
use tao::window::WindowBuilder;
use serde::Serialize;
use serde_json::Value;
use url::Url;
use wry::{WebView, WebViewBuilder};

use crate::markdown;

const WINDOW_TITLE: &str = "MarkHola";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_AUTHOR: &str = "Ronnie Deng";
const APP_GITHUB_URL: &str = "https://github.com/phpple/markhola";
const APP_BUILD_TARGET: &str = std::env::consts::ARCH;
const APP_BUILD_PLATFORM: &str = std::env::consts::OS;

#[derive(Clone, Debug)]
enum UserEvent {
    OpenFile,
    OpenPath(PathBuf),
    OpenExternal(String),
    ShowAbout,
    Exit,
    Render(Result<DocumentPayload, String>),
}

#[derive(Clone, Debug, Serialize)]
struct DocumentPayload {
    file_name: String,
    file_path: String,
    title: String,
    base_url: String,
    word_count: usize,
    line_count: usize,
    html: String,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let mut modifiers = ModifiersState::default();

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
                render_info(&webview, "Ready. Open a Markdown file or press Command+O.");
            }
            Event::Opened { urls } => {
                if let Some(url) = urls.into_iter().find(|url| url.scheme() == "file") {
                    match url.to_file_path() {
                        Ok(path) => {
                            let _ = proxy.send_event(UserEvent::OpenPath(path));
                        }
                        Err(_) => {
                            render_error(&webview, "The requested file path is not valid.");
                        }
                    }
                }
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::ModifiersChanged(next_modifiers) => {
                    modifiers = next_modifiers;
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == ElementState::Released
                        && modifiers.super_key()
                        && event.physical_key == KeyCode::KeyO
                    {
                        let _ = proxy.send_event(UserEvent::OpenFile);
                    }
                }
                WindowEvent::HoveredFile(path) => {
                    let message = format!("Drop to open: {}", path.display());
                    render_info(&webview, &message);
                }
                WindowEvent::HoveredFileCancelled => {
                    render_info(&webview, "Ready. Open a Markdown file or press Command+O.");
                }
                WindowEvent::DroppedFile(path) => {
                    let _ = proxy.send_event(UserEvent::OpenPath(path));
                }
                _ => {}
            },
            Event::UserEvent(UserEvent::OpenFile) => {
                match open_document_dialog() {
                    OpenDialogResult::Selected(document) => {
                        let _ = proxy.send_event(UserEvent::Render(Ok(document)));
                    }
                    OpenDialogResult::Cancelled => {
                        render_info(&webview, "Open cancelled.");
                    }
                    OpenDialogResult::Failed(message) => {
                        let _ = proxy.send_event(UserEvent::Render(Err(message)));
                    }
                }
            }
            Event::UserEvent(UserEvent::OpenPath(path)) => {
                render_info(&webview, "Loading preview...");
                let _ = proxy.send_event(UserEvent::Render(load_document(&path)));
            }
            Event::UserEvent(UserEvent::OpenExternal(href)) => {
                if let Err(error) = open::that(href) {
                    render_error(&webview, &format!("Failed to open link: {error}"));
                }
            }
            Event::UserEvent(UserEvent::ShowAbout) => {
                render_about(&webview);
            }
            Event::UserEvent(UserEvent::Exit) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::Render(result)) => match result {
                Ok(document) => {
                    if let Err(error) = render_document(&webview, &document) {
                        render_error(&webview, &format!("Failed to render document: {error}"));
                    } else {
                        let title = format!("{} - {}", document.title, WINDOW_TITLE);
                        window.set_title(&title);
                    }
                }
                Err(message) => {
                    render_error(&webview, &message);
                }
            },
            _ => {}
        }
    });

    #[allow(unreachable_code)]
    Ok(())
}

enum OpenDialogResult {
    Selected(DocumentPayload),
    Cancelled,
    Failed(String),
}

fn handle_ipc_message(proxy: &EventLoopProxy<UserEvent>, payload: String) {
    let Ok(value) = serde_json::from_str::<Value>(&payload) else {
        return;
    };

    match value.get("kind").and_then(Value::as_str) {
        Some("open-file") => {
            let _ = proxy.send_event(UserEvent::OpenFile);
        }
        Some("open-external") => {
            if let Some(href) = value.get("href").and_then(Value::as_str) {
                let _ = proxy.send_event(UserEvent::OpenExternal(href.to_string()));
            }
        }
        _ => {}
    }
}

fn open_document_dialog() -> OpenDialogResult {
    let Some(path) = FileDialog::new()
        .add_filter("Markdown", &["md", "markdown"])
        .set_title("Open Markdown File")
        .pick_file()
    else {
        return OpenDialogResult::Cancelled;
    };

    match load_document(&path) {
        Ok(document) => OpenDialogResult::Selected(document),
        Err(message) => OpenDialogResult::Failed(message),
    }
}

fn load_document(path: &Path) -> Result<DocumentPayload, String> {
    ensure_supported_markdown_path(path)?;
    let markdown_text =
        fs::read_to_string(path).map_err(|error| format!("Failed to read file: {error}"))?;
    let title = markdown::extract_title(&markdown_text)
        .unwrap_or_else(|| file_name(path).unwrap_or_else(|| "Untitled".to_string()));
    let html = markdown::render_html(&markdown_text);
    let base_url = directory_base_url(path)
        .map_err(|error| format!("Failed to resolve document directory: {error}"))?;
    let line_count = markdown_text.lines().count();
    let word_count = markdown_text.split_whitespace().count();

    Ok(DocumentPayload {
        file_name: file_name(path).unwrap_or_else(|| "Untitled".to_string()),
        file_path: path.display().to_string(),
        title,
        base_url,
        word_count,
        line_count,
        html,
    })
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
}

fn ensure_supported_markdown_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("md") | Some("markdown") => Ok(()),
        _ => Err("Only .md and .markdown files are supported in v0.5.0.".to_string()),
    }
}

fn directory_base_url(path: &Path) -> Result<String, &'static str> {
    let directory = path
        .parent()
        .ok_or("Document path does not have a parent directory.")?;
    let url = Url::from_directory_path(directory)
        .map_err(|_| "Document directory cannot be converted to a file URL.")?;
    Ok(url.to_string())
}

fn render_document(webview: &WebView, document: &DocumentPayload) -> Result<(), String> {
    let payload = serde_json::to_string(document)
        .map_err(|error| format!("Failed to serialize document payload: {error}"))?;
    let script = format!("window.renderDocument({payload});");
    webview
        .evaluate_script(&script)
        .map_err(|error| format!("WebView script error: {error}"))
}

fn render_error(webview: &WebView, message: &str) {
    let script = format!(
        "window.renderError({});",
        serde_json::to_string(message).unwrap_or_else(|_| "\"Unexpected error.\"".to_string())
    );
    let _ = webview.evaluate_script(&script);
}

fn render_info(webview: &WebView, message: &str) {
    let script = format!(
        "window.renderInfo({});",
        serde_json::to_string(message).unwrap_or_else(|_| "\"Ready.\"".to_string())
    );
    let _ = webview.evaluate_script(&script);
}

fn render_about(webview: &WebView) {
    let script = format!(
        "window.showAbout({{version:{}, author:{}, githubUrl:{}, buildTarget:{}, buildPlatform:{}}});",
        serde_json::to_string(APP_VERSION).unwrap_or_else(|_| "\"0.5.0\"".to_string()),
        serde_json::to_string(APP_AUTHOR).unwrap_or_else(|_| "\"Ronnie Deng\"".to_string()),
        serde_json::to_string(APP_GITHUB_URL)
            .unwrap_or_else(|_| "\"https://github.com/phpple/markhola\"".to_string()),
        serde_json::to_string(APP_BUILD_TARGET).unwrap_or_else(|_| "\"unknown\"".to_string()),
        serde_json::to_string(APP_BUILD_PLATFORM).unwrap_or_else(|_| "\"unknown\"".to_string())
    );
    let _ = webview.evaluate_script(&script);
}

fn app_shell_html() -> &'static str {
    r##"<!DOCTYPE html>
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

      .preview {
        overflow: auto;
        padding: 24px;
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
        grid-template-columns: minmax(0, 1fr) auto auto auto;
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
        background: #211d19;
        color: #f8f5ee;
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
          <span id="status" class="status">Ready.</span>
        </div>
        <div class="preview" id="preview">
          <div class="empty-state" id="emptyState">
            <div class="empty-card">
              <h2>No document opened</h2>
              <p>Open, drag, or drop a Markdown file to preview headings, links, images, and tables.</p>
            </div>
          </div>
          <article class="markdown-body hidden" id="content"></article>
        </div>
      </section>

      <footer class="bottom-bar">
        <div class="bottom-item" id="filePath">Path: No file opened</div>
        <div class="bottom-item" id="wordCount"><strong>Words</strong> 0</div>
        <div class="bottom-item" id="lineCount"><strong>Lines</strong> 0</div>
        <div class="bottom-item" id="bottomStatus"><strong>Status</strong> Ready.</div>
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
            <p>Markdown reader and exporter</p>
          </div>
        </div>
        <div class="about-meta">
          <div class="about-meta-row">
            <strong>Version</strong>
            <span class="about-value" id="aboutVersion">0.5.0</span>
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
        <div class="about-footer">Built for local Markdown reading on Apple Silicon.</div>
      </div>
    </div>

    <script>
      const status = document.getElementById("status");
      const bottomStatus = document.getElementById("bottomStatus");
      const documentTitle = document.getElementById("documentTitle");
      const documentSubtitle = document.getElementById("documentSubtitle");
      const emptyState = document.getElementById("emptyState");
      const content = document.getElementById("content");
      const documentBase = document.getElementById("document-base");
      const filePath = document.getElementById("filePath");
      const wordCount = document.getElementById("wordCount");
      const lineCount = document.getElementById("lineCount");
      const aboutOverlay = document.getElementById("aboutOverlay");
      const aboutClose = document.getElementById("aboutClose");
      const aboutVersion = document.getElementById("aboutVersion");
      const aboutAuthor = document.getElementById("aboutAuthor");
      const aboutBuild = document.getElementById("aboutBuild");
      const aboutGithub = document.getElementById("aboutGithub");
      const aboutCopy = document.getElementById("aboutCopy");

      const hideAbout = () => {
        aboutOverlay.classList.add("hidden");
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

      document.addEventListener("keydown", (event) => {
        if (event.key === "Escape" && !aboutOverlay.classList.contains("hidden")) {
          hideAbout();
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

      window.renderInfo = (message) => {
        status.textContent = message;
        bottomStatus.innerHTML = `<strong>Status</strong> ${message}`;
      };

      window.renderError = (message) => {
        status.textContent = message;
        bottomStatus.innerHTML = `<strong>Status</strong> ${message}`;
        documentTitle.textContent = "Preview";
        documentSubtitle.textContent = "Use File > Open, Command+O, or drag a Markdown file into the window.";
        filePath.textContent = "Path: No file opened";
        wordCount.innerHTML = "<strong>Words</strong> 0";
        lineCount.innerHTML = "<strong>Lines</strong> 0";
        emptyState.classList.remove("hidden");
        content.classList.add("hidden");
        content.innerHTML = "";
      };

      window.renderDocument = (payload) => {
        document.title = `${payload.title} - MarkHola`;
        documentTitle.textContent = payload.title;
        documentSubtitle.textContent = payload.file_name;
        filePath.textContent = `Path: ${payload.file_path}`;
        wordCount.innerHTML = `<strong>Words</strong> ${payload.word_count}`;
        lineCount.innerHTML = `<strong>Lines</strong> ${payload.line_count}`;
        status.textContent = "Preview loaded.";
        bottomStatus.innerHTML = "<strong>Status</strong> Preview loaded.";
        documentBase.setAttribute("href", payload.base_url);
        content.innerHTML = payload.html;
        content.classList.remove("hidden");
        emptyState.classList.add("hidden");
      };

      window.showAbout = (payload) => {
        aboutVersion.textContent = payload.version;
        aboutAuthor.textContent = payload.author;
        aboutBuild.textContent = `${payload.buildPlatform} / ${payload.buildTarget}`;
        aboutGithub.textContent = payload.githubUrl;
        aboutGithub.setAttribute("href", payload.githubUrl);
        aboutCopy.textContent = "Copy";
        aboutOverlay.classList.remove("hidden");
      };
    </script>
  </body>
</html>
"##
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::ensure_supported_markdown_path;

    #[test]
    fn accepts_markdown_extensions() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir();

        let md_path = temp_dir.join(format!("markhola-{stamp}.md"));
        fs::write(&md_path, "# hello").unwrap();
        assert!(ensure_supported_markdown_path(&md_path).is_ok());
        let _ = fs::remove_file(&md_path);

        let markdown_path = temp_dir.join(format!("markhola-{stamp}.markdown"));
        fs::write(&markdown_path, "# hello").unwrap();
        assert!(ensure_supported_markdown_path(&markdown_path).is_ok());
        let _ = fs::remove_file(&markdown_path);
    }

    #[test]
    fn rejects_non_markdown_extensions() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_path = std::env::temp_dir().join(format!("markhola-{stamp}.txt"));
        fs::write(&temp_path, "hello").unwrap();

        let error = ensure_supported_markdown_path(&temp_path).unwrap_err();
        assert!(error.contains("Only .md and .markdown files are supported"));
        let _ = fs::remove_file(&temp_path);
    }
}

#[cfg(target_os = "macos")]
mod macos_menu {
    use std::error::Error;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{DefinedClass, MainThreadOnly, define_class, sel};
    use objc2_app_kit::{
        NSApp, NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem,
    };
    use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, ns_string};

    use super::UserEvent;
    use tao::event_loop::EventLoopProxy;

    #[derive(Debug)]
    struct MenuTargetIvars {
        proxy: EventLoopProxy<UserEvent>,
    }

    define_class!(
        #[unsafe(super = NSObject)]
        #[thread_kind = MainThreadOnly]
        #[ivars = MenuTargetIvars]
        struct MenuTarget;

        unsafe impl NSObjectProtocol for MenuTarget {}

        impl MenuTarget {
            #[unsafe(method(openMenuDocument:))]
            fn open_menu_document(&self, _sender: Option<&AnyObject>) {
                let _ = self.ivars().proxy.send_event(UserEvent::OpenFile);
            }

            #[unsafe(method(showAboutPanel:))]
            fn show_about_panel(&self, _sender: Option<&AnyObject>) {
                let _ = self.ivars().proxy.send_event(UserEvent::ShowAbout);
            }

            #[unsafe(method(exitApplication:))]
            fn exit_application(&self, _sender: Option<&AnyObject>) {
                let _ = self.ivars().proxy.send_event(UserEvent::Exit);
            }
        }
    );

    impl MenuTarget {
        fn new(
            mtm: MainThreadMarker,
            proxy: EventLoopProxy<UserEvent>,
        ) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(MenuTargetIvars { proxy });
            unsafe { objc2::msg_send![super(this), init] }
        }
    }

    pub fn install(proxy: &EventLoopProxy<UserEvent>) -> Result<(), Box<dyn Error>> {
        let mtm = MainThreadMarker::new().ok_or("menu setup must run on main thread")?;
        let app = NSApplication::sharedApplication(mtm);
        let target = MenuTarget::new(mtm, proxy.clone());

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
        unsafe { about_item.setTarget(Some((&*target).as_ref())) };
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
        unsafe { quit_item.setTarget(Some((&*target).as_ref())) };
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
        unsafe { open_item.setTarget(Some((&*target).as_ref())) };
        open_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        file_menu.addItem(&open_item);
        file_menu.addItem(&NSMenuItem::separatorItem(mtm));
        let exit_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Exit"),
                Some(sel!(exitApplication:)),
                ns_string!("q"),
            )
        };
        unsafe { exit_item.setTarget(Some((&*target).as_ref())) };
        exit_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
        file_menu.addItem(&exit_item);
        file_menu_item.setSubmenu(Some(&file_menu));

        app.setMainMenu(Some(&main_menu));

        let _ = NSApp(mtm);
        std::mem::forget(target);
        Ok(())
    }
}
