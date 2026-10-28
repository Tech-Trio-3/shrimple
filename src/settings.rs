//! Persistent settings, stored in the current user's registry hive.

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_DWORD, REG_SZ,
    RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
};

const APP_KEY: &str = r"Software\Shrimple";
const INTERVAL_VALUE: &str = "IntervalMinutes";
const POP_OUT_VALUE: &str = "PopOut";
const ALWAYS_SHOWN_VALUE: &str = "PopOutAlways";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "Shrimple";

pub const DEFAULT_INTERVAL_MINUTES: u32 = 30;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn interval_minutes() -> u32 {
    read_dword(INTERVAL_VALUE)
        .filter(|v| (1..=24 * 60).contains(v))
        .unwrap_or(DEFAULT_INTERVAL_MINUTES)
}

pub fn set_interval_minutes(minutes: u32) {
    write_dword(INTERVAL_VALUE, minutes);
}

/// Whether the shrimp climbs out of the tray onto the screen when ignored (on by default).
pub fn pops_out() -> bool {
    read_dword(POP_OUT_VALUE) != Some(0)
}

pub fn set_pops_out(enabled: bool) {
    write_dword(POP_OUT_VALUE, enabled as u32);
}

/// Whether the big shrimp stays on screen all the time, not just when ignored (off by default).
pub fn always_shown() -> bool {
    read_dword(ALWAYS_SHOWN_VALUE).is_some_and(|v| v != 0)
}

pub fn set_always_shown(enabled: bool) {
    write_dword(ALWAYS_SHOWN_VALUE, enabled as u32);
}

fn read_dword(name: &str) -> Option<u32> {
    let mut value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide(APP_KEY).as_ptr(),
            wide(name).as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut value as *mut u32 as *mut _,
            &mut size,
        )
    };
    (status == ERROR_SUCCESS).then_some(value)
}

fn write_dword(name: &str, value: u32) {
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            wide(APP_KEY).as_ptr(),
            wide(name).as_ptr(),
            REG_DWORD,
            &value as *const u32 as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

/// Whether Shrimple is registered to launch when you sign in to Windows.
pub fn starts_with_windows() -> bool {
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            wide(RUN_VALUE).as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    status == ERROR_SUCCESS
}

pub fn set_starts_with_windows(enabled: bool) {
    let key = wide(RUN_KEY);
    let name = wide(RUN_VALUE);
    unsafe {
        if !enabled {
            RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr());
            return;
        }
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let command = wide(&format!("\"{}\"", exe.display()));
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            REG_SZ,
            command.as_ptr() as *const _,
            (command.len() * 2) as u32,
        );
    }
}
