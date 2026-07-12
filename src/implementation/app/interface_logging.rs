use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;

use super::current_date_stamp;
use super::interface_clock::current_timestamp;
use super::interface_constants::{DEBUG_LOG_DIR, DEBUG_LOG_FALLBACK_PATH};

pub(crate) fn primary_debug_log_path() -> Option<PathBuf> {
    let date = current_date_stamp()?;
    Some(Path::new(DEBUG_LOG_DIR).join(format!("markholo-{date}.log")))
}

pub(crate) fn append_log_line(path: &Path, line: &str) -> bool {
    if let Some(parent) = path.parent() {
        if create_dir_all(parent).is_err() {
            return false;
        }
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        return file.write_all(line.as_bytes()).is_ok();
    }
    false
}

pub(crate) fn debug_log(message: impl AsRef<str>) {
    let ts = current_timestamp().unwrap_or_else(|| "unknown-ts".to_string());
    let pid = std::process::id();
    let tid = thread::current().name().unwrap_or("unnamed").to_string();
    let line = format!("ts={ts} pid={pid} tid={tid} {}\n", message.as_ref());
    eprint!("{line}");

    let wrote_primary = primary_debug_log_path()
        .as_deref()
        .map(|path| append_log_line(path, &line))
        .unwrap_or(false);
    if !wrote_primary {
        let fallback_notice = format!(
            "ts={ts} pid={pid} tid={tid} stage=logger event_id=system msg=\"primary log path unavailable\" primary_path={} fallback_path={}\n",
            primary_debug_log_path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<none>".to_string()),
            DEBUG_LOG_FALLBACK_PATH,
        );
        let _ = append_log_line(Path::new(DEBUG_LOG_FALLBACK_PATH), &fallback_notice);
        let _ = append_log_line(Path::new(DEBUG_LOG_FALLBACK_PATH), &line);
    }
}

pub(crate) fn log_event(stage: &str, event_id: Option<u64>, message: &str, extra: impl AsRef<str>) {
    let event_id = event_id
        .map(|id| format!("open-{id}"))
        .unwrap_or_else(|| "system".to_string());
    let extra = extra.as_ref();
    if extra.is_empty() {
        debug_log(format!(
            "stage={stage} event_id={event_id} msg=\"{message}\""
        ));
    } else {
        debug_log(format!(
            "stage={stage} event_id={event_id} msg=\"{message}\" {extra}"
        ));
    }
}
