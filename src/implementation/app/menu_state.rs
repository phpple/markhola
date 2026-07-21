use std::cell::RefCell;

use objc2::rc::Retained;
use objc2_app_kit::{
    NSControlStateValueOff, NSControlStateValueOn, NSMenuItem,
};

use crate::app::AppTheme;

thread_local! {
    static EXPORT_PDF_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static EXPORT_HTML_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static SAVE_AS_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static PRINT_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static DEFAULT_THEME_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static GITHUB_THEME_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static DARK_THEME_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
    static LIGHT_THEME_ITEM: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub(super) enum ThemeMenuSlot {
    Default,
    Github,
    Dark,
    Light,
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

pub(super) fn remember_theme_item(slot: ThemeMenuSlot, item: &Retained<NSMenuItem>) {
    theme_item_slot(slot).with(|state| *state.borrow_mut() = Some(item.clone()));
}

pub fn set_document_output_enabled(enabled: bool) {
    for_each_output_item(|item| item.setEnabled(enabled));
}

pub fn set_selected_theme(theme: AppTheme) {
    for slot in [
        ThemeMenuSlot::Default,
        ThemeMenuSlot::Github,
        ThemeMenuSlot::Dark,
        ThemeMenuSlot::Light,
    ] {
        theme_item_slot(slot).with(|state| {
            if let Some(item) = state.borrow().as_deref() {
                item.setState(if theme_for_slot(slot) == theme {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                });
            }
        });
    }
}

fn for_each_output_item(mut f: impl FnMut(&NSMenuItem)) {
    SAVE_AS_ITEM.with(|slot| slot.borrow().as_deref().map(&mut f));
    PRINT_ITEM.with(|slot| slot.borrow().as_deref().map(&mut f));
    EXPORT_PDF_ITEM.with(|slot| slot.borrow().as_deref().map(&mut f));
    EXPORT_HTML_ITEM.with(|slot| slot.borrow().as_deref().map(&mut f));
}

fn theme_item_slot(
    slot: ThemeMenuSlot,
) -> &'static std::thread::LocalKey<RefCell<Option<Retained<NSMenuItem>>>> {
    match slot {
        ThemeMenuSlot::Default => &DEFAULT_THEME_ITEM,
        ThemeMenuSlot::Github => &GITHUB_THEME_ITEM,
        ThemeMenuSlot::Dark => &DARK_THEME_ITEM,
        ThemeMenuSlot::Light => &LIGHT_THEME_ITEM,
    }
}

fn theme_for_slot(slot: ThemeMenuSlot) -> AppTheme {
    match slot {
        ThemeMenuSlot::Default => AppTheme::Default,
        ThemeMenuSlot::Github => AppTheme::Github,
        ThemeMenuSlot::Dark => AppTheme::Dark,
        ThemeMenuSlot::Light => AppTheme::Light,
    }
}
