//! Windows: lets the tray menu follow a dark system theme.
//!
//! Win32 menus draw light unless the app opts in. Windows has no public call for it, so this
//! takes `SetPreferredAppMode` from uxtheme by its ordinal, as Chromium and Firefox do.

use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
use windows::core::{PCSTR, w};

/// Windows 10 1809, the first build with ordinal 135. It takes a BOOL there and an app mode later,
/// and 1 means allow dark in both.
const FIRST_BUILD: u32 = 17763;
const SET_PREFERRED_APP_MODE: usize = 135;
const FLUSH_MENU_THEMES: usize = 136;
const ALLOW_DARK: i32 = 1;

fn build() -> u32 {
    let mut text = [0u16; 16];
    let mut size = size_of_val(&text) as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(text.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if read.is_err() {
        return 0;
    }
    let len = text.iter().position(|&c| c == 0).unwrap_or(text.len());
    String::from_utf16_lossy(&text[..len]).parse().unwrap_or(0)
}

/// Menus follow the system theme from now on. An older Windows keeps them light.
pub fn follow_system() {
    if build() < FIRST_BUILD {
        return;
    }
    unsafe {
        let Ok(lib) = LoadLibraryExW(w!("uxtheme.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) else {
            return;
        };
        if let Some(set) = GetProcAddress(lib, PCSTR(SET_PREFERRED_APP_MODE as *const u8)) {
            let set: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(set);
            set(ALLOW_DARK);
        }
        if let Some(flush) = GetProcAddress(lib, PCSTR(FLUSH_MENU_THEMES as *const u8)) {
            let flush: unsafe extern "system" fn() = std::mem::transmute(flush);
            flush();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_the_windows_build() {
        assert!(super::build() >= 10240);
    }
}
