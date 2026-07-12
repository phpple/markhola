#[path = "../implementation/file_io.rs"]
mod implementation;
#[cfg(test)]
#[path = "../tests/file_io.rs"]
mod tests;

pub use self::implementation::*;
