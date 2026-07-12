use crate::document::ActiveDocument;

pub fn print_document(_document: &ActiveDocument) -> Result<PrintOutcome, String> {
    Err("Printing is currently available on macOS only.".to_string())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrintOutcome {
    Started,
    Cancelled,
}

pub fn smoke_prepare_markdown_file_for_print(
    _input_path: &std::path::Path,
) -> Result<(), String> {
    Err("Print smoke validation is currently available on macOS only.".to_string())
}
