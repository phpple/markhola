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
    let item = item_with_key(mtm, title, action, key);
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
        ("New", "n") => unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("New"),
                action,
                ns_string!("n"),
            )
        },
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
                match title {
                    "Exit" => ns_string!("Exit"),
                    "Toggle Full Screen" => ns_string!("Toggle Full Screen"),
                    "Default" => ns_string!("Default"),
                    "GitHub" => ns_string!("GitHub"),
                    "Dark" => ns_string!("Dark"),
                    "Light" => ns_string!("Light"),
                    _ => ns_string!(""),
                },
                action,
                match key {
                    "q" => ns_string!("q"),
                    "f" => ns_string!("f"),
                    _ => ns_string!(""),
                },
            )
        },
    }
}
