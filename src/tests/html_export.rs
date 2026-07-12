use std::path::PathBuf;

use crate::document::ActiveDocument;

use super::build_export_html;

#[test]
fn exported_html_contains_rendered_content_and_runtime_assets() {
    let document = ActiveDocument::open_with_id(
        1,
        PathBuf::from("/tmp/demo.md"),
        "# Hello\n\n```mermaid\nflowchart TD\n A-->B\n```".to_string(),
        "file:///tmp/".to_string(),
    );

    let html = build_export_html(&document);

    assert!(html.contains("<article class=\"markdown-body\">"));
    assert!(html.contains("mermaid-block"));
    assert!(html.contains("Exported by MarkHola v"));
    assert!(html.contains("window.MathJax"));
    assert!(html.contains("window.mermaid"));
    assert!(html.contains("overflow: auto;"));
}
