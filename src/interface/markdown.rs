#[path = "../implementation/markdown.rs"]
mod implementation;
#[cfg(test)]
#[path = "../tests/markdown.rs"]
mod tests;

pub use self::implementation::*;
