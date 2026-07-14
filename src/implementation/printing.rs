use objc2::MainThreadMarker;
use objc2_app_kit::{NSApp, NSPrintInfo};
use tao::window::Window;

use crate::app::log_event;
use crate::document::ActiveDocument;
use crate::file_io;
use crate::pdf_export::{
    prepare_printable_webview, prepare_printable_webview_with_measurement,
    printable_page_count_for_height,
};

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

    let print_info = NSPrintInfo::sharedPrintInfo();
    let mtm = MainThreadMarker::new().ok_or("Print preview must run on the main thread.")?;
    let app = NSApp(mtm);
    let host_window = app
        .keyWindow()
        .or_else(|| app.mainWindow())
        .ok_or("No active macOS window available for the print panel.")?;
    let webview = prepare_printable_webview(document)?;
    let print_operation = unsafe { webview.printOperationWithPrintInfo(&print_info) };

    log_event(
        "printing.operation.begin",
        None,
        "starting NSPrintOperation",
        "host_window=active".to_string(),
    );
    print_operation.setCanSpawnSeparateThread(true);
    print_operation.setShowsPrintPanel(true);
    print_operation.setShowsProgressPanel(true);
    unsafe {
        print_operation.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
            &host_window,
            None,
            None,
            std::ptr::null_mut(),
        );
    }
    log_event(
        "printing.operation.end",
        None,
        "finished NSPrintOperation",
        "did_run=true output=pdfkit-view modal=window",
    );

    Ok(PrintOutcome::Started)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrintOutcome {
    Started,
}

pub fn smoke_prepare_markdown_file_for_print(input_path: &std::path::Path) -> Result<(), String> {
    let input_path = std::fs::canonicalize(input_path)
        .map_err(|error| format!("Failed to canonicalize input path: {error}"))?;
    let markdown = file_io::load_markdown(&input_path)?;
    let base_url = file_io::directory_base_url(&input_path)?;
    let document = ActiveDocument::open_with_id(1, input_path.clone(), markdown, base_url);

    let _webview = prepare_printable_webview(&document)?;
    log_event(
        "printing.smoke",
        None,
        "prepared print preview WKWebView for smoke validation",
        format!("path={} output=webkit-view", input_path.display()),
    );
    Ok(())
}

pub fn smoke_count_markdown_file_print_pages(input_path: &std::path::Path) -> Result<usize, String> {
    let input_path = std::fs::canonicalize(input_path)
        .map_err(|error| format!("Failed to canonicalize input path: {error}"))?;
    let markdown = file_io::load_markdown(&input_path)?;
    let base_url = file_io::directory_base_url(&input_path)?;
    let document = ActiveDocument::open_with_id(1, input_path.clone(), markdown, base_url);
    let (_webview, measurement) = prepare_printable_webview_with_measurement(&document)?;
    let page_count = printable_page_count_for_height(measurement.height);

    log_event(
        "printing.smoke.pages",
        None,
        "prepared printable content page count for smoke validation",
        format!(
            "path={} height={} pages={}",
            input_path.display(),
            measurement.height,
            page_count
        ),
    );

    Ok(page_count)
}
