use std::ffi::{CStr, c_char, c_long};
use std::time::{SystemTime, UNIX_EPOCH};

#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

unsafe extern "C" {
    fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
    fn strftime(s: *mut c_char, max: usize, format: *const c_char, tm: *const Tm) -> usize;
}

pub(crate) fn current_date_stamp() -> Option<String> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let mut tm = zeroed_tm();
    unsafe {
        if localtime_r(&seconds, &mut tm).is_null() {
            return None;
        }
        format_tm(&tm, b"%Y%m%d\0", 16)
    }
}

pub(crate) fn current_timestamp() -> Option<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let seconds = now.as_secs() as i64;
    let millis = now.subsec_millis();
    let mut tm = zeroed_tm();
    unsafe {
        if localtime_r(&seconds, &mut tm).is_null() {
            return None;
        }
        let base = format_tm(&tm, b"%Y-%m-%dT%H:%M:%S\0", 32)?;
        Some(format!("{base}.{millis:03}"))
    }
}

unsafe fn format_tm(tm: &Tm, format: &[u8], len: usize) -> Option<String> {
    let mut buffer = vec![0 as c_char; len];
    let written = unsafe {
        strftime(
            buffer.as_mut_ptr(),
            buffer.len(),
            format.as_ptr().cast(),
            tm,
        )
    };
    if written == 0 {
        return None;
    }
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_str()
        .ok()
        .map(ToOwned::to_owned)
}

fn zeroed_tm() -> Tm {
    Tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    }
}
