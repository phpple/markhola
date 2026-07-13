use std::borrow::Cow;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use lopdf::{Dictionary, Document as LoDocument, Object};
use rfd::FileDialog;
use serde::Deserialize;
use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event_loop::EventLoopBuilder;
use tao::window::{Window, WindowBuilder};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
};
use wry::{PageLoadEvent, Rect, WebView, WebViewBuilder};

use crate::app::log_event;
use crate::document::ActiveDocument;
use crate::file_io;
use crate::markdown;
use crate::render_assets;

pub(crate) const APP_NAME: &str = "MarkHola";
pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const EXPORT_WEBVIEW_WIDTH: f64 = 816.0;
pub(crate) const EXPORT_WEBVIEW_HEIGHT: f64 = 1120.0;
pub(crate) const EXPORT_TIMEOUT: Duration = Duration::from_secs(60);
pub(crate) const FAST_EXPORT_BUDGET: Duration = Duration::from_secs(3);
const EXPORT_SCHEME: &str = "markhola-export";
const EXPORT_URL: &str = "markhola-export://export/index.html";
const EXPORT_FOOTER_TEXT: &str = "Exported by MarkHola v__APP_VERSION__";
const EXPORT_PRINT_CSS: &str = r#"
html, body {
  margin: 0;
  padding: 0;
  background: #ffffff;
}

body {
  color: #111111;
}

.export-page {
  box-sizing: border-box;
  width: 100%;
  max-width: 816px;
  margin: 0 auto;
  padding: 48px 56px 40px;
  background: #ffffff;
}

.export-footer {
  box-sizing: border-box;
  width: 100%;
  max-width: 816px;
  margin: 0 auto;
  padding: 0 56px 72px;
  text-align: right;
  font-size: 10px;
  line-height: 1.4;
  color: #f8f7f4;
  user-select: none;
}

.app,
.tabs-bar,
.preview-header,
.bottom-bar,
.about-overlay,
.editor-pane,
.empty-state {
  display: none !important;
}
"#;

const EXPORT_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <base href="__BASE_URL__" />
    <title>__TITLE__</title>
    <style>__APP_THEME__</style>
    <style>__EXPORT_PRINT_CSS__</style>
  </head>
  <body>
    <article class="markdown-body export-page" id="content">__DOCUMENT_HTML__</article>
    <footer class="export-footer">__EXPORT_FOOTER__</footer>

    <script>
      window.MathJax = {
        startup: { typeset: false },
        svg: { fontCache: "none" }
      };
    </script>
    <script>__MERMAID_RUNTIME__</script>
    <script>__MATHJAX_RUNTIME__</script>
    <script>
      const content = document.getElementById("content");
      let mermaidInitialized = false;
      let mathJaxReadyPromise = null;
      window.markholaExportProgress = {
        stage: "boot",
        timings: [],
        imageCount: document.images.length
      };

      const markStage = (stage, extra = {}) => {
        window.markholaExportProgress = {
          ...window.markholaExportProgress,
          stage,
          updatedAt: Date.now(),
          ...extra
        };
      };

      const recordTiming = (label, startedAt) => {
        const durationMs = Date.now() - startedAt;
        const timings = Array.isArray(window.markholaExportProgress.timings)
          ? window.markholaExportProgress.timings
          : [];
        timings.push({ label, durationMs });
        window.markholaExportProgress.timings = timings;
      };

      const withTiming = async (label, fn) => {
        const startedAt = Date.now();
        try {
          return await fn();
        } finally {
          recordTiming(label, startedAt);
        }
      };

      const withTimeout = async (label, promise, timeoutMs) => {
        let timeoutId = null;
        try {
          return await Promise.race([
            promise,
            new Promise((resolve) => {
              timeoutId = setTimeout(() => {
                markStage(`${label}-timeout`, { timeoutMs });
                resolve({ timedOut: true });
              }, timeoutMs);
            })
          ]);
        } finally {
          if (timeoutId !== null) {
            clearTimeout(timeoutId);
          }
        }
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
              `markhola-export-mermaid-${index}-${Date.now()}`,
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

      const waitForImages = async (label) => {
        const pending = Array.from(document.images).filter((image) => !image.complete);
        markStage(label, { pendingImages: pending.length });
        const result = await withTimeout(
          label,
          Promise.all(
            pending.map(
              (image) =>
                new Promise((resolve) => {
                  const done = () => resolve();
                  image.addEventListener("load", done, { once: true });
                  image.addEventListener("error", done, { once: true });
                })
            )
          ),
          8000
        );

        if (result && result.timedOut) {
          markStage(`${label}-continued`, { pendingImages: pending.length });
        }
      };

      const waitForNextPaint = async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      };

      window.markholaMeasureExportPage = () => {
        const root = document.documentElement;
        const body = document.body;
        return {
          width: Math.max(root.scrollWidth, root.clientWidth, body.scrollWidth, body.clientWidth),
          height: Math.max(root.scrollHeight, root.clientHeight, body.scrollHeight, body.clientHeight)
        };
      };

      window.markholaPreparePdf = async () => {
        markStage("prepare-start");
        await withTiming("wait-initial-images", () => waitForImages("wait-initial-images"));
        markStage("render-mermaid");
        await withTiming("render-mermaid", renderMermaidDiagrams);
        markStage("render-math");
        await withTiming("render-math", renderMathExpressions);
        await withTiming("wait-post-render-images", () => waitForImages("wait-post-render-images"));
        markStage("wait-next-paint");
        await withTiming("wait-next-paint", waitForNextPaint);
        const measurement = window.markholaMeasureExportPage();
        markStage("prepare-complete", measurement);
        return measurement;
      };

      window.markholaPreparePdfFast = async () => {
        markStage("fast-prepare-start");
        const measurement = window.markholaMeasureExportPage();
        markStage("fast-prepare-complete", measurement);
        return measurement;
      };

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
    </script>
  </body>
</html>
"#;

const PREPARE_EXPORT_JS: &str = r#"
window.markholaPreparePdf()
  .then((measurement) => window.ipc.postMessage(JSON.stringify({ kind: "prepared", measurement })))
  .catch((error) =>
    window.ipc.postMessage(
      JSON.stringify({
        kind: "prepare-error",
        message: String((error && error.message) || error || "Unknown export error")
      })
    )
  );
"#;

const PREPARE_EXPORT_FAST_JS: &str = r#"
window.markholaPreparePdfFast()
  .then((measurement) => window.ipc.postMessage(JSON.stringify({ kind: "prepared", measurement })))
  .catch((error) =>
    window.ipc.postMessage(
      JSON.stringify({
        kind: "prepare-error",
        message: String((error && error.message) || error || "Unknown export error")
      })
    )
  );
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdfExportOutcome {
    Exported(PathBuf),
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExportPreparationMode {
    Fast,
    Full,
}

#[derive(Debug, Deserialize)]
struct ExportMeasurement {
    width: f64,
    height: f64,
}

#[derive(Debug, Deserialize)]
struct ExportIpcMessage {
    kind: String,
    measurement: Option<ExportMeasurement>,
    message: Option<String>,
}

enum ExportSignal {
    PageLoaded,
    Prepared(ExportMeasurement),
    Failed(String),
}

pub(crate) struct PreparedPrintWebView {
    pub webview: WebView,
    pub preparation_mode: ExportPreparationMode,
}

pub fn export_document(window: &Window, document: &ActiveDocument) -> Result<PdfExportOutcome, String> {
    log_event(
        "pdf_export.begin",
        None,
        "starting PDF export",
        format!(
            "path={} dirty={} mode={:?}",
            document.file_path().display(),
            document.is_dirty(),
            document.mode()
        ),
    );
    let Some(export_path) = choose_export_path(document) else {
        log_event(
            "pdf_export.cancelled",
            None,
            "PDF export cancelled in save dialog",
            "",
        );
        return Ok(PdfExportOutcome::Cancelled);
    };
    let rendered_document_html = markdown::render_html(document.markdown());
    let html = build_export_html(document, &rendered_document_html);
    let preparation_mode = export_preparation_mode(&rendered_document_html);
    let pdf_data = render_pdf_data(window, document, &html, preparation_mode)?;
    let pdf_data = apply_pdf_metadata(document, pdf_data)?;
    fs::write(&export_path, pdf_data)
        .map_err(|error| format!("Failed to write PDF file: {error}"))?;
    log_event(
        "pdf_export.end",
        None,
        "finished PDF export",
        format!("output={}", export_path.display()),
    );
    Ok(PdfExportOutcome::Exported(export_path))
}

pub fn export_markdown_file_to_path(input_path: &Path, output_path: &Path) -> Result<(), String> {
    let input_path = std::fs::canonicalize(input_path)
        .map_err(|error| format!("Failed to canonicalize input path: {error}"))?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create export directory: {error}"))?;
    }

    let markdown = file_io::load_markdown(&input_path)?;
    let base_url = file_io::directory_base_url(&input_path)?;
    let document = ActiveDocument::open_with_id(1, input_path.clone(), markdown, base_url);
    let rendered_document_html = markdown::render_html(document.markdown());
    let html = build_export_html(&document, &rendered_document_html);
    let preparation_mode = export_preparation_mode(&rendered_document_html);

    let event_loop = EventLoopBuilder::<()>::new().build();
    let host_window = WindowBuilder::new()
        .with_visible(false)
        .with_title("MarkHola PDF Export")
        .with_inner_size(LogicalSize::new(32.0, 32.0))
        .build(&event_loop)
        .map_err(|error| format!("Failed to create temporary export window: {error}"))?;

    let pdf_data = render_pdf_data(&host_window, &document, &html, preparation_mode)?;
    let pdf_data = apply_pdf_metadata(&document, pdf_data)?;
    fs::write(output_path, pdf_data).map_err(|error| format!("Failed to write PDF file: {error}"))
}

fn choose_export_path(document: &ActiveDocument) -> Option<PathBuf> {
    let suggested_name = document
        .suggested_pdf_export_path()
        .file_name()?
        .to_string_lossy()
        .into_owned();

    FileDialog::new()
        .add_filter("PDF", &["pdf"])
        .set_title("Export PDF")
        .set_directory(
            document
                .file_path()
                .parent()
                .unwrap_or(document.file_path()),
        )
        .set_file_name(suggested_name)
        .save_file()
}

fn render_pdf_data(
    host_window: &Window,
    document: &ActiveDocument,
    html: &str,
    preparation_mode: ExportPreparationMode,
) -> Result<Vec<u8>, String> {
    let started_at = Instant::now();
    let webview = prepare_export_webview(host_window, html, preparation_mode, started_at)?;
    let temp_output_path = temporary_pdf_output_path(document);
    webview
        .print_to_pdf(&temp_output_path)
        .map_err(|error| format!("Failed to render PDF: {error}"))?;
    let pdf_data = fs::read(&temp_output_path)
        .map_err(|error| format!("Failed to read temporary PDF file: {error}"))?;
    let _ = fs::remove_file(&temp_output_path);
    log_event(
        "pdf_export.render.end",
        None,
        "completed render_pdf_data",
        format!("elapsed_ms={}", started_at.elapsed().as_millis()),
    );
    Ok(pdf_data)
}

pub(crate) fn prepare_print_webview(
    host_window: &Window,
    document: &ActiveDocument,
) -> Result<PreparedPrintWebView, String> {
    let rendered_document_html = markdown::render_html(document.markdown());
    let html = build_export_html(document, &rendered_document_html);
    let preparation_mode = export_preparation_mode(&rendered_document_html);
    let webview = prepare_export_webview(host_window, &html, preparation_mode, Instant::now())?;

    Ok(PreparedPrintWebView {
        webview,
        preparation_mode,
    })
}

fn prepare_export_webview(
    host_window: &Window,
    html: &str,
    preparation_mode: ExportPreparationMode,
    started_at: Instant,
) -> Result<WebView, String> {
    let (signal_tx, signal_rx) = mpsc::channel();
    let html_bytes = html.as_bytes().to_vec();
    let bounds = Rect {
        position: LogicalPosition::new(0.0, 0.0).into(),
        size: LogicalSize::new(EXPORT_WEBVIEW_WIDTH, EXPORT_WEBVIEW_HEIGHT).into(),
    };

    let webview = WebViewBuilder::new()
        .with_visible(false)
        .with_bounds(bounds)
        .with_custom_protocol(EXPORT_SCHEME.into(), move |_id, _request| {
            wry::http::Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(Cow::Owned(html_bytes.clone()))
                .unwrap()
        })
        .with_url(EXPORT_URL)
        .with_ipc_handler({
            let signal_tx = signal_tx.clone();
            move |request| {
                let message = parse_export_message(request.body());
                let _ = signal_tx.send(message);
            }
        })
        .with_on_page_load_handler({
            let signal_tx = signal_tx.clone();
            move |event, _url| {
                if matches!(event, PageLoadEvent::Finished) {
                    let _ = signal_tx.send(ExportSignal::PageLoaded);
                }
            }
        })
        .build_as_child(host_window)
        .map_err(|error| format!("Failed to create export WebView: {error}"))?;

    wait_for_page_loaded(&signal_rx)?;
    run_prepare_script(&webview, preparation_mode)?;
    let measurement = wait_for_prepared(&signal_rx, timeout_for_mode(preparation_mode, started_at)?)?;
    let final_bounds = Rect {
        position: LogicalPosition::new(0.0, 0.0).into(),
        size: LogicalSize::new(
            measurement.width.ceil().max(EXPORT_WEBVIEW_WIDTH),
            measurement.height.ceil().max(EXPORT_WEBVIEW_HEIGHT),
        )
        .into(),
    };
    webview
        .set_bounds(final_bounds)
        .map_err(|error| format!("Failed to resize export WebView: {error}"))?;
    Ok(webview)
}

fn run_prepare_script(webview: &WebView, preparation_mode: ExportPreparationMode) -> Result<(), String> {
    let script = match preparation_mode {
        ExportPreparationMode::Fast => PREPARE_EXPORT_FAST_JS,
        ExportPreparationMode::Full => PREPARE_EXPORT_JS,
    };
    webview
        .evaluate_script(script)
        .map_err(|error| format!("Failed to start PDF preparation script: {error}"))
}

fn wait_for_page_loaded(rx: &Receiver<ExportSignal>) -> Result<(), String> {
    wait_for_signal(rx, Duration::from_secs(15), |signal| match signal {
        ExportSignal::PageLoaded => Some(Ok(())),
        ExportSignal::Failed(message) => Some(Err(message)),
        ExportSignal::Prepared(_) => None,
    })
}

fn wait_for_prepared(
    rx: &Receiver<ExportSignal>,
    timeout: Duration,
) -> Result<ExportMeasurement, String> {
    wait_for_signal(rx, timeout, |signal| match signal {
        ExportSignal::Prepared(measurement) => Some(Ok(measurement)),
        ExportSignal::Failed(message) => Some(Err(message)),
        ExportSignal::PageLoaded => None,
    })
}

fn wait_for_signal<T>(
    rx: &Receiver<ExportSignal>,
    timeout: Duration,
    mut handle: impl FnMut(ExportSignal) -> Option<Result<T, String>>,
) -> Result<T, String> {
    let started_at = Instant::now();
    loop {
        match rx.try_recv() {
            Ok(signal) => {
                if let Some(result) = handle(signal) {
                    return result;
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                return Err("Export WebView channel disconnected unexpectedly.".to_string())
            }
        }

        if started_at.elapsed() > timeout {
            return Err("Timed out while preparing the export page.".to_string());
        }

        pump_windows_messages()?;
        thread::sleep(Duration::from_millis(10));
    }
}

fn pump_windows_messages() -> Result<(), String> {
    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

fn parse_export_message(payload: &str) -> ExportSignal {
    match serde_json::from_str::<ExportIpcMessage>(payload) {
        Ok(message) if message.kind == "prepared" => match message.measurement {
            Some(measurement) => ExportSignal::Prepared(measurement),
            None => ExportSignal::Failed(
                "PDF preparation finished without measurement data.".to_string(),
            ),
        },
        Ok(message) if message.kind == "prepare-error" => ExportSignal::Failed(
            message
                .message
                .unwrap_or_else(|| "Unknown export preparation error.".to_string()),
        ),
        Ok(message) => ExportSignal::Failed(format!(
            "Unexpected export IPC message: {}",
            message.kind
        )),
        Err(error) => ExportSignal::Failed(format!(
            "Failed to decode export IPC payload: {error}"
        )),
    }
}

fn apply_pdf_metadata(
    document: &ActiveDocument,
    pdf_data: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let mut pdf = LoDocument::load_mem(&pdf_data)
        .map_err(|error| format!("Failed to load generated PDF for metadata injection: {error}"))?;

    let mut info = Dictionary::new();
    info.set("Title", Object::string_literal(document.file_name()));
    info.set("Creator", Object::string_literal(APP_NAME));
    info.set(
        "Producer",
        Object::string_literal(format!("{APP_NAME} v{APP_VERSION}")),
    );
    info.set(
        "Subject",
        Object::string_literal(format!("Exported by {APP_NAME} v{APP_VERSION}")),
    );

    let info_id = pdf.add_object(info);
    pdf.trailer.set("Info", info_id);

    let mut output = Vec::new();
    pdf.save_to(&mut Cursor::new(&mut output))
        .map_err(|error| format!("Failed to save generated PDF metadata: {error}"))?;
    Ok(output)
}

fn export_preparation_mode(html: &str) -> ExportPreparationMode {
    let requires_full_prepare = html.contains("mermaid-block")
        || html.contains("math-block")
        || html.contains("math math-inline")
        || html.contains("math math-display")
        || html.contains("<img");

    if requires_full_prepare {
        ExportPreparationMode::Full
    } else {
        ExportPreparationMode::Fast
    }
}

fn build_export_html(document: &ActiveDocument, rendered_html: &str) -> String {
    EXPORT_HTML_TEMPLATE
        .replace("__TITLE__", document.file_name())
        .replace("__BASE_URL__", document.base_url())
        .replace("__EXPORT_FOOTER__", &export_footer_text())
        .replace(
            "__APP_THEME__",
            &render_assets::load_app_theme_css_for_inline_style(),
        )
        .replace("__EXPORT_PRINT_CSS__", EXPORT_PRINT_CSS)
        .replace(
            "__MERMAID_RUNTIME__",
            &render_assets::mermaid_runtime_for_inline_script(),
        )
        .replace(
            "__MATHJAX_RUNTIME__",
            &render_assets::mathjax_runtime_for_inline_script(),
        )
        .replace("__DOCUMENT_HTML__", rendered_html)
}

fn export_footer_text() -> String {
    EXPORT_FOOTER_TEXT.replace("__APP_VERSION__", APP_VERSION)
}

fn timeout_for_mode(
    mode: ExportPreparationMode,
    started_at: Instant,
) -> Result<Duration, String> {
    match mode {
        ExportPreparationMode::Full => Ok(EXPORT_TIMEOUT),
        ExportPreparationMode::Fast => {
            let elapsed = started_at.elapsed();
            if elapsed >= FAST_EXPORT_BUDGET {
                Err("Timed out while preparing the export page.".to_string())
            } else {
                Ok(FAST_EXPORT_BUDGET - elapsed)
            }
        }
    }
}

fn temporary_pdf_output_path(document: &ActiveDocument) -> PathBuf {
    let stem = document
        .file_path()
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("document");
    std::env::temp_dir().join(format!(
        "markhola-export-{}-{stem}.pdf",
        std::process::id()
    ))
}
