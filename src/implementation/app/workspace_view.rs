use tao::window::Window;
use wry::WebView;

use crate::workspace::DocumentWorkspace;

use super::{
    APP_AUTHOR, APP_BUILD_PLATFORM, APP_BUILD_TARGET, APP_GITHUB_URL, APP_VERSION, StatusPayload,
    WINDOW_TITLE, WorkspacePresentation,
};
#[cfg(target_os = "macos")]
use super::macos_menu;

pub(super) fn present_workspace(
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

pub(super) fn sync_native_menu_state(_workspace: &DocumentWorkspace) {
    #[cfg(target_os = "macos")]
    macos_menu::set_document_output_enabled(_workspace.active_document().is_some());
}

pub(super) fn sync_workspace_state(
    window: &Window,
    webview: &WebView,
    workspace: &DocumentWorkspace,
    status: &str,
) {
    update_window_title(window, workspace.active_window_title().as_deref());
    evaluate_workspace_script(webview, "window.updateWorkspaceState", workspace, status);
}

pub(super) fn render_status(webview: &WebView, message: &str, level: &str) {
    render_status_with_action(webview, message, level, None, None);
}

pub(super) fn render_status_with_action(
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
    if let Ok(serialized) = serde_json::to_string(&payload) {
        let _ = webview.evaluate_script(&format!("window.showStatus({serialized});"));
    }
}

pub(super) fn render_about(webview: &WebView) {
    let script = format!(
        "window.showAbout({{version:{}, author:{}, githubUrl:{}, buildTarget:{}, buildPlatform:{}}});",
        serde_json::to_string(APP_VERSION).unwrap_or_else(|_| "\"0.8.0\"".to_string()),
        serde_json::to_string(APP_AUTHOR).unwrap_or_else(|_| "\"Ronnie Deng\"".to_string()),
        serde_json::to_string(APP_GITHUB_URL)
            .unwrap_or_else(|_| "\"https://github.com/phpple/markhola\"".to_string()),
        serde_json::to_string(APP_BUILD_TARGET).unwrap_or_else(|_| "\"unknown\"".to_string()),
        serde_json::to_string(APP_BUILD_PLATFORM).unwrap_or_else(|_| "\"unknown\"".to_string()),
    );
    let _ = webview.evaluate_script(&script);
}

fn render_workspace(webview: &WebView, workspace: &DocumentWorkspace, status: &str) {
    evaluate_workspace_script(webview, "window.renderWorkspace", workspace, status);
}

fn update_window_title(window: &Window, title: Option<&str>) {
    window.set_title(title.unwrap_or(WINDOW_TITLE));
}

fn workspace_presentation(workspace: &DocumentWorkspace, status: &str) -> WorkspacePresentation {
    WorkspacePresentation {
        tabs: workspace.tab_snapshots(),
        active_document: workspace.active_document_snapshot(),
        status_message: status.to_string(),
    }
}

fn evaluate_workspace_script(
    webview: &WebView,
    function_name: &str,
    workspace: &DocumentWorkspace,
    status: &str,
) {
    let payload = workspace_presentation(workspace, status);
    let serialized = match serde_json::to_string(&payload) {
        Ok(serialized) => serialized,
        Err(error) => {
            render_status(
                webview,
                &format!("Failed to serialize workspace: {error}"),
                "error",
            );
            return;
        }
    };
    if let Err(error) = webview.evaluate_script(&format!("{function_name}({serialized});")) {
        render_status(webview, &format!("WebView script error: {error}"), "error");
    }
}
