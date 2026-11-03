//! Windows: a task dialog with buttons of its own, for the update question.

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOG_NOTIFICATIONS, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0,
    TDF_ALLOW_DIALOG_CANCELLATION, TDN_CREATED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
};
use windows::core::{HRESULT, HSTRING, PCWSTR, s, w};

pub const ID_UPDATE: i32 = 100;
pub const ID_LATER: i32 = 101;

type TaskDialogIndirect =
    unsafe extern "system" fn(*const TASKDIALOGCONFIG, *mut i32, *mut i32, *mut i32) -> HRESULT;

/// Puts the dialog above the other windows without taking the keyboard, so a Space typed elsewhere can't press a button.
unsafe extern "system" fn on_event(
    hwnd: HWND,
    event: TASKDIALOG_NOTIFICATIONS,
    _: WPARAM,
    _: LPARAM,
    _: isize,
) -> HRESULT {
    if event == TDN_CREATED {
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
        let _ = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags) };
    }
    HRESULT(0)
}

/// Shows the dialog until a click and returns the id of the button. No buttons gives OK.
/// The last button is the default, and Esc or the close box give `IDCANCEL`.
pub fn show(main: &str, content: &str, buttons: &[(i32, String)], icon: PCWSTR) -> i32 {
    let (main, content) = (HSTRING::from(main), HSTRING::from(content));
    let texts: Vec<HSTRING> = buttons
        .iter()
        .map(|(_, t)| HSTRING::from(t.as_str()))
        .collect();
    let rows: Vec<TASKDIALOG_BUTTON> = buttons
        .iter()
        .zip(&texts)
        .map(|((id, _), text)| TASKDIALOG_BUTTON {
            nButtonID: *id,
            pszButtonText: PCWSTR(text.as_ptr()),
        })
        .collect();
    let config = TASKDIALOGCONFIG {
        cbSize: size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
        pszWindowTitle: w!("Langflip"),
        Anonymous1: TASKDIALOGCONFIG_0 { pszMainIcon: icon },
        pszMainInstruction: PCWSTR(main.as_ptr()),
        pszContent: PCWSTR(content.as_ptr()),
        cButtons: rows.len() as u32,
        pButtons: rows.as_ptr(),
        nDefaultButton: buttons.last().map_or(0, |(id, _)| *id),
        pfCallback: Some(on_event),
        ..Default::default()
    };
    // Only Common Controls 6 has the task dialog. The exe manifest asks for it,
    // and a lookup at run time keeps the test binary, which has no manifest, loading.
    unsafe {
        let Ok(lib) = LoadLibraryW(w!("comctl32.dll")) else {
            return 0;
        };
        let Some(proc) = GetProcAddress(lib, s!("TaskDialogIndirect")) else {
            return 0;
        };
        let dialog: TaskDialogIndirect = std::mem::transmute(proc);
        let mut pressed = 0;
        let _ = dialog(
            &config,
            &mut pressed,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        pressed
    }
}
