use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

use crate::app::AppTheme;
use crate::workspace::DocumentWorkspace;

use super::implementation::{
    file_paths_from_urls, load_document, reload_workspace_documents_from_disk,
};
use super::shell::{
    app_shell_html, should_dispatch_shell_recovery, should_recover_shell_on_page_load,
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

    assert!(!should_dispatch_shell_recovery(
        "about:blank",
        &suppress_once
    ));
    assert!(should_dispatch_shell_recovery(
        "about:blank",
        &suppress_once
    ));
    assert!(!should_dispatch_shell_recovery(
        "file:///tmp/demo.md",
        &suppress_once
    ));
}

#[test]
fn app_shell_includes_find_panel_markup_and_handlers() {
    let html = app_shell_html(AppTheme::Default);

    assert!(html.contains("id=\"findPanel\""));
    assert!(html.contains("data:image/png;base64,"));
    assert!(html.contains("<img class=\"about-logo\""));
    assert!(html.contains("window.openFindPanel = openFindPanel;"));
    assert!(html.contains("window.applyAppTheme = applyAppTheme;"));
    assert!(html.contains("id=\"appThemeStyle\""));
    assert!(html.contains("className = \"find-match\""));
    assert!(html.contains("replaceAllWritableMatches"));
    assert!(html.contains("event.key.toLowerCase() === \"f\""));
    assert!(!html.contains("class=\"bottom-bar\""));
}

#[test]
fn app_shell_uses_requested_theme_css() {
    let html = app_shell_html(AppTheme::Github);

    assert!(html.contains("#f6f8fa"));
    assert!(html.contains("id=\"appThemeStyle\""));
}

#[test]
fn app_theme_keys_and_labels_are_stable() {
    let summary = AppTheme::ALL
        .iter()
        .map(|theme| (theme.key(), theme.label()))
        .collect::<Vec<_>>();

    assert_eq!(
        summary,
        vec![
            ("default", "Default"),
            ("github", "GitHub"),
            ("dark", "Dark"),
            ("light", "Light"),
        ]
    );
}

#[test]
fn app_theme_round_trips_from_stable_key() {
    for theme in AppTheme::ALL {
        assert_eq!(AppTheme::from_key(theme.key()), Some(theme));
    }

    assert_eq!(AppTheme::from_key("unknown"), None);
}

#[test]
fn file_paths_from_urls_keeps_all_file_paths_in_order() {
    let paths = file_paths_from_urls(vec![
        Url::parse("file:///tmp/one.md").unwrap(),
        Url::parse("file:///tmp/two.md").unwrap(),
        Url::parse("file:///tmp/three.md").unwrap(),
    ]);

    assert_eq!(
        paths,
        vec![
            PathBuf::from("/tmp/one.md"),
            PathBuf::from("/tmp/two.md"),
            PathBuf::from("/tmp/three.md"),
        ]
    );
}

#[test]
fn file_paths_from_urls_ignores_non_file_urls() {
    let paths = file_paths_from_urls(vec![
        Url::parse("https://example.com/demo.md").unwrap(),
        Url::parse("file:///tmp/real.md").unwrap(),
    ]);

    assert_eq!(paths, vec![PathBuf::from("/tmp/real.md")]);
}
