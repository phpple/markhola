#[path = "../implementation/app/implementation.rs"]
mod implementation;
#[path = "../implementation/app/interface.rs"]
mod interface;
#[cfg(target_os = "macos")]
#[path = "../implementation/app/macos_menu.rs"]
mod macos_menu;
#[cfg(target_os = "windows")]
#[path = "../implementation/app/windows_menu.rs"]
mod windows_menu;
pub(crate) use self::implementation::platform;
#[path = "../implementation/app/shell.rs"]
mod shell;
#[cfg(test)]
#[path = "../tests/app.rs"]
mod tests;

pub use self::implementation::run;
pub(crate) use self::interface::*;
