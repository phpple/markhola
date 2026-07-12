#[path = "../implementation/app/implementation.rs"]
mod implementation;
#[path = "../implementation/app/interface.rs"]
mod interface;
#[cfg(target_os = "macos")]
#[path = "../implementation/app/macos_menu.rs"]
mod macos_menu;
#[path = "../implementation/app/shell.rs"]
mod shell;
#[cfg(test)]
#[path = "../tests/app.rs"]
mod tests;

pub use self::implementation::run;
pub(crate) use self::interface::*;
