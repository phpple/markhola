#[path = "../implementation/workspace.rs"]
mod implementation;
#[cfg(test)]
#[path = "../tests/workspace.rs"]
mod tests;

pub use self::implementation::*;
