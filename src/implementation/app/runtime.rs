use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tao::event_loop::EventLoopProxy;
use tao::keyboard::ModifiersState;
use tao::window::Window;
use wry::WebView;

use crate::workspace::DocumentWorkspace;

use super::{OpenPathRequest, UserEvent};
use super::asset_access::AssetAccessRegistry;

pub(super) struct ShellRuntime {
    pub(super) ready: bool,
    pub(super) recovery_pending: bool,
    pub(super) pending_open_requests: Vec<OpenPathRequest>,
    pub(super) suppress_blank_recovery: Arc<AtomicBool>,
}

impl ShellRuntime {
    pub(super) fn new(suppress_blank_recovery: Arc<AtomicBool>) -> Self {
        Self {
            ready: false,
            recovery_pending: false,
            pending_open_requests: Vec::new(),
            suppress_blank_recovery,
        }
    }
}

pub(super) struct AppRuntime {
    pub(super) proxy: EventLoopProxy<UserEvent>,
    pub(super) window: Window,
    pub(super) webview: WebView,
    pub(super) workspace: DocumentWorkspace,
    pub(super) modifiers: ModifiersState,
    pub(super) shell: ShellRuntime,
    pub(super) asset_access: AssetAccessRegistry,
}

impl AppRuntime {
    pub(super) fn new(
        proxy: EventLoopProxy<UserEvent>,
        window: Window,
        webview: WebView,
        suppress_blank_recovery: Arc<AtomicBool>,
        asset_access: AssetAccessRegistry,
    ) -> Self {
        Self {
            proxy,
            window,
            webview,
            workspace: DocumentWorkspace::new(),
            modifiers: ModifiersState::default(),
            shell: ShellRuntime::new(suppress_blank_recovery),
            asset_access,
        }
    }
}
