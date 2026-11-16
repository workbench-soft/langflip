//! `langflip.exe --show`, the shortcuts' start: a copy already running opens its settings.
//!
//! The Run value and the sign in task start Langflip with no argument, so a sign in shows no window.

use windows::Win32::Foundation::{CloseHandle, HANDLE, LPARAM, WAIT_OBJECT_0, WPARAM};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForSingleObject};
use windows::Win32::UI::WindowsAndMessaging::{
    ASFW_ANY, AllowSetForegroundWindow, PostThreadMessageW,
};
use windows::core::HSTRING;

/// The event a second start signals. `Local` keeps it to this sign in, like the mutex.
const EVENT: &str = "Local\\LangflipShow";

/// The start came from a shortcut.
pub fn wanted(mut args: impl Iterator<Item = String>) -> bool {
    args.any(|a| a == "--show")
}

/// Tells the running copy to open its settings. True once the event is signalled.
pub fn ask_running() -> bool {
    // This copy got the front from Explorer, so the running copy may take it for the editor.
    let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    signal(EVENT)
}

/// Posts `message` to `thread` each time a second start asks, on a thread of its own.
pub fn listen(thread: u32, message: u32) {
    let Some(event) = open(EVENT) else { return };
    // A HANDLE holds a raw pointer, so it crosses to the thread as a number.
    let raw = event.0 as usize;
    std::thread::spawn(move || wait(HANDLE(raw as _), thread, message));
}

fn wait(event: HANDLE, thread: u32, message: u32) {
    while unsafe { WaitForSingleObject(event, INFINITE) } == WAIT_OBJECT_0 {
        let _ = unsafe { PostThreadMessageW(thread, message, WPARAM(0), LPARAM(0)) };
    }
}

/// Creates the event, or opens it when the other copy made it first. Auto reset, so one ask opens one editor.
fn open(name: &str) -> Option<HANDLE> {
    unsafe { CreateEventW(None, false, false, &HSTRING::from(name)) }.ok()
}

fn signal(name: &str) -> bool {
    let Some(event) = open(name) else {
        return false;
    };
    let done = unsafe { SetEvent(event) }.is_ok();
    let _ = unsafe { CloseHandle(event) };
    done
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{MSG, PM_REMOVE, PeekMessageW, WM_APP};

    #[test]
    fn only_the_show_argument_counts() {
        let args = |a: &[&str]| {
            a.iter()
                .map(|s| (*s).to_owned())
                .collect::<Vec<_>>()
                .into_iter()
        };
        assert!(wanted(args(&["langflip.exe", "--show"])));
        assert!(!wanted(args(&["langflip.exe"])));
        assert!(!wanted(args(&["langflip.exe", "--updated"])));
    }

    #[test]
    fn a_second_start_reaches_the_running_copy() {
        let name = format!("Local\\LangflipShowTest{}", std::process::id());
        let message = WM_APP + 77;
        let mut msg = MSG::default();
        // The first peek gives this thread its message queue.
        let _ = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) };
        let event = open(&name).expect("the event opens");
        let raw = event.0 as usize;
        let thread = unsafe { GetCurrentThreadId() };
        std::thread::spawn(move || wait(HANDLE(raw as _), thread, message));

        assert!(signal(&name));
        let until = Instant::now() + Duration::from_secs(5);
        let got = loop {
            if unsafe { PeekMessageW(&mut msg, None, message, message, PM_REMOVE) }.as_bool() {
                break true;
            }
            if Instant::now() > until {
                break false;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(got, "the running copy got no message");
    }
}
