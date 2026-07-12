use std::path::{Path, PathBuf};

use crate::document::ActiveDocument;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdfExportOutcome {
    Exported(PathBuf),
    Cancelled,
}

pub fn export_document(_document: &ActiveDocument) -> Result<PdfExportOutcome, String> {
    Err("PDF export is currently available on macOS only.".to_string())
}

pub fn export_markdown_file_to_path(
    _input_path: &Path,
    _output_path: &Path,
) -> Result<(), String> {
    Err("PDF export smoke validation is currently available on macOS only.".to_string())
}
