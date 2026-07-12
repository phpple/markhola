use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::ns_string;

pub(super) fn add_edit_menu(mtm: MainThreadMarker, main_menu: &NSMenu, target: &AnyObject) {
    let edit_menu_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Edit"),
            None,
            ns_string!(""),
        )
    };
    main_menu.addItem(&edit_menu_item);

    let edit_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Edit"));
    let undo_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Undo"),
            Some(sel!(undo:)),
            ns_string!("z"),
        )
    };
    undo_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
    edit_menu.addItem(&undo_item);

    let redo_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Redo"),
            Some(sel!(redo:)),
            ns_string!("r"),
        )
    };
    redo_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
    edit_menu.addItem(&redo_item);
    edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
    let find_item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            ns_string!("Find"),
            Some(sel!(openFindPanel:)),
            ns_string!("f"),
        )
    };
    unsafe { find_item.setTarget(Some(target)) };
    find_item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
    edit_menu.addItem(&find_item);
    edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
    edit_menu.addItem(&basic_system_item(mtm, "Cut", Some(sel!(cut:)), "x"));
    edit_menu.addItem(&basic_system_item(mtm, "Copy", Some(sel!(copy:)), "c"));
    edit_menu.addItem(&basic_system_item(mtm, "Paste", Some(sel!(paste:)), "v"));
    edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
    edit_menu.addItem(&basic_system_item(
        mtm,
        "Select All",
        Some(sel!(selectAll:)),
        "a",
    ));
    edit_menu_item.setSubmenu(Some(&edit_menu));
}

fn basic_system_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<objc2::runtime::Sel>,
    key: &str,
) -> objc2::rc::Retained<NSMenuItem> {
    let item = match title {
        "Cut" => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Cut"),
                action,
                ns_string!("x"),
            )
        },
        "Copy" => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Copy"),
                action,
                ns_string!("c"),
            )
        },
        "Paste" => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Paste"),
                action,
                ns_string!("v"),
            )
        },
        _ => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Select All"),
                action,
                ns_string!("a"),
            )
        },
    };
    let _ = key;
    item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
    item
}
