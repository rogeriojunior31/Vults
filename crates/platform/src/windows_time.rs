//! The local UTC offset on Windows, from the system's time zone rules.

use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::System::Time::{
    FileTimeToSystemTime, SystemTimeToFileTime, SystemTimeToTzSpecificLocalTime,
};

/// The user's locale name (`pt-BR`).
pub fn locale() -> Option<String> {
    let mut name = [0u16; 85];
    // SAFETY: the buffer is a live local of the length passed.
    let len = unsafe { GetUserDefaultLocaleName(&mut name) };
    let len = usize::try_from(len).ok().filter(|l| *l > 1)?;
    String::from_utf16(&name[..len - 1]).ok()
}

/// 100 ns ticks between 1601-01-01 (FILETIME's epoch) and 1970-01-01.
const EPOCH_GAP: i64 = 11_644_473_600;
const TICKS: i64 = 10_000_000;

fn filetime(ticks: i64) -> FILETIME {
    let t = ticks as u64;
    FILETIME {
        dwLowDateTime: t as u32,
        dwHighDateTime: (t >> 32) as u32,
    }
}

fn ticks(ft: FILETIME) -> i64 {
    ((u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)) as i64
}

pub fn utc_offset(unix: i64) -> Option<i32> {
    let utc_ticks = unix.checked_add(EPOCH_GAP)?.checked_mul(TICKS)?;
    let utc_ft = filetime(utc_ticks);
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    let mut local_ft = FILETIME::default();
    // SAFETY: every pointer is to a live local of the right type, for the length of the call.
    unsafe {
        FileTimeToSystemTime(&utc_ft, &mut utc).ok()?;
        SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).ok()?;
        SystemTimeToFileTime(&local, &mut local_ft).ok()?;
    }
    i32::try_from((ticks(local_ft) - utc_ticks) / TICKS).ok()
}
