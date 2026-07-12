use std::cell::RefCell;

use objc2::rc::Retained;
use objc2_app_kit::NSMenuItem;

thread_local! {
    static EXPORT_PDF_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static EXPORT_HTML_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static SAVE_AS_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static PRINT_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
}

pub(super) fn remember_save_as(item: &Retained<NSMenuItem>) {
    SAVE_AS_ITEM.with(|slot| *slot.borrow_mut() = Some(item.clone()));
}

pub(super) fn remember_print(item: &Retained<NSMenuItem>) {
    PRINT_ITEM.with(|slot| *slot.borrow_mut() = Some(item.clone()));
}

pub(super) fn remember_export_pdf(item: &Retained<NSMenuItem>) {
    EXPORT_PDF_ITEM.with(|slot| *slot.borrow_mut() = Some(item.clone()));
}

pub(super) fn remember_export_html(item: &Retained<NSMenuItem>) {
    EXPORT_HTML_ITEM.with(|slot| *slot.borrow_mut() = Some(item.clone()));
}

pub fn set_document_output_enabled(enabled: bool) {
    for_each_output_item(|item| item.setEnabled(enabled));
}

fn for_each_output_item(mut f: impl FnMut(&NSMenuItem)) {
    SAVE_AS_ITEM.with(|slot| slot.borrow().as_deref().map(&mut f));
    PRINT_ITEM.with(|slot| slot.borrow().as_deref().map(&mut f));
    EXPORT_PDF_ITEM.with(|slot| slot.borrow().as_deref().map(&mut f));
    EXPORT_HTML_ITEM.with(|slot| slot.borrow().as_deref().map(&mut f));
}
