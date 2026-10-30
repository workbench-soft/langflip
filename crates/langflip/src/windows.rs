//! Windows: low-level keyboard and mouse hooks in this process, `SendInput` for the fix.
//!
//! The hooks run on the main thread. A hook decides at once and posts the fix to the same
//! thread's message loop, so the hook returns before Windows times it out.

use std::cell::RefCell;
use std::os::windows::ffi::OsStrExt;

use langflip_core::{Engine, Fix, Key, Lang, Stroke};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, GetCurrentThreadId, OpenProcess, OpenProcessToken,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    AccessibleObjectFromEvent, HWINEVENTHOOK, IAccessible, SetWinEventHook,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, GetKeyboardLayoutList, HKL, INPUT, INPUT_0,
    INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::Shell::ShellExecuteW;

use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, EVENT_OBJECT_FOCUS, EVENT_SYSTEM_FOREGROUND, GetClassNameW,
    GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, KBDLLHOOKSTRUCT, LLKHF_EXTENDED,
    MSG, PostMessageW, PostQuitMessage, PostThreadMessageW, STATE_SYSTEM_PROTECTED, SW_SHOWNORMAL,
    SetForegroundWindow, SetTimer, SetWindowsHookExW, TranslateMessage, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP, WM_INPUTLANGCHANGEREQUEST,
    WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_TIMER,
    WM_XBUTTONDOWN,
};
use windows::core::{PCWSTR, PWSTR, w};

use crate::settings::{self, Settings};
use crate::tray::{Action, Tray};

/// Marks the events Langflip sends, so the hook lets them through. "LF" and a kind.
const TAG_OWN: usize = 0x4C46_0001;
/// The last event of a fix. Keys held during the fix replay once it arrives.
const TAG_END: usize = 0x4C46_0002;
/// A held key sent again. The hook treats it as typing.
const TAG_REPLAY: usize = 0x4C46_0003;
/// An unassigned virtual key, used only for the end marker. The hook swallows it.
const VK_MARKER: u16 = 0xFF;

const WM_APP_FIX: u32 = WM_APP + 1;
const WM_APP_REPLAY: u32 = WM_APP + 2;

const VK_BACK: u32 = 0x08;
const VK_TAB: u32 = 0x09;
const VK_RETURN: u32 = 0x0D;
const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;
const VK_PAUSE: u32 = 0x13;
const VK_CAPITAL: i32 = 0x14;
const VK_SPACE: u32 = 0x20;
const VK_LWIN: i32 = 0x5B;
const VK_RWIN: i32 = 0x5C;

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

/// A key press or release swallowed while a fix was being typed.
#[derive(Clone, Copy)]
struct Held {
    vk: u16,
    scan: u16,
    up: bool,
    extended: bool,
}

struct State {
    engine: Engine,
    settings: Settings,
    settings_mtime: Option<std::time::SystemTime>,
    main_thread: u32,
    self_elevated: bool,
    /// The window in front is excluded or elevated, or its focus is a password field.
    skip_window: bool,
    password: bool,
    /// A fix is on its way. Until its end marker arrives, typed keys wait in `held`.
    pending: bool,
    held: Vec<Held>,
    /// The layout the last fix switched to. Held keys wait for it before they replay.
    switching_to: Option<Lang>,
    /// The fix and the end key to type after it, if any.
    queued: Option<(Fix, Option<u16>)>,
    /// The end key of the last finished word, for Undo and the manual fix.
    last_end_vk: u16,
    last_focus: (isize, i32, i32),
    /// The last window in front other than the taskbar, which a tray click brings to the front.
    target: HWND,
    tray: Option<Tray>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}

pub fn run(engine: Engine, settings: Settings, first_start: bool) {
    unsafe {
        // A second copy would fix each word twice.
        let _mutex = CreateMutexW(None, true, w!("Local\\Langflip"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        // Autostart turns on by itself after install. Later starts keep the person's choice,
        // and rewrite the path in case langflip.exe moved.
        let autostart = first_start || autostart_enabled();
        if autostart {
            set_autostart(true);
        }
        let tray = Tray::new(settings.enabled, autostart);
        STATE.with(|s| {
            *s.borrow_mut() = Some(State {
                engine,
                settings,
                settings_mtime: settings::modified(),
                main_thread: GetCurrentThreadId(),
                self_elevated: process_elevated(GetCurrentProcess()),
                skip_window: false,
                password: false,
                pending: false,
                held: Vec::new(),
                switching_to: None,
                queued: None,
                last_end_vk: VK_SPACE as u16,
                last_focus: (0, 0, 0),
                target: HWND::default(),
                tray: Some(tray),
            })
        });

        let _kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0)
            .expect("the keyboard hook could not be set");
        let _mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), None, 0)
            .expect("the mouse hook could not be set");
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        let _fg = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event),
            0,
            0,
            flags,
        );
        let _focus = SetWinEventHook(
            EVENT_OBJECT_FOCUS,
            EVENT_OBJECT_FOCUS,
            None,
            Some(win_event),
            0,
            0,
            flags,
        );
        foreground_changed();
        // Windows tells no other process when a window switches its layout, so the tray asks.
        SetTimer(None, 0, 250, None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            match msg.message {
                WM_APP_FIX => apply_queued_fix(),
                WM_APP_REPLAY => replay_held(),
                WM_TIMER => {
                    let lang = target().and_then(window_lang);
                    with_state(|s| s.tray.as_ref().map(|t| t.set_lang(lang)));
                }
                _ => {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            handle_tray();
        }
    }
}

fn handle_tray() {
    let Some(action) = with_state(|s| s.tray.as_ref().and_then(Tray::poll)).flatten() else {
        return;
    };
    match action {
        Action::Enable(on) => {
            with_state(|s| {
                s.settings.enabled = on;
                s.settings.save();
                s.settings_mtime = settings::modified();
                s.engine.reset();
            });
        }
        Action::SwitchLayout => {
            let Some(target) = target() else { return };
            let to = window_lang(target).map_or(Lang::En, Lang::other);
            unsafe {
                // The click brought the taskbar to the front, so the window gets it back.
                let _ = SetForegroundWindow(target);
                switch_layout(target, to);
            }
            with_state(|s| s.tray.as_ref().map(|t| t.set_lang(Some(to))));
        }
        Action::Autostart(on) => set_autostart(on),
        Action::OpenSettings => open_settings(),
        Action::Quit => unsafe { PostQuitMessage(0) },
    }
}

/// Picks up an edit of the settings file made by hand.
fn reload_settings(s: &mut State) {
    let mtime = settings::modified();
    if mtime == s.settings_mtime {
        return;
    }
    s.settings_mtime = mtime;
    s.settings = Settings::load();
    s.engine.keep_words = s.settings.keep_set();
    if let Some(tray) = &s.tray {
        tray.set_enabled(s.settings.enabled);
    }
}

fn open_settings() {
    let path: Vec<u16> = settings::path()
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect();
    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(path.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}

/// The key at this US position, from its scan code. Scan codes follow the key, not the layout.
fn key_for_scan(scan: u32) -> Option<Key> {
    Some(match scan {
        0x10 => Key::Q,
        0x11 => Key::W,
        0x12 => Key::E,
        0x13 => Key::R,
        0x14 => Key::T,
        0x15 => Key::Y,
        0x16 => Key::U,
        0x17 => Key::I,
        0x18 => Key::O,
        0x19 => Key::P,
        0x1A => Key::BracketLeft,
        0x1B => Key::BracketRight,
        0x1E => Key::A,
        0x1F => Key::S,
        0x20 => Key::D,
        0x21 => Key::F,
        0x22 => Key::G,
        0x23 => Key::H,
        0x24 => Key::J,
        0x25 => Key::K,
        0x26 => Key::L,
        0x27 => Key::Semicolon,
        0x28 => Key::Quote,
        0x29 => Key::Backquote,
        0x2C => Key::Z,
        0x2D => Key::X,
        0x2E => Key::C,
        0x2F => Key::V,
        0x30 => Key::B,
        0x31 => Key::N,
        0x32 => Key::M,
        0x33 => Key::Comma,
        0x34 => Key::Period,
        _ => return None,
    })
}

fn is_modifier(vk: u32) -> bool {
    matches!(vk, 0x10..=0x12 | 0x14 | 0x5B | 0x5C | 0xA0..=0xA5)
}

fn pressed(vk: i32) -> bool {
    unsafe { GetAsyncKeyState(vk) < 0 }
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
        if with_state(|s| on_key(s, info, down)) == Some(true) {
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Returns true to swallow the event.
fn on_key(s: &mut State, info: &KBDLLHOOKSTRUCT, down: bool) -> bool {
    match info.dwExtraInfo {
        TAG_OWN => return false,
        TAG_END => {
            s.pending = false;
            if !s.held.is_empty() {
                post(s, WM_APP_REPLAY);
            }
            return true;
        }
        _ => {}
    }
    let replay = info.dwExtraInfo == TAG_REPLAY;
    if s.pending && !replay {
        s.held.push(Held {
            vk: info.vkCode as u16,
            scan: info.scanCode as u16,
            up: !down,
            extended: info.flags.0 & LLKHF_EXTENDED.0 != 0,
        });
        return true;
    }
    if !down {
        return false;
    }
    let vk = info.vkCode;
    if vk == VK_PAUSE {
        debug(format_args!("pause, pending {}", s.pending));
        let fix = if pressed(VK_SHIFT) {
            let lang = current_lang();
            s.engine.convert(lang)
        } else {
            s.engine.undo()
        };
        if let Some(fix) = fix {
            s.settings.store_keep_words(&s.engine.keep_words);
            s.settings_mtime = settings::modified();
            queue(s, fix, Some(s.last_end_vk));
            return true;
        }
        return false;
    }
    if is_modifier(vk) {
        return false;
    }
    if !s.settings.enabled || s.skip_window || s.password {
        s.engine.reset();
        return false;
    }
    if pressed(VK_CONTROL) || pressed(VK_MENU) || pressed(VK_LWIN) || pressed(VK_RWIN) {
        s.engine.reset();
        return false;
    }
    if vk == VK_BACK {
        s.engine.backspace();
        return false;
    }
    let extended = info.flags.0 & LLKHF_EXTENDED.0 != 0;
    if let Some(key) = key_for_scan(info.scanCode).filter(|_| !extended) {
        let caps = unsafe { GetKeyState(VK_CAPITAL) } & 1 != 0;
        let lang = current_lang();
        if let Some(fix) = s
            .engine
            .key(Stroke::new(key, pressed(VK_SHIFT), caps), lang)
        {
            debug(format_args!(
                "switch while typed in {lang:?}: fix of {} keys",
                fix.erase
            ));
            queue(s, fix, None);
            return true;
        }
        return false;
    }
    if matches!(vk, VK_SPACE | VK_RETURN | VK_TAB) {
        s.last_end_vk = vk as u16;
        let lang = current_lang();
        let fix = s.engine.end_word(lang);
        // The log keeps the decision, not the word.
        let erase = fix.as_ref().map(|f| f.erase);
        debug(format_args!("word end in {lang:?}: fix of {erase:?} keys"));
        if let Some(fix) = fix {
            queue(s, fix, Some(vk as u16));
            return true;
        }
        return false;
    }
    s.engine.reset();
    false
}

fn queue(s: &mut State, fix: Fix, end_vk: Option<u16>) {
    s.pending = true;
    s.queued = Some((fix, end_vk));
    post(s, WM_APP_FIX);
}

fn post(s: &State, msg: u32) {
    unsafe {
        let _ = PostThreadMessageW(s.main_thread, msg, WPARAM(0), LPARAM(0));
    }
}

fn key_input(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS, tag: usize) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: tag,
            },
        },
    }
}

fn tap(inputs: &mut Vec<INPUT>, vk: u16) {
    inputs.push(key_input(vk, 0, KEYBD_EVENT_FLAGS(0), TAG_OWN));
    inputs.push(key_input(vk, 0, KEYEVENTF_KEYUP, TAG_OWN));
}

fn apply_queued_fix() {
    let Some((fix, end_vk)) = with_state(|s| s.queued.take()).flatten() else {
        return;
    };
    let mut inputs = Vec::new();
    for _ in 0..fix.erase {
        tap(&mut inputs, VK_BACK as u16);
    }
    for unit in fix.text.encode_utf16() {
        inputs.push(key_input(0, unit, KEYEVENTF_UNICODE, TAG_OWN));
        inputs.push(key_input(
            0,
            unit,
            KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
            TAG_OWN,
        ));
    }
    if let Some(vk) = end_vk {
        tap(&mut inputs, vk);
    }
    inputs.push(key_input(VK_MARKER, 0, KEYEVENTF_KEYUP, TAG_END));
    unsafe {
        let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
        if sent as usize != inputs.len() {
            // Windows blocked the input, often for an elevated window. Stop waiting for the marker.
            with_state(|s| s.pending = false);
        }
        with_state(|s| s.switching_to = Some(fix.switch_to));
        switch_layout(GetForegroundWindow(), fix.switch_to);
    }
}

fn replay_held() {
    // The window switches its layout on its own thread. Keys sent before that land in the old one.
    let lang = with_state(|s| s.switching_to.take()).flatten();
    for _ in 0..20 {
        if lang.is_none() || current_lang() == lang {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let held = with_state(|s| std::mem::take(&mut s.held)).unwrap_or_default();
    let inputs: Vec<INPUT> = held
        .iter()
        .map(|h| {
            let mut flags = KEYBD_EVENT_FLAGS(0);
            if h.up {
                flags |= KEYEVENTF_KEYUP;
            }
            if h.extended {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }
            key_input(h.vk, h.scan, flags, TAG_REPLAY)
        })
        .collect();
    unsafe {
        SendInput(&inputs, size_of::<INPUT>() as i32);
    }
}

unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0
        && matches!(
            wparam.0 as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        with_state(|s| s.engine.reset());
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn win_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object: i32,
    child: i32,
    _thread: u32,
    _time: u32,
) {
    if event == EVENT_SYSTEM_FOREGROUND {
        foreground_changed();
    } else if event == EVENT_OBJECT_FOCUS {
        let password = unsafe { is_protected(hwnd, object, child) };
        debug(format_args!(
            "focus {hwnd:?} {object} {child} password {password}"
        ));
        with_state(|s| {
            s.password = password;
            // Some apps repeat the focus event for the same field, which keeps the word.
            let focus = (hwnd.0 as isize, object, child);
            if s.last_focus != focus {
                s.last_focus = focus;
                s.engine.reset();
            }
        });
    }
}

/// The focused object reports itself as protected: a password field.
unsafe fn is_protected(hwnd: HWND, object: i32, child: i32) -> bool {
    let mut acc: Option<IAccessible> = None;
    let mut var = VARIANT::default();
    unsafe {
        if AccessibleObjectFromEvent(hwnd, object as u32, child as u32, &mut acc, &mut var).is_err()
        {
            return false;
        }
        let Some(acc) = acc else { return false };
        match acc.get_accState(&var) {
            Ok(state) => {
                i32::try_from(&state).is_ok_and(|v| v as u32 & STATE_SYSTEM_PROTECTED != 0)
            }
            Err(_) => false,
        }
    }
}

fn foreground_changed() {
    let (app, elevated) = unsafe { foreground_process() };
    let hwnd = unsafe { GetForegroundWindow() };
    let taskbar = unsafe { is_taskbar(hwnd) };
    with_state(|s| {
        if !taskbar && !hwnd.is_invalid() {
            s.target = hwnd;
        }
        reload_settings(s);
        s.engine.reset();
        s.password = false;
        let excluded = app.as_deref().is_some_and(|a| s.settings.is_excluded(a));
        s.skip_window = excluded || (elevated && !s.self_elevated);
    });
}

/// The taskbar and the tray overflow, which take the front on a click on the tray icon.
unsafe fn is_taskbar(hwnd: HWND) -> bool {
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) } as usize;
    matches!(
        String::from_utf16_lossy(&buf[..len]).as_str(),
        "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "NotifyIconOverflowWindow"
            | "TopLevelWindowForOverflowXamlIsland"
    )
}

/// The executable name of the window in front, and whether it runs elevated.
unsafe fn foreground_process() -> (Option<String>, bool) {
    unsafe {
        let hwnd = GetForegroundWindow();
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            // Access denied: a process above this one, such as an elevated app.
            return (None, true);
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let name = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
        .ok()
        .map(|_| String::from_utf16_lossy(&buf[..len as usize]))
        .and_then(|path| path.rsplit('\\').next().map(str::to_owned));
        let elevated = process_elevated(process);
        let _ = CloseHandle(process);
        (name, elevated)
    }
}

fn process_elevated(process: HANDLE) -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut len = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

fn lang_of(hkl: HKL) -> Option<Lang> {
    match hkl.0 as usize & 0x3FF {
        0x09 => Some(Lang::En),
        0x19 => Some(Lang::Ru),
        _ => None,
    }
}

/// The layout of the window in front. Another language turns fixing off for the word.
fn current_lang() -> Option<Lang> {
    window_lang(unsafe { GetForegroundWindow() })
}

fn window_lang(hwnd: HWND) -> Option<Lang> {
    unsafe {
        let thread = GetWindowThreadProcessId(hwnd, None);
        lang_of(GetKeyboardLayout(thread))
    }
}

/// The window the tray icon shows and switches. The taskbar in front doesn't count.
fn target() -> Option<HWND> {
    with_state(|s| s.target).filter(|h| !h.is_invalid())
}

/// Appends a line to `%TEMP%\langflip-debug.log` in debug builds.
fn debug(line: std::fmt::Arguments) {
    if cfg!(debug_assertions) {
        use std::io::Write;
        let path = std::env::temp_dir().join("langflip-debug.log");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// Asks the window to switch to the first installed layout of this language.
unsafe fn switch_layout(hwnd: HWND, lang: Lang) {
    unsafe {
        let mut list = [HKL::default(); 32];
        let count = GetKeyboardLayoutList(Some(&mut list)) as usize;
        let found = list[..count].iter().find(|&&h| lang_of(h) == Some(lang));
        debug(format_args!(
            "switch to {lang:?}: {found:?} of {:?}",
            &list[..count]
        ));
        if let Some(hkl) = found {
            let _ = PostMessageW(
                Some(hwnd),
                WM_INPUTLANGCHANGEREQUEST,
                WPARAM(0),
                LPARAM(hkl.0 as isize),
            );
        }
    }
}

fn exe_value() -> Vec<u16> {
    let exe = std::env::current_exe().unwrap_or_default();
    format!("\"{}\"", exe.display())
        .encode_utf16()
        .chain([0])
        .collect()
}

fn autostart_enabled() -> bool {
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            w!("Langflip"),
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        )
        .is_ok()
    }
}

fn set_autostart(on: bool) {
    unsafe {
        if on {
            let value = exe_value();
            let _ = RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                w!("Langflip"),
                REG_SZ.0,
                Some(value.as_ptr() as *const _),
                (value.len() * 2) as u32,
            );
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, w!("Langflip"));
        }
    }
}
