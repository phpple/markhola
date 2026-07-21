mod asset_access;
mod bootstrap;
mod close_actions;
mod document_actions;
mod documentation;
mod event_loop;
mod export_actions;
mod ipc;
mod navigation_actions;
mod runtime;
mod save_actions;
mod shell_events;
mod shortcuts;
mod user_events;
mod window_events;
mod workspace_view;

#[allow(unused_imports)]
pub(crate) use self::document_actions::{load_document, reload_workspace_documents_from_disk};

use super::*;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();
    log_event(
        "app.start",
        None,
        "app run started",
        format!("version={APP_VERSION} platform={APP_BUILD_PLATFORM}/{APP_BUILD_TARGET}"),
    );

    let (event_loop, mut runtime) = bootstrap::build_runtime()?;
    workspace_view::sync_native_menu_state(&runtime.workspace);

    event_loop.run(move |event, _, control_flow| {
        event_loop::handle_event(event, &mut runtime, control_flow);
    });

    #[allow(unreachable_code)]
    Ok(())
}
