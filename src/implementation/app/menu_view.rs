use objc2::runtime::AnyObject;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::ns_string;

use crate::app::AppTheme;

use super::menu_file_items::action;
use super::menu_state::{remember_theme_item, ThemeMenuSlot};

pub(super) fn add_view_menu(mtm: MainThreadMarker, main_menu: &NSMenu, target: &AnyObject) {
    let view_menu_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("View"),
            None,
            ns_string!(""),
        )
    };
    main_menu.addItem(&view_menu_item);

    let view_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("View"));
    let theme_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Theme"),
            None,
            ns_string!(""),
        )
    };
    theme_item.setSubmenu(Some(&build_theme_menu(mtm, target)));
    view_menu.addItem(&theme_item);
    view_menu.addItem(&NSMenuItem::separatorItem(mtm));
    view_menu.addItem(&action(
        mtm,
        "Toggle Full Screen",
        Some(sel!(toggleFullscreenWindow:)),
        "f",
        NSEventModifierFlags::Control | NSEventModifierFlags::Command,
        target,
    ));

    view_menu_item.setSubmenu(Some(&view_menu));
}

fn build_theme_menu(mtm: MainThreadMarker, target: &AnyObject) -> Retained<NSMenu> {
    let theme_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Theme"));

    for theme in AppTheme::ALL {
        let item = action(
            mtm,
            theme.label(),
            Some(selector_for_theme(theme)),
            "",
            NSEventModifierFlags::empty(),
            target,
        );
        remember_theme_item(slot_for_theme(theme), &item);
        theme_menu.addItem(&item);
    }

    theme_menu
}

fn slot_for_theme(theme: AppTheme) -> ThemeMenuSlot {
    match theme {
        AppTheme::Default => ThemeMenuSlot::Default,
        AppTheme::Github => ThemeMenuSlot::Github,
        AppTheme::Dark => ThemeMenuSlot::Dark,
        AppTheme::Light => ThemeMenuSlot::Light,
    }
}

fn selector_for_theme(theme: AppTheme) -> objc2::runtime::Sel {
    match theme {
        AppTheme::Default => sel!(selectDefaultTheme:),
        AppTheme::Github => sel!(selectGithubTheme:),
        AppTheme::Dark => sel!(selectDarkTheme:),
        AppTheme::Light => sel!(selectLightTheme:),
    }
}
