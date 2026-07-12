#[cfg(target_os = "macos")]
#[path = "../implementation/pdf_export.rs"]
mod implementation;
#[cfg(not(target_os = "macos"))]
#[path = "../implementation/pdf_export_stub.rs"]
mod implementation;
#[cfg(all(test, target_os = "macos"))]
#[path = "../tests/pdf_export.rs"]
mod tests;

pub use self::implementation::*;
