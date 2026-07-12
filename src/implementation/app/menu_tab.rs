use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::ns_string;

pub(super) fn add_tab_menu(mtm: MainThreadMarker, main_menu: &NSMenu, target: &AnyObject) {
    let tab_menu_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Tab"),
            None,
            ns_string!(""),
        )
    };
    main_menu.addItem(&tab_menu_item);

    let tab_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Tab"));
    tab_menu.addItem(&action(
        mtm,
        "Next Tab",
        Some(sel!(activateNextDocument:)),
        "]",
        NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
        target,
    ));
    tab_menu.addItem(&action(
        mtm,
        "Previous Tab",
        Some(sel!(activatePreviousDocument:)),
        "[",
        NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
        target,
    ));
    tab_menu.addItem(&NSMenuItem::separatorItem(mtm));
    tab_menu.addItem(&action(
        mtm,
        "Close Tab",
        Some(sel!(closeCurrentDocument:)),
        "w",
        NSEventModifierFlags::Command,
        target,
    ));
    tab_menu.addItem(&action(
        mtm,
        "Close Other Tabs",
        Some(sel!(closeOtherDocuments:)),
        "w",
        NSEventModifierFlags::Command | NSEventModifierFlags::Option,
        target,
    ));
    tab_menu.addItem(&action(
        mtm,
        "Close All Tabs",
        Some(sel!(closeAllDocuments:)),
        "w",
        NSEventModifierFlags::Command | NSEventModifierFlags::Option | NSEventModifierFlags::Shift,
        target,
    ));
    tab_menu_item.setSubmenu(Some(&tab_menu));
}

fn action(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<objc2::runtime::Sel>,
    key: &str,
    modifiers: NSEventModifierFlags,
    target: &AnyObject,
) -> objc2::rc::Retained<NSMenuItem> {
    let item = match (title, key) {
        ("Next Tab", "]") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Next Tab"),
                action,
                ns_string!("]"),
            )
        },
        ("Previous Tab", "[") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Previous Tab"),
                action,
                ns_string!("["),
            )
        },
        ("Close Tab", "w") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Close Tab"),
                action,
                ns_string!("w"),
            )
        },
        ("Close Other Tabs", "w") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Close Other Tabs"),
                action,
                ns_string!("w"),
            )
        },
        _ => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Close All Tabs"),
                action,
                ns_string!("w"),
            )
        },
    };
    unsafe { item.setTarget(Some(target)) };
    item.setKeyEquivalentModifierMask(modifiers);
    item
}
