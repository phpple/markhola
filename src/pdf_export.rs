use std::cell::RefCell;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use block2::RcBlock;
use lopdf::{Dictionary, Document as LoDocument, Object};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::NSApp;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    NSData, NSDefaultRunLoopMode, NSDate, NSError, NSRunLoop, NSString, NSURL,
};
use objc2_web_kit::{
    WKContentWorld, WKPDFConfiguration, WKWebView, WKWebViewConfiguration,
};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use serde::Deserialize;
use serde_json::Value;

use crate::app::log_event;
use crate::document::ActiveDocument;
use crate::file_io;
use crate::markdown;
use crate::render_assets;

const APP_NAME: &str = "MarkHola";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const EXPORT_WEBVIEW_WIDTH: f64 = 816.0;
const EXPORT_WEBVIEW_HEIGHT: f64 = 1120.0;
const EXPORT_TIMEOUT: Duration = Duration::from_secs(60);
const FAST_EXPORT_BUDGET: Duration = Duration::from_secs(3);
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
const measurement = await window.markholaPreparePdf();
return JSON.stringify(measurement);
"#;

const PREPARE_EXPORT_FAST_JS: &str = r#"
const measurement = await window.markholaPreparePdfFast();
return JSON.stringify(measurement);
"#;

#[derive(Debug, Eq, PartialEq)]
pub enum PdfExportOutcome {
    Cancelled,
    Exported(PathBuf),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExportPreparationMode {
    Fast,
    Full,
}

#[derive(Deserialize)]
struct ExportMeasurement {
    width: f64,
    height: f64,
}

pub fn export_document(document: &ActiveDocument) -> Result<PdfExportOutcome, String> {
    log_event(
        "pdf_export.begin",
        None,
        "starting PDF export",
        format!("path={} dirty={} mode={:?}", document.file_path().display(), document.is_dirty(), document.mode()),
    );
    let Some(export_path) = choose_export_path(document) else {
        log_event("pdf_export.cancelled", None, "PDF export cancelled in save dialog", "");
        return Ok(PdfExportOutcome::Cancelled);
    };
    if export_path.exists() && !confirm_overwrite(&export_path) {
        log_event(
            "pdf_export.cancelled",
            None,
            "PDF export cancelled by overwrite confirmation",
            format!("output={}", export_path.display()),
        );
        return Ok(PdfExportOutcome::Cancelled);
    }

    let rendered_document_html = markdown::render_html(document.markdown());
    let html = build_export_html(document, &rendered_document_html);
    let preparation_mode = export_preparation_mode(&rendered_document_html);
    log_event(
        "pdf_export.html",
        None,
        "built PDF export HTML",
        format!("bytes={} mode={preparation_mode:?}", html.len()),
    );
    write_export(document, &export_path, &html, preparation_mode)?;
    Ok(PdfExportOutcome::Exported(export_path))
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
        .set_directory(document.file_path().parent().unwrap_or(document.file_path()))
        .set_file_name(suggested_name)
        .save_file()
}

fn confirm_overwrite(path: &PathBuf) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("this file");
    log_event(
        "pdf_export.overwrite_prompt",
        None,
        "asking whether to overwrite existing PDF",
        format!("output={}", path.display()),
    );

    let result = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title("Replace existing PDF")
        .set_description(format!("{file_name} already exists. Replace it with the new export?"))
        .set_buttons(MessageButtons::YesNo)
        .show();

    let should_overwrite = matches!(result, MessageDialogResult::Yes);
    log_event(
        "pdf_export.overwrite_prompt.end",
        None,
        "overwrite decision captured",
        format!("output={} overwrite={should_overwrite}", path.display()),
    );
    should_overwrite
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
    write_export(&document, output_path, &html, preparation_mode)
}

fn write_export(
    document: &ActiveDocument,
    export_path: &Path,
    html: &str,
    preparation_mode: ExportPreparationMode,
) -> Result<(), String> {
    let pdf_data = render_pdf_data(document, html, preparation_mode)?;
    let pdf_data = apply_pdf_metadata(document, pdf_data)?;
    log_event(
        "pdf_export.data",
        None,
        "generated PDF data",
        format!("bytes={} output={}", pdf_data.len(), export_path.display()),
    );
    fs::write(export_path, pdf_data)
        .map_err(|error| format!("Failed to write PDF file: {error}"))?;

    log_event(
        "pdf_export.end",
        None,
        "finished PDF export",
        format!("output={}", export_path.display()),
    );
    Ok(())
}

fn apply_pdf_metadata(document: &ActiveDocument, pdf_data: Vec<u8>) -> Result<Vec<u8>, String> {
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

fn render_pdf_data(
    document: &ActiveDocument,
    html: &str,
    preparation_mode: ExportPreparationMode,
) -> Result<Vec<u8>, String> {
    let started_at = Instant::now();
    let mtm = MainThreadMarker::new().ok_or("PDF export must run on the main thread.")?;

    let configuration = unsafe { WKWebViewConfiguration::new(mtm) };
    let webview = unsafe {
        WKWebView::initWithFrame_configuration(
            WKWebView::alloc(mtm),
            CGRect::new(
                CGPoint::ZERO,
                CGSize::new(EXPORT_WEBVIEW_WIDTH, EXPORT_WEBVIEW_HEIGHT),
            ),
            &configuration,
        )
    };

    let html = NSString::from_str(html);
    let base_url = NSURL::URLWithString(&NSString::from_str(document.base_url()))
        .or_else(|| NSURL::from_directory_path(document.file_path().parent().unwrap_or(document.file_path())))
        .ok_or("Failed to resolve the document base URL for PDF export.")?;

    let _app = NSApp(mtm);
    unsafe {
        webview.loadHTMLString_baseURL(&html, Some(&base_url));
    }
    log_event("pdf_export.load.begin", None, "loading export HTML into WKWebView", "");
    let load_timeout = timeout_for_mode(preparation_mode, started_at, "load")?;

    wait_for(
        || !unsafe { webview.isLoading() },
        "Timed out while loading the export page.",
        load_timeout,
    )?;
    log_event(
        "pdf_export.load.end",
        None,
        "finished loading export HTML",
        format!("elapsed_ms={}", started_at.elapsed().as_millis()),
    );

    let measurement = match preparation_mode {
        ExportPreparationMode::Fast => prepare_export_page_fast(
            &webview,
            mtm,
            timeout_for_mode(preparation_mode, started_at, "prepare-fast")?,
        )?,
        ExportPreparationMode::Full => prepare_export_page(&webview, mtm)?,
    };
    log_event(
        "pdf_export.prepare.measurement",
        None,
        "prepared export page measurement",
        format!("width={} height={}", measurement.width, measurement.height),
    );
    let pdf_configuration = unsafe { WKPDFConfiguration::new(mtm) };
    unsafe {
        pdf_configuration.setAllowTransparentBackground(false);
    }

    let pdf_data = create_pdf(
        &webview,
        &pdf_configuration,
        timeout_for_mode(preparation_mode, started_at, "create-pdf")?,
    )?;
    log_event(
        "pdf_export.render.end",
        None,
        "completed render_pdf_data",
        format!("elapsed_ms={}", started_at.elapsed().as_millis()),
    );
    Ok(pdf_data)
}

fn prepare_export_page_fast(
    webview: &WKWebView,
    mtm: MainThreadMarker,
    timeout: Duration,
) -> Result<ExportMeasurement, String> {
    log_event("pdf_export.prepare_fast.begin", None, "preparing export page with fast path", "");
    let result = run_prepare_script(
        webview,
        mtm,
        PREPARE_EXPORT_FAST_JS,
        timeout,
        "Timed out while preparing the export page.",
        "pdf_export.prepare_fast.timeout",
    )?;
    log_event(
        "pdf_export.prepare_fast.end",
        None,
        "prepared export page with fast path",
        format!("width={} height={}", result.width, result.height),
    );
    Ok(result)
}

fn prepare_export_page(webview: &WKWebView, mtm: MainThreadMarker) -> Result<ExportMeasurement, String> {
    log_event("pdf_export.prepare.begin", None, "preparing export page", "");
    let measurement = run_prepare_script(
        webview,
        mtm,
        PREPARE_EXPORT_JS,
        EXPORT_TIMEOUT,
        "Timed out while preparing the export page.",
        "pdf_export.prepare.timeout",
    )?;
    log_event(
        "pdf_export.prepare.end",
        None,
        "prepared export page",
        format!("width={} height={}", measurement.width, measurement.height),
    );

    Ok(measurement)
}

fn run_prepare_script(
    webview: &WKWebView,
    mtm: MainThreadMarker,
    script: &str,
    timeout: Duration,
    timeout_message: &str,
    timeout_stage: &str,
) -> Result<ExportMeasurement, String> {
    let outcome = Rc::new(RefCell::new(None));
    let outcome_for_block = Rc::clone(&outcome);
    let completion = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
        let export_result = if let Some(error) = unsafe { Retained::retain(error) } {
            Err(error.localizedDescription().to_string())
        } else if let Some(result) = unsafe { Retained::retain(result) } {
            match result.downcast::<NSString>() {
                Ok(string) => Ok(string.to_string()),
                Err(_) => Err("Export preparation returned an unexpected JavaScript result.".to_string()),
            }
        } else {
            Err("Export preparation did not return a JavaScript result.".to_string())
        };

        *outcome_for_block.borrow_mut() = Some(export_result);
    });

    let world = unsafe { WKContentWorld::pageWorld(mtm) };
    unsafe {
        webview.callAsyncJavaScript_arguments_inFrame_inContentWorld_completionHandler(
            &NSString::from_str(script),
            None,
            None,
            &world,
            Some(&completion),
        );
    }

    wait_for(
        || outcome.borrow().is_some(),
        timeout_message,
        timeout,
    )
    .map_err(|message| {
        let progress = export_progress_snapshot(webview, mtm)
            .unwrap_or_else(|error| format!("progress-unavailable:{error}"));
        log_event(
            timeout_stage,
            None,
            "timed out while preparing export page",
            progress.as_str(),
        );
        format!("{message} Last progress: {progress}")
    })?;

    let raw = outcome
        .borrow_mut()
        .take()
        .ok_or("Export preparation finished without a result.")??;
    let measurement: ExportMeasurement = serde_json::from_str(&raw)
        .map_err(|error| format!("Failed to decode export page size: {error}"))?;

    Ok(ExportMeasurement {
        width: measurement.width.ceil().max(EXPORT_WEBVIEW_WIDTH),
        height: measurement.height.ceil().max(EXPORT_WEBVIEW_HEIGHT),
    })
}

fn create_pdf(
    webview: &WKWebView,
    configuration: &WKPDFConfiguration,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    log_event("pdf_export.create_pdf.begin", None, "creating PDF from WKWebView", "");
    let outcome = Rc::new(RefCell::new(None));
    let outcome_for_block = Rc::clone(&outcome);
    let completion = RcBlock::new(move |data: *mut NSData, error: *mut NSError| {
        let export_result = if let Some(error) = unsafe { Retained::retain(error) } {
            Err(error.localizedDescription().to_string())
        } else if let Some(data) = unsafe { Retained::retain(data) } {
            Ok(data.to_vec())
        } else {
            Err("WKWebView returned no PDF data.".to_string())
        };

        *outcome_for_block.borrow_mut() = Some(export_result);
    });

    unsafe {
        webview.createPDFWithConfiguration_completionHandler(Some(configuration), &completion);
    }

    wait_for(
        || outcome.borrow().is_some(),
        "Timed out while creating the PDF file.",
        timeout,
    )?;
    let pdf = outcome
        .borrow_mut()
        .take()
        .ok_or("PDF export completed without a result.")?;
    if let Ok(bytes) = &pdf {
        log_event(
            "pdf_export.create_pdf.end",
            None,
            "created PDF from WKWebView",
            format!("bytes={}", bytes.len()),
        );
    }
    pdf
}

fn export_progress_snapshot(webview: &WKWebView, mtm: MainThreadMarker) -> Result<String, String> {
    let _ = mtm;
    let outcome = Rc::new(RefCell::new(None));
    let outcome_for_block = Rc::clone(&outcome);
    let completion = RcBlock::new(move |result: *mut AnyObject, error: *mut NSError| {
        let snapshot = if let Some(error) = unsafe { Retained::retain(error) } {
            Err(error.localizedDescription().to_string())
        } else if let Some(result) = unsafe { Retained::retain(result) } {
            match result.downcast::<NSString>() {
                Ok(string) => Ok(string.to_string()),
                Err(_) => Err("diagnostic result type mismatch".to_string()),
            }
        } else {
            Ok("null".to_string())
        };

        *outcome_for_block.borrow_mut() = Some(snapshot);
    });

    unsafe {
        webview.evaluateJavaScript_completionHandler(
            &NSString::from_str("JSON.stringify(window.markholaExportProgress || null)"),
            Some(&completion),
        );
    }

    wait_for(
        || outcome.borrow().is_some(),
        "Timed out while reading export progress.",
        Duration::from_secs(2),
    )?;

    let raw = outcome
        .borrow_mut()
        .take()
        .ok_or("Missing export progress result.")??;

    let parsed = serde_json::from_str::<Value>(&raw)
        .map(|value| value.to_string())
        .unwrap_or(raw);
    Ok(parsed)
}

fn wait_for(
    mut is_ready: impl FnMut() -> bool,
    timeout_message: &str,
    timeout: Duration,
) -> Result<(), String> {
    let run_loop = NSRunLoop::currentRunLoop();
    let started_at = Instant::now();

    while !is_ready() {
        if started_at.elapsed() > timeout {
            return Err(timeout_message.to_string());
        }

        let next_slice = NSDate::dateWithTimeIntervalSinceNow(0.05);
        unsafe {
            run_loop.runMode_beforeDate(NSDefaultRunLoopMode, &next_slice);
        }
    }

    Ok(())
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

fn timeout_for_mode(
    mode: ExportPreparationMode,
    started_at: Instant,
    stage: &str,
) -> Result<Duration, String> {
    match mode {
        ExportPreparationMode::Full => Ok(EXPORT_TIMEOUT),
        ExportPreparationMode::Fast => {
            let elapsed = started_at.elapsed();
            if elapsed >= FAST_EXPORT_BUDGET {
                log_event(
                    "pdf_export.fast_budget.exceeded",
                    None,
                    "fast export budget exhausted",
                    format!("stage={stage} elapsed_ms={}", elapsed.as_millis()),
                );
                Err("Timed out while preparing the export page.".to_string())
            } else {
                Ok(FAST_EXPORT_BUDGET - elapsed)
            }
        }
    }
}

pub(crate) fn build_export_html(document: &ActiveDocument, rendered_html: &str) -> String {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::document::{suggested_pdf_export_path, ActiveDocument};

    use crate::markdown;
    use lopdf::{Dictionary, Document as LoDocument, Object, Stream};

    use super::{
        apply_pdf_metadata, build_export_html, export_footer_text, export_preparation_mode,
        ExportPreparationMode, APP_NAME, APP_VERSION,
    };

    fn document(path: &str, markdown: &str) -> ActiveDocument {
        ActiveDocument::open_with_id(
            1,
            PathBuf::from(path),
            markdown.to_string(),
            "file:///tmp/".to_string(),
        )
    }

    #[test]
    fn suggested_pdf_path_replaces_markdown_extension() {
        assert_eq!(
            suggested_pdf_export_path(PathBuf::from("/tmp/note.md").as_path()),
            PathBuf::from("/tmp/note.pdf")
        );
        assert_eq!(
            suggested_pdf_export_path(PathBuf::from("/tmp/README.markdown").as_path()),
            PathBuf::from("/tmp/README.pdf")
        );
        assert_eq!(
            suggested_pdf_export_path(PathBuf::from("/tmp/notes").as_path()),
            PathBuf::from("/tmp/notes.pdf")
        );
    }

    #[test]
    fn export_html_contains_document_content_without_app_shell() {
        let document = document(
            "/tmp/example.md",
            "# Example\n\n```mermaid\nflowchart TD\n  A-->B\n```\n\n$$x^2$$",
        );
        let html = build_export_html(&document, &markdown::render_html(document.markdown()));

        assert!(html.contains("markdown-body export-page"));
        assert!(html.contains("window.markholaPreparePdf"));
        assert!(html.contains("mermaid-block"));
        assert!(html.contains("math math-display"));
        assert!(html.contains(&export_footer_text()));
        assert!(!html.contains("<div class=\"tabs-bar\""));
        assert!(!html.contains("<div class=\"editor-pane\""));
        assert!(!html.contains("<div class=\"about-overlay\""));
    }

    #[test]
    fn uses_fast_prepare_for_plain_markdown() {
        let document = document("/tmp/plain.md", "# Plain\n\nhello world");
        assert_eq!(
            export_preparation_mode(&markdown::render_html(document.markdown())),
            ExportPreparationMode::Fast
        );
    }

    #[test]
    fn uses_full_prepare_for_async_rendering_content() {
        let document = document(
            "/tmp/async.md",
            "# Async\n\n```mermaid\nflowchart TD\n  A-->B\n```\n\n![img](./demo.png)\n\n$E=mc^2$",
        );
        assert_eq!(
            export_preparation_mode(&markdown::render_html(document.markdown())),
            ExportPreparationMode::Full
        );
    }

    #[test]
    fn injects_pdf_metadata_with_markhola_version() {
        let document = document("/tmp/meta.md", "# Meta");
        let mut base = LoDocument::with_version("1.5");
        let pages_id = base.new_object_id();
        let page_id = base.new_object_id();
        let content_id = base.add_object(Stream::new(Dictionary::new(), Vec::new()));
        let resources_id = base.add_object(Dictionary::new());

        let mut page = Dictionary::new();
        page.set("Type", "Page");
        page.set("Parent", pages_id);
        page.set("Contents", content_id);
        page.set("Resources", resources_id);
        page.set(
            "MediaBox",
            vec![Object::Integer(0), Object::Integer(0), Object::Integer(100), Object::Integer(100)],
        );
        base.objects.insert(page_id, Object::Dictionary(page));

        let mut pages = Dictionary::new();
        pages.set("Type", "Pages");
        pages.set("Kids", vec![page_id.into()]);
        pages.set("Count", 1);
        base.objects.insert(pages_id, Object::Dictionary(pages));

        let mut catalog = Dictionary::new();
        catalog.set("Type", "Catalog");
        catalog.set("Pages", pages_id);
        let catalog_id = base.add_object(catalog);
        base.trailer.set("Root", catalog_id);

        let mut base_pdf = Vec::new();
        base.save_to(&mut base_pdf).expect("base pdf should serialize");

        let output = apply_pdf_metadata(&document, base_pdf).expect("metadata injection should succeed");
        let parsed = LoDocument::load_mem(&output).expect("output pdf should parse");
        let info_ref = parsed
            .trailer
            .get(b"Info")
            .expect("info should exist")
            .as_reference()
            .expect("info should be a reference");
        let info = parsed
            .get_dictionary(info_ref)
            .expect("info dictionary should be readable");

        assert_eq!(
            info.get(b"Creator").and_then(Object::as_str).ok(),
            Some(APP_NAME.as_bytes())
        );
        assert_eq!(
            info.get(b"Producer").and_then(Object::as_str).ok(),
            Some(format!("{APP_NAME} v{APP_VERSION}").as_bytes())
        );
    }
}
