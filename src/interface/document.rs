#[path = "../implementation/document/implementation.rs"]
mod implementation;
#[path = "../implementation/document/interface.rs"]
mod interface;
#[cfg(test)]
#[path = "../tests/document.rs"]
mod tests;

#[allow(unused_imports)]
pub use self::implementation::suggested_pdf_export_path;
pub use self::interface::{ActiveDocument, DocumentMode, DocumentSnapshot, DocumentTabSnapshot};
