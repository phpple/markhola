use std::fs;
use std::path::{Path, PathBuf};

use rfd::FileDialog;

use crate::app::log_event;
use crate::document::ActiveDocument;
use crate::markdown;
use crate::render_assets;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const HTML_EXPORT_CSS: &str = r#"
html,
body {
  min-height: 100%;
  overflow: auto;
}

body {
  padding: 32px 24px 48px;
}
"#;

const HTML_EXPORT_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <base href="__BASE_URL__" />
    <title>__TITLE__</title>
    <style>__APP_THEME__</style>
    <style>__EXPORT_CSS__</style>
  </head>
  <body>
    <article class="markdown-body">__DOCUMENT_HTML__</article>
    <footer style="margin:24px auto 48px;max-width:816px;padding:0 56px;text-align:right;font-size:10px;line-height:1.4;color:#f8f7f4;user-select:none;">
      Exported by MarkHola v__APP_VERSION__
    </footer>
    <script>
      window.MathJax = {
        startup: { typeset: false },
        svg: { fontCache: "none" }
      };
    </script>
    <script>__MERMAID_RUNTIME__</script>
    <script>__MATHJAX_RUNTIME__</script>
    <script>
      const escapeHtml = (value) =>
        value
          .replaceAll("&", "&amp;")
          .replaceAll("<", "&lt;")
          .replaceAll(">", "&gt;")
          .replaceAll('"', "&quot;")
          .replaceAll("'", "&#39;");

      const ensureMermaidInitialized = () => {
        if (!window.mermaid) return;
        window.mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: "default"
        });
      };

      const renderMermaidDiagrams = async () => {
        ensureMermaidInitialized();
        if (!window.mermaid) return;
        const blocks = document.querySelectorAll(".mermaid-block");
        for (const [index, block] of blocks.entries()) {
          const sourceNode = block.querySelector(".mermaid-block__source");
          const diagramNode = block.querySelector(".mermaid-block__diagram");
          const statusNode = block.querySelector(".mermaid-block__status");
          const source = sourceNode?.textContent || "";
          if (!diagramNode) continue;
          try {
            const { svg } = await window.mermaid.render(`markhola-html-export-${index}-${Date.now()}`, source);
            diagramNode.innerHTML = svg;
            statusNode?.remove();
          } catch (error) {
            const message =
              error && typeof error === "object" && "message" in error
                ? String(error.message)
                : String(error || "Unknown Mermaid error");
            diagramNode.innerHTML = `<pre class="mermaid-block__error">${escapeHtml(message)}\n\n${escapeHtml(source)}</pre>`;
          }
        }
      };

      const ensureMathJaxReady = () => {
        if (!window.MathJax || !window.MathJax.startup) return null;
        return window.MathJax.startup.promise;
      };

      const extractRenderedMathNode = (rendered) =>
        rendered.querySelector("mjx-container") || rendered.firstElementChild || rendered;

      const renderMathSource = async (node, source, display) => {
        const ready = ensureMathJaxReady();
        if (!ready) return;
        await ready;
        const rendered = await window.MathJax.tex2svgPromise(source, { display });
        const mathNode = extractRenderedMathNode(rendered);
        node.replaceChildren(mathNode.cloneNode(true));
      };

      const renderMathExpressions = async () => {
        if (!window.MathJax) return;
        const mathNodes = document.querySelectorAll(".math.math-inline, .math.math-display");
        for (const node of mathNodes) {
          const source = node.textContent || "";
          const display = node.classList.contains("math-display");
          try {
            await renderMathSource(node, source, display);
          } catch (error) {
            node.innerHTML = `<code>${escapeHtml(String(error || "Math render failed"))}</code>`;
          }
        }
        const blocks = document.querySelectorAll(".math-block");
        for (const block of blocks) {
          const sourceNode = block.querySelector(".math-block__source");
          const formulaNode = block.querySelector(".math-block__formula");
          const statusNode = block.querySelector(".math-block__status");
          const source = sourceNode?.textContent || "";
          if (!formulaNode) continue;
          try {
            await renderMathSource(formulaNode, source, true);
            statusNode?.remove();
          } catch (error) {
            formulaNode.innerHTML = `<pre class="math-block__error">${escapeHtml(String(error || "Math render failed"))}</pre>`;
          }
        }
      };

      void renderMermaidDiagrams().then(renderMathExpressions);
    </script>
  </body>
</html>
"#;

pub enum HtmlExportOutcome {
    Cancelled,
    Exported(PathBuf),
}

pub fn export_document(document: &ActiveDocument) -> Result<HtmlExportOutcome, String> {
    let Some(export_path) = choose_export_path(document) else {
        log_event(
            "html_export.cancelled",
            None,
            "HTML export cancelled in save dialog",
            "",
        );
        return Ok(HtmlExportOutcome::Cancelled);
    };

    write_export(document, &export_path)?;

    Ok(HtmlExportOutcome::Exported(export_path))
}

pub fn export_markdown_file_to_path(input_path: &Path, output_path: &Path) -> Result<(), String> {
    let input_path = std::fs::canonicalize(input_path)
        .map_err(|error| format!("Failed to canonicalize input path: {error}"))?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create export directory: {error}"))?;
    }

    let markdown = crate::file_io::load_markdown(&input_path)?;
    let base_url = crate::file_io::directory_base_url(&input_path)?;
    let document = ActiveDocument::open_with_id(1, input_path, markdown, base_url);
    write_export(&document, output_path)
}

fn write_export(document: &ActiveDocument, export_path: &Path) -> Result<(), String> {
    let html = build_export_html(document);
    fs::write(export_path, html).map_err(|error| format!("Failed to write HTML file: {error}"))?;
    log_event(
        "html_export.end",
        None,
        "finished HTML export",
        format!("output={}", export_path.display()),
    );
    Ok(())
}

fn choose_export_path(document: &ActiveDocument) -> Option<PathBuf> {
    let suggested_name = suggested_html_export_path(document.file_path())
        .file_name()?
        .to_string_lossy()
        .into_owned();

    FileDialog::new()
        .add_filter("HTML", &["html"])
        .set_title("Export HTML")
        .set_directory(
            document
                .file_path()
                .parent()
                .unwrap_or(document.file_path()),
        )
        .set_file_name(suggested_name)
        .save_file()
}

fn suggested_html_export_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            path.file_name()
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("document");

    path.with_file_name(format!("{stem}.html"))
}

pub(crate) fn build_export_html(document: &ActiveDocument) -> String {
    HTML_EXPORT_TEMPLATE
        .replace("__TITLE__", document.file_name())
        .replace("__BASE_URL__", document.base_url())
        .replace("__APP_VERSION__", APP_VERSION)
        .replace(
            "__APP_THEME__",
            &render_assets::load_app_theme_css_for_inline_style("default"),
        )
        .replace("__EXPORT_CSS__", HTML_EXPORT_CSS)
        .replace(
            "__MERMAID_RUNTIME__",
            &render_assets::mermaid_runtime_for_inline_script(),
        )
        .replace(
            "__MATHJAX_RUNTIME__",
            &render_assets::mathjax_runtime_for_inline_script(),
        )
        .replace(
            "__DOCUMENT_HTML__",
            &markdown::render_html(document.markdown()),
        )
}
