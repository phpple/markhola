use std::error::Error;

use objc2::MainThreadOnly;
use objc2_app_kit::{NSApp, NSApplication, NSMenu};
use objc2_foundation::MainThreadMarker;
use objc2_foundation::ns_string;
use tao::event_loop::EventLoopProxy;

use crate::app::UserEvent;

use super::menu_app::add_app_menu;
use super::menu_edit::add_edit_menu;
use super::menu_file::add_file_menu;
use super::menu_help::add_help_menu;
use super::menu_tab::add_tab_menu;
use super::menu_target::target_ref;
use super::menu_view::add_view_menu;

pub fn install(proxy: &EventLoopProxy<UserEvent>) -> Result<(), Box<dyn Error>> {
    let mtm = MainThreadMarker::new().ok_or("menu setup must run on main thread")?;
    let app = NSApplication::sharedApplication(mtm);
    let target = target_ref(mtm, proxy.clone());
    let main_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("MainMenu"));

    add_app_menu(mtm, &main_menu, target);
    add_file_menu(mtm, &main_menu, target);
    add_edit_menu(mtm, &main_menu, target);
    add_tab_menu(mtm, &main_menu, target);
    add_view_menu(mtm, &main_menu, target);
    add_help_menu(mtm, &main_menu, target);

    app.setMainMenu(Some(&main_menu));
    let _ = NSApp(mtm);
    Ok(())
}
