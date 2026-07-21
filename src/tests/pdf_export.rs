use std::path::PathBuf;
use std::process::Command;

use crate::document::{ActiveDocument, suggested_pdf_export_path};
use crate::markdown;
use lopdf::{Dictionary, Document as LoDocument, Object, Stream};

use super::implementation::{
    APP_NAME, APP_VERSION, EXPORT_WEBVIEW_HEIGHT, EXPORT_WEBVIEW_WIDTH, ExportMeasurement,
    ExportPreparationMode, apply_pdf_metadata, build_export_html, export_capture_rect,
    export_footer_text, export_preparation_mode, printable_page_count_for_height,
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
        vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(100),
            Object::Integer(100),
        ],
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
    base.save_to(&mut base_pdf)
        .expect("base pdf should serialize");

    let output =
        apply_pdf_metadata(&document, base_pdf).expect("metadata injection should succeed");
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

#[test]
fn export_capture_rect_uses_full_measured_height() {
    let rect = export_capture_rect(&ExportMeasurement {
        width: EXPORT_WEBVIEW_WIDTH,
        height: 4820.0,
    });

    assert_eq!(rect.size.width, EXPORT_WEBVIEW_WIDTH);
    assert_eq!(rect.size.height, 4820.0);
}

#[test]
fn export_capture_rect_respects_minimum_viewport_size() {
    let rect = export_capture_rect(&ExportMeasurement {
        width: 100.0,
        height: 200.0,
    });

    assert_eq!(rect.size.width, EXPORT_WEBVIEW_WIDTH);
    assert_eq!(rect.size.height, EXPORT_WEBVIEW_HEIGHT);
}

#[test]
fn printable_page_count_rounds_up_for_partial_last_page() {
    assert_eq!(printable_page_count_for_height(EXPORT_WEBVIEW_HEIGHT), 1);
    assert_eq!(printable_page_count_for_height(EXPORT_WEBVIEW_HEIGHT + 1.0), 2);
    assert_eq!(printable_page_count_for_height(6647.0), 6);
}

#[test]
#[ignore = "Requires WKWebView JavaScript evaluation support (may fail in sandboxed/headless environments)."]
fn mermaid_example_print_preview_generates_expected_page_count() {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let example_path = root_dir.join("examples/mermaid.md");

    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("markhola")
        .arg("--")
        .arg("--smoke-print-pages")
        .arg(&example_path)
        .current_dir(&root_dir)
        .output()
        .expect("smoke print page-count command should start");

    assert!(
        output.status.success(),
        "smoke print page-count failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let page_count = stdout
        .split("pages=")
        .nth(1)
        .and_then(|value| value.trim().parse::<usize>().ok())
        .expect("stdout should contain the computed print page count");

    assert_eq!(page_count, 6, "examples/mermaid.md page count changed");
}
