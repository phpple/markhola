use tao::keyboard::ModifiersState;

use super::{OpenPathRequest, new_action_context};

pub(crate) fn primary_shortcut_is_pressed(modifiers: &ModifiersState) -> bool {
    #[cfg(target_os = "macos")]
    {
        modifiers.super_key()
    }

    #[cfg(not(target_os = "macos"))]
    {
        modifiers.control_key()
    }
}

pub(crate) fn primary_shortcut_label(key: &str) -> String {
    format!("{}+{key}", primary_shortcut_name())
}

pub(crate) fn ready_status_message() -> String {
    format!(
        "Ready. Open a Markdown file or press {}.",
        primary_shortcut_label("O")
    )
}

pub(crate) fn document_subtitle() -> String {
    format!(
        "Use {} or drag a Markdown file into the window.",
        primary_shortcut_label("O")
    )
}

pub(crate) fn about_footer() -> &'static str {
    "Built for local Markdown reading and writing on macOS and Windows 11."
}

pub(crate) fn browser_shortcut_uses_meta_key() -> bool {
    cfg!(target_os = "macos")
}

pub(crate) fn startup_open_requests() -> Vec<OpenPathRequest> {
    std::env::args_os()
        .skip(1)
        .filter(|value| !value.to_string_lossy().starts_with("--"))
        .map(|path| OpenPathRequest {
            ctx: new_action_context("cli-open-path"),
            path: path.into(),
        })
        .collect()
}

fn primary_shortcut_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "Command"
    } else {
        "Ctrl"
    }
}
