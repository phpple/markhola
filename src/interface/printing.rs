#[cfg(target_os = "macos")]
#[path = "../implementation/printing.rs"]
mod implementation;
#[cfg(not(target_os = "macos"))]
#[path = "../implementation/printing_stub.rs"]
mod implementation;

pub use self::implementation::*;
