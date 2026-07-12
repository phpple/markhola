use chrono::Local;

pub(crate) fn current_date_stamp() -> Option<String> {
    Some(Local::now().format("%Y%m%d").to_string())
}

pub(crate) fn current_timestamp() -> Option<String> {
    Some(Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string())
}
