use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadOnly, define_class};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol};
use tao::event_loop::EventLoopProxy;

use crate::app::{UserEvent, dispatch_user_event, log_event, new_action_context};

#[derive(Debug)]
struct ProxyIvars {
    proxy: EventLoopProxy<UserEvent>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ProxyIvars]
    struct MenuTarget;

unsafe impl NSObjectProtocol for MenuTarget {}

    impl MenuTarget {
        #[unsafe(method(newMenuDocument:))]
        fn new_menu_document(&self, _sender: Option<&AnyObject>) {
            emit(
                &self.ivars().proxy,
                UserEvent::NewDocument,
                "newMenuDocument:",
            );
        }

        #[unsafe(method(openMenuDocument:))]
        fn open_menu_document(&self, _sender: Option<&AnyObject>) {
            let ctx = new_action_context("macos-menu-open");
            log_event("macos.menu.action", Some(ctx.event_id), "macOS menu action openMenuDocument:", "");
            dispatch_user_event(&self.ivars().proxy, "macos-menu", UserEvent::OpenFile(ctx));
        }

        #[unsafe(method(saveMenuDocument:))]
        fn save_menu_document(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::SaveDocument, "saveMenuDocument:"); }
        #[unsafe(method(saveMenuDocumentAs:))]
        fn save_menu_document_as(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::SaveDocumentAs, "saveMenuDocumentAs:"); }
        #[unsafe(method(exportPdfDocument:))]
        fn export_pdf_document(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::ExportPdf, "exportPdfDocument:"); }
        #[unsafe(method(exportHtmlDocument:))]
        fn export_html_document(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::ExportHtml, "exportHtmlDocument:"); }
        #[unsafe(method(printDocument:))]
        fn print_document(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::PrintDocument, "printDocument:"); }
        #[unsafe(method(openFindPanel:))]
        fn open_find_panel(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::OpenFind, "openFindPanel:"); }
        #[unsafe(method(toggleDocumentMode:))]
        fn toggle_document_mode(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::ToggleMode, "toggleDocumentMode:"); }
        #[unsafe(method(closeCurrentDocument:))]
        fn close_current_document(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::CloseCurrentDocument, "closeCurrentDocument:"); }
        #[unsafe(method(activateNextDocument:))]
        fn activate_next_document(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::ActivateNextDocument, "activateNextDocument:"); }
        #[unsafe(method(activatePreviousDocument:))]
        fn activate_previous_document(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::ActivatePreviousDocument, "activatePreviousDocument:"); }
        #[unsafe(method(closeOtherDocuments:))]
        fn close_other_documents(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::CloseOtherDocuments, "closeOtherDocuments:"); }
        #[unsafe(method(closeAllDocuments:))]
        fn close_all_documents(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::CloseAllDocuments, "closeAllDocuments:"); }
        #[unsafe(method(showAboutPanel:))]
        fn show_about_panel(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::ShowAbout, "showAboutPanel:"); }
        #[unsafe(method(openDocumentation:))]
        fn open_documentation(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::OpenDocumentation, "openDocumentation:"); }
        #[unsafe(method(exitApplication:))]
        fn exit_application(&self, _sender: Option<&AnyObject>) { emit(&self.ivars().proxy, UserEvent::Exit, "exitApplication:"); }
    }
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker, proxy: EventLoopProxy<UserEvent>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ProxyIvars { proxy });
        unsafe { objc2::msg_send![super(this), init] }
    }
}

pub(super) fn target_ref(
    mtm: MainThreadMarker,
    proxy: EventLoopProxy<UserEvent>,
) -> &'static AnyObject {
    let target = Box::leak(Box::new(MenuTarget::new(mtm, proxy)));
    (&**target).as_ref()
}

fn emit(proxy: &EventLoopProxy<UserEvent>, event: UserEvent, action: &str) {
    log_event(
        "macos.menu.action",
        None,
        &format!("macOS menu action {action}"),
        "",
    );
    dispatch_user_event(proxy, "macos-menu", event);
}
