use lopdf::Document as LoDocument;
use tao::dpi::LogicalSize;
use tao::event_loop::EventLoopBuilder;
use tao::window::{Window, WindowBuilder};

use crate::app::log_event;
use crate::document::ActiveDocument;
use crate::file_io;
use crate::pdf_export::{export_markdown_file_to_path, prepare_print_webview};

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

pub fn smoke_count_markdown_file_print_pages(input_path: &std::path::Path) -> Result<usize, String> {
    let input_path = std::fs::canonicalize(input_path)
        .map_err(|error| format!("Failed to canonicalize input path: {error}"))?;
    let temp_pdf = std::env::temp_dir().join(format!(
        "markhola-print-pages-{}.pdf",
        std::process::id()
    ));
    if temp_pdf.exists() {
        let _ = std::fs::remove_file(&temp_pdf);
    }

    export_markdown_file_to_path(&input_path, &temp_pdf)?;
    let pdf =
        LoDocument::load(&temp_pdf).map_err(|error| format!("Failed to read generated PDF: {error}"))?;
    let _ = std::fs::remove_file(&temp_pdf);
    let page_count = pdf.get_pages().len();

    log_event(
        "printing.smoke.pages",
        None,
        "counted Windows printable content pages for smoke validation",
        format!("path={} pages={page_count}", input_path.display()),
    );
    Ok(page_count)
}
