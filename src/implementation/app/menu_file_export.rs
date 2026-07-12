use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{NSMenu, NSMenuItem};
use objc2_foundation::ns_string;

use super::menu_state::{remember_export_html, remember_export_pdf};

pub(super) fn build_export_menu(mtm: MainThreadMarker, target: &AnyObject) -> Retained<NSMenu> {
    let export_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Export"));

    let pdf = export_item(mtm, "PDF", Some(sel!(exportPdfDocument:)), target);
    pdf.setEnabled(false);
    remember_export_pdf(&pdf);
    export_menu.addItem(&pdf);

    let html = export_item(mtm, "HTML", Some(sel!(exportHtmlDocument:)), target);
    html.setEnabled(false);
    remember_export_html(&html);
    export_menu.addItem(&html);

    export_menu
}

fn export_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<objc2::runtime::Sel>,
    target: &AnyObject,
) -> Retained<NSMenuItem> {
    let item = match title {
        "PDF" => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("PDF"),
                action,
                ns_string!(""),
            )
        },
        _ => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("HTML"),
                action,
                ns_string!(""),
            )
        },
    };
    unsafe { item.setTarget(Some(target)) };
    item
}
