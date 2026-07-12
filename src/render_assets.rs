use std::fs::read_to_string;
use std::path::PathBuf;

pub const MERMAID_RUNTIME: &str = include_str!("../assets/mermaid/mermaid.min.js");
pub const MATHJAX_RUNTIME: &str = include_str!("../assets/mathjax/tex-svg-full.js");
const DEFAULT_APP_THEME_NAME: &str = "default";
const DEFAULT_APP_THEME_LAYOUT_FILE: &str = "layout.css";
const DEFAULT_APP_THEME_CSS: &str = include_str!("../themes/default/layout.css");

pub fn load_app_theme_css() -> String {
    for path in app_theme_candidates() {
        if let Ok(css) = read_to_string(&path) {
            return css;
        }
    }

    DEFAULT_APP_THEME_CSS.to_string()
}

pub fn load_app_theme_css_for_inline_style() -> String {
    load_app_theme_css().replace("</style", "<\\/style")
}

pub fn mermaid_runtime_for_inline_script() -> String {
    MERMAID_RUNTIME.replace("</script", "<\\/script")
}

pub fn mathjax_runtime_for_inline_script() -> String {
    MATHJAX_RUNTIME.replace("</script", "<\\/script")
}

fn app_theme_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(
            current_dir
                .join("themes")
                .join(DEFAULT_APP_THEME_NAME)
                .join(DEFAULT_APP_THEME_LAYOUT_FILE),
        );
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(executable_dir) = current_exe.parent() {
            candidates.push(
                executable_dir
                    .join("themes")
                    .join(DEFAULT_APP_THEME_NAME)
                    .join(DEFAULT_APP_THEME_LAYOUT_FILE),
            );
        }

        if let Some(contents_dir) = current_exe.parent().and_then(|path| path.parent()) {
            candidates.push(
                contents_dir
                    .join("Resources")
                    .join("themes")
                    .join(DEFAULT_APP_THEME_NAME)
                    .join(DEFAULT_APP_THEME_LAYOUT_FILE),
            );
        }
    }

    candidates.dedup();
    candidates
}
