use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSEventModifierFlags, NSMenuItem};
use objc2_foundation::ns_string;

pub(super) fn action(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<objc2::runtime::Sel>,
    key: &str,
    modifiers: NSEventModifierFlags,
    target: &AnyObject,
) -> objc2::rc::Retained<NSMenuItem> {
    let item = match (title, key) {
        ("Open", "o") => item_with_key(mtm, "Open", action, "o"),
        ("Save", "s") => item_with_key(mtm, "Save", action, "s"),
        ("Save As", "S") => item_with_key(mtm, "Save As", action, "S"),
        ("Print", "p") => item_with_key(mtm, "Print", action, "p"),
        ("Toggle Mode", "/") => item_with_key(mtm, "Toggle Mode", action, "/"),
        ("Close", "w") => item_with_key(mtm, "Close", action, "w"),
        _ => item_with_key(mtm, "Exit", action, "q"),
    };
    unsafe { item.setTarget(Some(target)) };
    item.setKeyEquivalentModifierMask(modifiers);
    item
}

fn item_with_key(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<objc2::runtime::Sel>,
    key: &str,
) -> objc2::rc::Retained<NSMenuItem> {
    match (title, key) {
        ("Open", "o") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Open"),
                action,
                ns_string!("o"),
            )
        },
        ("Save", "s") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Save"),
                action,
                ns_string!("s"),
            )
        },
        ("Save As", "S") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Save As"),
                action,
                ns_string!("S"),
            )
        },
        ("Print", "p") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Print"),
                action,
                ns_string!("p"),
            )
        },
        ("Toggle Mode", "/") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Toggle Mode"),
                action,
                ns_string!("/"),
            )
        },
        ("Close", "w") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Close"),
                action,
                ns_string!("w"),
            )
        },
        _ => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Exit"),
                action,
                ns_string!("q"),
            )
        },
    }
}
