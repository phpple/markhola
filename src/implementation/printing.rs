use objc2_app_kit::NSPrintInfo;
use tao::window::Window;

use crate::app::log_event;
use crate::document::ActiveDocument;
use crate::file_io;
use crate::pdf_export::prepare_print_webview;

pub fn print_document(_window: &Window, document: &ActiveDocument) -> Result<PrintOutcome, String> {
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

    let prepared = prepare_print_webview(_window, document)?;
    let print_info = NSPrintInfo::sharedPrintInfo();
    let operation = unsafe { prepared.webview.printOperationWithPrintInfo(&print_info) };

    log_event(
        "printing.operation.begin",
        None,
        "starting NSPrintOperation",
        "",
    );
    let did_run = operation.runOperation();
    log_event(
        "printing.operation.end",
        None,
        "finished NSPrintOperation",
        format!(
            "did_run={did_run} preparation_mode={:?}",
            prepared.preparation_mode
        ),
    );

    if did_run {
        Ok(PrintOutcome::Started)
    } else {
        Ok(PrintOutcome::Cancelled)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrintOutcome {
    Started,
    Cancelled,
}

pub fn smoke_prepare_markdown_file_for_print(input_path: &std::path::Path) -> Result<(), String> {
    let input_path = std::fs::canonicalize(input_path)
        .map_err(|error| format!("Failed to canonicalize input path: {error}"))?;
    let markdown = file_io::load_markdown(&input_path)?;
    let base_url = file_io::directory_base_url(&input_path)?;
    let document = ActiveDocument::open_with_id(1, input_path.clone(), markdown, base_url);

    let event_loop = tao::event_loop::EventLoopBuilder::<()>::new().build();
    let host_window = tao::window::WindowBuilder::new()
        .with_visible(false)
        .with_title("MarkHola Print Prepare")
        .with_inner_size(tao::dpi::LogicalSize::new(32.0, 32.0))
        .build(&event_loop)
        .map_err(|error| format!("Failed to create temporary print window: {error}"))?;

    let prepared = prepare_print_webview(&host_window, &document)?;
    log_event(
        "printing.smoke",
        None,
        "prepared print webview for smoke validation",
        format!(
            "path={} preparation_mode={:?}",
            input_path.display(),
            prepared.preparation_mode
        ),
    );
    Ok(())
}
