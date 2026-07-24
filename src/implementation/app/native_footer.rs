use tao::window::Window;
use wry::WebView;

use crate::app::AppTheme;
use crate::workspace::DocumentWorkspace;

#[cfg(target_os = "macos")]
use objc2::MainThreadMarker;
#[cfg(target_os = "macos")]
use objc2::MainThreadOnly;
#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2::msg_send;
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSBox, NSBoxType, NSColor, NSFont, NSTextField, NSWindow,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
#[cfg(target_os = "macos")]
use tao::platform::macos::WindowExtMacOS;
#[cfg(target_os = "macos")]
use wry::WebViewExtMacOS;

const FOOTER_HEIGHT: f64 = 42.0;
const FOOTER_PADDING_X: f64 = 16.0;
const FOOTER_LABEL_Y: f64 = 11.0;
const FOOTER_LABEL_HEIGHT: f64 = 18.0;
const FOOTER_GAP: f64 = 10.0;
const FOOTER_STATUS_WIDTH: f64 = 160.0;
const FOOTER_MODE_WIDTH: f64 = 118.0;
const FOOTER_LINES_WIDTH: f64 = 86.0;
const FOOTER_WORDS_WIDTH: f64 = 90.0;

pub(super) struct NativeFooter {
    #[cfg(target_os = "macos")]
    handle: Option<NativeFooterHandle>,
}

#[cfg(target_os = "macos")]
struct NativeFooterHandle {
    footer_view: Retained<NSBox>,
    path_field: Retained<NSTextField>,
    words_field: Retained<NSTextField>,
    lines_field: Retained<NSTextField>,
    mode_field: Retained<NSTextField>,
    status_field: Retained<NSTextField>,
}

impl NativeFooter {
    pub(super) fn install(window: &Window, webview: &WebView, theme: AppTheme) -> Self {
        #[cfg(target_os = "macos")]
        unsafe {
            let Some(mtm) = MainThreadMarker::new() else {
                return Self { handle: None };
            };
            let ns_window = &*(window.ns_window() as *mut NSWindow);
            let Some(content_view) = ns_window.contentView() else {
                return Self { handle: None };
            };

            let footer_view = NSBox::initWithFrame(
                NSBox::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(100.0, FOOTER_HEIGHT)),
            );
            footer_view.setBoxType(NSBoxType::Custom);
            footer_view.setBorderWidth(0.0);
            footer_view.setAutoresizingMask(
                NSAutoresizingMaskOptions::ViewWidthSizable
                    | NSAutoresizingMaskOptions::ViewMaxYMargin,
            );

            let path_field = footer_label(mtm, "");
            let words_field = footer_label(mtm, "");
            let lines_field = footer_label(mtm, "");
            let mode_field = footer_label(mtm, "");
            let status_field = footer_label(mtm, "");

            apply_footer_fonts(
                &path_field,
                &words_field,
                &lines_field,
                &mode_field,
                &status_field,
            );

            footer_view.addSubview(&path_field);
            footer_view.addSubview(&words_field);
            footer_view.addSubview(&lines_field);
            footer_view.addSubview(&mode_field);
            footer_view.addSubview(&status_field);
            content_view.addSubview(&footer_view);

            let handle = NativeFooterHandle {
                footer_view,
                path_field,
                words_field,
                lines_field,
                mode_field,
                status_field,
            };

            let footer = Self {
                handle: Some(handle),
            };
            footer.set_theme(theme);
            footer.relayout(window, webview);
            footer
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (window, webview, theme);
            Self {}
        }
    }

    pub(super) fn set_theme(&self, theme: AppTheme) {
        #[cfg(target_os = "macos")]
        {
            let Some(handle) = &self.handle else {
                return;
            };
            let (background, primary, secondary) = footer_theme_colors(theme);
            handle.footer_view.setFillColor(&background);
            handle.path_field.setTextColor(Some(&secondary));
            handle.words_field.setTextColor(Some(&primary));
            handle.lines_field.setTextColor(Some(&primary));
            handle.mode_field.setTextColor(Some(&primary));
            handle.status_field.setTextColor(Some(&primary));
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = theme;
        }
    }

    pub(super) fn relayout(&self, window: &Window, webview: &WebView) {
        #[cfg(target_os = "macos")]
        unsafe {
            let Some(handle) = &self.handle else {
                return;
            };
            let ns_window = &*(window.ns_window() as *mut NSWindow);
            let Some(content_view) = ns_window.contentView() else {
                return;
            };
            let content_frame = content_view.frame();
            let width = content_frame.size.width;
            let height = content_frame.size.height;
            let footer_height = FOOTER_HEIGHT.min(height.max(0.0));

            handle.footer_view.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(width, footer_height),
            ));

            let status_x = width - FOOTER_PADDING_X - FOOTER_STATUS_WIDTH;
            let mode_x = status_x - FOOTER_GAP - FOOTER_MODE_WIDTH;
            let lines_x = mode_x - FOOTER_GAP - FOOTER_LINES_WIDTH;
            let words_x = lines_x - FOOTER_GAP - FOOTER_WORDS_WIDTH;
            let path_width = (words_x - FOOTER_GAP - FOOTER_PADDING_X).max(120.0);

            handle.path_field.setFrame(NSRect::new(
                NSPoint::new(FOOTER_PADDING_X, FOOTER_LABEL_Y),
                NSSize::new(path_width, FOOTER_LABEL_HEIGHT),
            ));
            handle.words_field.setFrame(NSRect::new(
                NSPoint::new(words_x, FOOTER_LABEL_Y),
                NSSize::new(FOOTER_WORDS_WIDTH, FOOTER_LABEL_HEIGHT),
            ));
            handle.lines_field.setFrame(NSRect::new(
                NSPoint::new(lines_x, FOOTER_LABEL_Y),
                NSSize::new(FOOTER_LINES_WIDTH, FOOTER_LABEL_HEIGHT),
            ));
            handle.mode_field.setFrame(NSRect::new(
                NSPoint::new(mode_x, FOOTER_LABEL_Y),
                NSSize::new(FOOTER_MODE_WIDTH, FOOTER_LABEL_HEIGHT),
            ));
            handle.status_field.setFrame(NSRect::new(
                NSPoint::new(status_x, FOOTER_LABEL_Y),
                NSSize::new(FOOTER_STATUS_WIDTH, FOOTER_LABEL_HEIGHT),
            ));

            let webview_handle = webview.webview();
            webview_handle.setFrame(NSRect::new(
                NSPoint::new(0.0, footer_height),
                NSSize::new(width, (height - footer_height).max(0.0)),
            ));
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (window, webview);
        }
    }

    pub(super) fn sync(&self, workspace: &DocumentWorkspace, status: &str) {
        #[cfg(target_os = "macos")]
        unsafe {
            let Some(handle) = &self.handle else {
                return;
            };

            if let Some(active) = workspace.active_document_snapshot() {
                set_label_text(&handle.path_field, &format!("Path: {}", active.file_path));
                set_label_text(&handle.words_field, &format!("Words {}", active.word_count));
                set_label_text(&handle.lines_field, &format!("Lines {}", active.line_count));
                set_label_text(&handle.mode_field, &format!("Mode {}", active.mode_label));
                set_label_text(&handle.status_field, &format!("Status {}", status));
                set_hidden(&handle.words_field, false);
                set_hidden(&handle.lines_field, false);
                set_hidden(&handle.mode_field, false);
            } else {
                set_label_text(&handle.path_field, "Path: No file opened");
                set_label_text(&handle.words_field, "");
                set_label_text(&handle.lines_field, "");
                set_label_text(&handle.mode_field, "");
                set_label_text(&handle.status_field, &format!("Status {}", status));
                set_hidden(&handle.words_field, true);
                set_hidden(&handle.lines_field, true);
                set_hidden(&handle.mode_field, true);
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (workspace, status);
        }
    }
}

#[cfg(target_os = "macos")]
unsafe fn footer_label(mtm: MainThreadMarker, value: &str) -> Retained<NSTextField> {
    let string = NSString::from_str(value);
    let label = NSTextField::labelWithString(&string, mtm);
    label.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewMaxXMargin | NSAutoresizingMaskOptions::ViewMinYMargin,
    );
    label
}

#[cfg(target_os = "macos")]
fn footer_theme_colors(
    theme: AppTheme,
) -> (Retained<NSColor>, Retained<NSColor>, Retained<NSColor>) {
    match theme {
        AppTheme::Default => (
            rgb_color(255, 255, 255),
            rgb_color(43, 36, 29),
            rgb_color(111, 98, 88),
        ),
        AppTheme::Github => (
            rgb_color(246, 248, 250),
            rgb_color(31, 35, 40),
            rgb_color(87, 96, 106),
        ),
        AppTheme::Dark => (
            rgb_color(13, 17, 23),
            rgb_color(230, 237, 243),
            rgb_color(139, 148, 158),
        ),
        AppTheme::Light => (
            rgb_color(238, 243, 248),
            rgb_color(32, 48, 65),
            rgb_color(93, 114, 136),
        ),
    }
}

#[cfg(target_os = "macos")]
fn rgb_color(red: u8, green: u8, blue: u8) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        f64::from(red) / 255.0,
        f64::from(green) / 255.0,
        f64::from(blue) / 255.0,
        1.0,
    )
}

#[cfg(target_os = "macos")]
unsafe fn apply_footer_fonts(
    path_field: &NSTextField,
    words_field: &NSTextField,
    lines_field: &NSTextField,
    mode_field: &NSTextField,
    status_field: &NSTextField,
) {
    let font = NSFont::systemFontOfSize(12.0);
    let _: () = msg_send![path_field, setFont: Some(&*font)];
    let _: () = msg_send![words_field, setFont: Some(&*font)];
    let _: () = msg_send![lines_field, setFont: Some(&*font)];
    let _: () = msg_send![mode_field, setFont: Some(&*font)];
    let _: () = msg_send![status_field, setFont: Some(&*font)];
}

#[cfg(target_os = "macos")]
unsafe fn set_label_text(field: &NSTextField, value: &str) {
    let string = NSString::from_str(value);
    let _: () = msg_send![field, setStringValue: &*string];
}

#[cfg(target_os = "macos")]
unsafe fn set_hidden(view: &AnyObject, hidden: bool) {
    let _: () = msg_send![view, setHidden: hidden];
}
