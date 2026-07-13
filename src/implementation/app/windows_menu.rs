use std::cell::RefCell;
use std::error::Error;

use tao::event_loop::EventLoopProxy;
use tao::platform::windows::WindowExtWindows;
use tao::window::Window;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateMenu, DrawMenuBar, EnableMenuItem, GetMenu, HMENU, MF_BYCOMMAND,
    MF_DISABLED, MF_ENABLED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, SetMenu,
    WM_COMMAND,
};

use crate::workspace::DocumentWorkspace;

use super::{EditCommand, UserEvent, dispatch_user_event, log_event, new_action_context};

const ID_OPEN: u16 = 1001;
const ID_SAVE: u16 = 1002;
const ID_SAVE_AS: u16 = 1003;
const ID_PRINT: u16 = 1004;
const ID_EXPORT_PDF: u16 = 1005;
const ID_EXPORT_HTML: u16 = 1006;
const ID_TOGGLE_MODE: u16 = 1007;
const ID_CLOSE_TAB: u16 = 1008;
const ID_EXIT: u16 = 1009;
const ID_UNDO: u16 = 1101;
const ID_REDO: u16 = 1102;
const ID_FIND: u16 = 1103;
const ID_CUT: u16 = 1104;
const ID_COPY: u16 = 1105;
const ID_PASTE: u16 = 1106;
const ID_SELECT_ALL: u16 = 1107;
const ID_NEXT_TAB: u16 = 1201;
const ID_PREVIOUS_TAB: u16 = 1202;
const ID_CLOSE_OTHER_TABS: u16 = 1203;
const ID_CLOSE_ALL_TABS: u16 = 1204;
const ID_DOCUMENTATION: u16 = 1301;
const ID_ABOUT: u16 = 1302;

thread_local! {
    static MENU_PROXY: RefCell<Option<EventLoopProxy<UserEvent>>> = const { RefCell::new(None) };
    static MENU_HWND: RefCell<Option<HWND>> = const { RefCell::new(None) };
}

pub fn install(window: &Window, proxy: EventLoopProxy<UserEvent>) -> Result<(), Box<dyn Error>> {
    MENU_PROXY.with(|slot| *slot.borrow_mut() = Some(proxy));
    MENU_HWND.with(|slot| *slot.borrow_mut() = Some(HWND(window.hwnd() as _)));

    unsafe {
        let main_menu = CreateMenu()?;
        let file_menu = CreateMenu()?;
        let edit_menu = CreateMenu()?;
        let tab_menu = CreateMenu()?;
        let help_menu = CreateMenu()?;

        append_item(file_menu, ID_OPEN, "&Open\tCtrl+O")?;
        append_item(file_menu, ID_SAVE, "&Save\tCtrl+S")?;
        append_item(file_menu, ID_SAVE_AS, "Save &As")?;
        append_item(file_menu, ID_PRINT, "&Print\tCtrl+P")?;
        append_separator(file_menu)?;
        append_item(file_menu, ID_EXPORT_PDF, "Export &PDF")?;
        append_item(file_menu, ID_EXPORT_HTML, "Export &HTML")?;
        append_separator(file_menu)?;
        append_item(file_menu, ID_TOGGLE_MODE, "&Toggle Mode\tCtrl+/")?;
        append_separator(file_menu)?;
        append_item(file_menu, ID_CLOSE_TAB, "&Close\tCtrl+W")?;
        append_item(file_menu, ID_EXIT, "E&xit")?;

        append_item(edit_menu, ID_UNDO, "&Undo")?;
        append_item(edit_menu, ID_REDO, "&Redo")?;
        append_separator(edit_menu)?;
        append_item(edit_menu, ID_FIND, "&Find\tCtrl+F")?;
        append_separator(edit_menu)?;
        append_item(edit_menu, ID_CUT, "Cu&t")?;
        append_item(edit_menu, ID_COPY, "&Copy")?;
        append_item(edit_menu, ID_PASTE, "&Paste")?;
        append_separator(edit_menu)?;
        append_item(edit_menu, ID_SELECT_ALL, "Select &All")?;

        append_item(tab_menu, ID_NEXT_TAB, "&Next Tab")?;
        append_item(tab_menu, ID_PREVIOUS_TAB, "&Previous Tab")?;
        append_separator(tab_menu)?;
        append_item(tab_menu, ID_CLOSE_TAB, "Close &Tab\tCtrl+W")?;
        append_item(tab_menu, ID_CLOSE_OTHER_TABS, "Close &Other Tabs")?;
        append_item(tab_menu, ID_CLOSE_ALL_TABS, "Close A&ll Tabs")?;

        append_item(help_menu, ID_DOCUMENTATION, "&Documentation")?;
        append_item(help_menu, ID_ABOUT, "&About")?;

        append_submenu(main_menu, file_menu, "&File")?;
        append_submenu(main_menu, edit_menu, "&Edit")?;
        append_submenu(main_menu, tab_menu, "&Tab")?;
        append_submenu(main_menu, help_menu, "&Help")?;

        let hwnd = HWND(window.hwnd() as _);
        SetMenu(hwnd, Some(main_menu))?;
        let _ = DrawMenuBar(hwnd);
    }

    Ok(())
}

pub fn handle_msg_hook(msg: *const std::ffi::c_void) -> bool {
    let msg = unsafe { &*(msg as *const MSG) };
    if msg.message != WM_COMMAND {
        return false;
    }

    let command_id = (msg.wParam.0 & 0xffff) as u16;
    MENU_PROXY.with(|slot| {
        let proxy = slot.borrow().clone();
        let Some(proxy) = proxy.as_ref() else {
            return;
        };
        dispatch_menu_command(proxy, command_id);
    });
    false
}

pub fn sync_workspace_state(workspace: &DocumentWorkspace) {
    let has_document = workspace.active_document().is_some();
    let has_multiple_documents = workspace.document_count() > 1;
    let enabled = if has_document { MF_ENABLED } else { MF_DISABLED | MF_GRAYED };
    let tab_enabled = if has_multiple_documents {
        MF_ENABLED
    } else {
        MF_DISABLED | MF_GRAYED
    };

    unsafe {
        MENU_HWND.with(|slot| {
            let hwnd = (*slot.borrow()).unwrap_or(HWND(std::ptr::null_mut()));
            if hwnd.0.is_null() {
                return;
            }
            let menu = GetMenu(hwnd);
            if menu.0.is_null() {
                return;
            }
            for id in [
                ID_SAVE,
                ID_SAVE_AS,
                ID_PRINT,
                ID_EXPORT_PDF,
                ID_EXPORT_HTML,
                ID_TOGGLE_MODE,
                ID_CLOSE_TAB,
                ID_UNDO,
                ID_REDO,
                ID_FIND,
                ID_CUT,
                ID_COPY,
                ID_PASTE,
                ID_SELECT_ALL,
            ] {
                let _ = EnableMenuItem(menu, id as u32, MF_BYCOMMAND | enabled);
            }
            for id in [ID_NEXT_TAB, ID_PREVIOUS_TAB, ID_CLOSE_OTHER_TABS, ID_CLOSE_ALL_TABS] {
                let _ = EnableMenuItem(menu, id as u32, MF_BYCOMMAND | tab_enabled);
            }
            let _ = DrawMenuBar(hwnd);
        });
    }
}

fn dispatch_menu_command(proxy: &EventLoopProxy<UserEvent>, command_id: u16) {
    match command_id {
        ID_OPEN => {
            let ctx = new_action_context("windows-menu-open");
            log_event(
                "windows.menu.action",
                Some(ctx.event_id),
                "Windows menu action Open",
                "",
            );
            dispatch_user_event(proxy, "windows-menu", UserEvent::OpenFile(ctx));
        }
        ID_SAVE => emit(proxy, UserEvent::SaveDocument, "Save"),
        ID_SAVE_AS => emit(proxy, UserEvent::SaveDocumentAs, "SaveAs"),
        ID_PRINT => emit(proxy, UserEvent::PrintDocument, "Print"),
        ID_EXPORT_PDF => emit(proxy, UserEvent::ExportPdf, "ExportPdf"),
        ID_EXPORT_HTML => emit(proxy, UserEvent::ExportHtml, "ExportHtml"),
        ID_TOGGLE_MODE => emit(proxy, UserEvent::ToggleMode, "ToggleMode"),
        ID_CLOSE_TAB => emit(proxy, UserEvent::CloseCurrentDocument, "CloseTab"),
        ID_EXIT => emit(proxy, UserEvent::Exit, "Exit"),
        ID_UNDO => emit(proxy, UserEvent::EditCommand(EditCommand::Undo), "Undo"),
        ID_REDO => emit(proxy, UserEvent::EditCommand(EditCommand::Redo), "Redo"),
        ID_FIND => emit(proxy, UserEvent::OpenFind, "Find"),
        ID_CUT => emit(proxy, UserEvent::EditCommand(EditCommand::Cut), "Cut"),
        ID_COPY => emit(proxy, UserEvent::EditCommand(EditCommand::Copy), "Copy"),
        ID_PASTE => emit(proxy, UserEvent::EditCommand(EditCommand::Paste), "Paste"),
        ID_SELECT_ALL => emit(
            proxy,
            UserEvent::EditCommand(EditCommand::SelectAll),
            "SelectAll",
        ),
        ID_NEXT_TAB => emit(proxy, UserEvent::ActivateNextDocument, "NextTab"),
        ID_PREVIOUS_TAB => emit(proxy, UserEvent::ActivatePreviousDocument, "PreviousTab"),
        ID_CLOSE_OTHER_TABS => emit(proxy, UserEvent::CloseOtherDocuments, "CloseOtherTabs"),
        ID_CLOSE_ALL_TABS => emit(proxy, UserEvent::CloseAllDocuments, "CloseAllTabs"),
        ID_DOCUMENTATION => emit(proxy, UserEvent::OpenDocumentation, "Documentation"),
        ID_ABOUT => emit(proxy, UserEvent::ShowAbout, "About"),
        _ => {}
    }
}

fn emit(proxy: &EventLoopProxy<UserEvent>, event: UserEvent, action: &str) {
    log_event(
        "windows.menu.action",
        None,
        &format!("Windows menu action {action}"),
        "",
    );
    dispatch_user_event(proxy, "windows-menu", event);
}

unsafe fn append_item(menu: HMENU, id: u16, title: &str) -> Result<(), windows::core::Error> {
    let title = wide(title);
    unsafe { AppendMenuW(menu, MF_STRING, id as usize, windows::core::PCWSTR(title.as_ptr()))? };
    Ok(())
}

unsafe fn append_separator(menu: HMENU) -> Result<(), windows::core::Error> {
    unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, windows::core::PCWSTR::null())? };
    Ok(())
}

unsafe fn append_submenu(
    menu: HMENU,
    submenu: HMENU,
    title: &str,
) -> Result<(), windows::core::Error> {
    let title = wide(title);
    unsafe {
        AppendMenuW(
            menu,
            MF_POPUP,
            submenu.0 as usize,
            windows::core::PCWSTR(title.as_ptr()),
        )?;
    }
    Ok(())
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
