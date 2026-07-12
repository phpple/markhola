use std::sync::Once;
use std::sync::atomic::AtomicU64;

pub(crate) const WINDOW_TITLE: &str = "MarkHola";
pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const APP_AUTHOR: &str = "Ronnie Deng";
pub(crate) const APP_GITHUB_URL: &str = "https://github.com/phpple/markhola";
pub(crate) const APP_BUILD_TARGET: &str = std::env::consts::ARCH;
pub(crate) const APP_BUILD_PLATFORM: &str = std::env::consts::OS;
pub(crate) const DEBUG_LOG_DIR: &str = "/var/log/markhola";
pub(crate) const DEBUG_LOG_FALLBACK_PATH: &str = "/tmp/markhola.log";
pub(crate) static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);
pub(crate) static PANIC_HOOK_ONCE: Once = Once::new();
