use tao::dpi::LogicalSize;
use tao::event_loop::EventLoopBuilder;
use tao::window::{Window, WindowBuilder};

use crate::app::log_event;
use crate::document::ActiveDocument;
use crate::file_io;
use crate::pdf_export::prepare_print_webview;

pub fn print_document(window: &Window, document: &ActiveDocument) -> Result<PrintOutcome, String> {
    log_event(
        "printing.begin",
        None,
        "starting print flow",
        format!(
            "path={} dirty={} mode={:?}",
            document.file_path().display(),
            document.is_dirty(),
            document.mode()
        ),
    );

    let prepared = prepare_print_webview(window, document)?;
    prepared
        .webview
        .print()
        .map_err(|error| format!("Failed to open print dialog: {error}"))?;
    log_event(
        "printing.operation.end",
        None,
        "started Windows print flow",
        format!("preparation_mode={:?}", prepared.preparation_mode),
    );
    Ok(PrintOutcome::Started)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrintOutcome {
    Started,
    Cancelled,
}

pub fn smoke_prepare_markdown_file_for_print(
    input_path: &std::path::Path,
) -> Result<(), String> {
    let input_path = std::fs::canonicalize(input_path)
        .map_err(|error| format!("Failed to canonicalize input path: {error}"))?;
    let markdown = file_io::load_markdown(&input_path)?;
    let base_url = file_io::directory_base_url(&input_path)?;
    let document = ActiveDocument::open_with_id(1, input_path.clone(), markdown, base_url);

    let event_loop = EventLoopBuilder::<()>::new().build();
    let host_window = WindowBuilder::new()
        .with_visible(false)
        .with_title("MarkHola Print Prepare")
        .with_inner_size(LogicalSize::new(32.0, 32.0))
        .build(&event_loop)
        .map_err(|error| format!("Failed to create temporary print window: {error}"))?;

    let prepared = prepare_print_webview(&host_window, &document)?;
    log_event(
        "printing.smoke",
        None,
        "prepared Windows print webview for smoke validation",
        format!(
            "path={} preparation_mode={:?}",
            input_path.display(),
            prepared.preparation_mode
        ),
    );
    Ok(())
}
