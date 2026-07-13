use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::OnceLock;

pub const MERMAID_RUNTIME: &str = include_str!("../../assets/mermaid/mermaid.min.js");
pub const MATHJAX_RUNTIME: &str = include_str!("../../assets/mathjax/tex-svg-full.js");
const DEFAULT_APP_THEME_NAME: &str = "default";
const DEFAULT_APP_THEME_LAYOUT_FILE: &str = "layout.css";
const DEFAULT_APP_THEME_CSS: &str = include_str!("../../themes/default/layout.css");
const DEFAULT_APP_LOGO_PNG: &[u8] = include_bytes!("../../assets/logo.png");

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

pub fn app_logo_data_url() -> &'static str {
    static APP_LOGO_DATA_URL: OnceLock<String> = OnceLock::new();

    APP_LOGO_DATA_URL.get_or_init(|| {
        format!(
            "data:image/png;base64,{}",
            encode_base64(DEFAULT_APP_LOGO_PNG)
        )
    })
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

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let value = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        encoded.push(TABLE[((value >> 18) & 0x3F) as usize] as char);
        encoded.push(TABLE[((value >> 12) & 0x3F) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            TABLE[((value >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            TABLE[(value & 0x3F) as usize] as char
        } else {
            '='
        });
    }

    encoded
}
