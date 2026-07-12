mod shell_markup;
mod shell_script;

use std::sync::atomic::{AtomicBool, Ordering};

use crate::render_assets;

use self::shell_markup::APP_SHELL_MARKUP;
use self::shell_script::APP_SHELL_SCRIPT;
use super::platform;

pub(crate) const APP_SHELL_URL: &str = "markhola://shell/index.html";

pub(crate) fn app_shell_html() -> String {
    [APP_SHELL_MARKUP, APP_SHELL_SCRIPT]
        .join("")
        .replace("__DOCUMENT_SUBTITLE__", &platform::document_subtitle())
        .replace("__ABOUT_FOOTER__", platform::about_footer())
        .replace(
            "__PRIMARY_SHORTCUT_IS_META__",
            if platform::browser_shortcut_uses_meta_key() {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__APP_THEME__",
            &render_assets::load_app_theme_css_for_inline_style(),
        )
        .replace(
            "__MERMAID_RUNTIME__",
            &render_assets::mermaid_runtime_for_inline_script(),
        )
        .replace(
            "__MATHJAX_RUNTIME__",
            &render_assets::mathjax_runtime_for_inline_script(),
        )
}

pub(crate) fn should_recover_shell_on_page_load(url: &str) -> bool {
    let trimmed = url.trim();
    trimmed.is_empty() || trimmed == "about:blank"
}

pub(crate) fn should_dispatch_shell_recovery(url: &str, suppress_once: &AtomicBool) -> bool {
    should_recover_shell_on_page_load(url) && !suppress_once.swap(false, Ordering::SeqCst)
}
