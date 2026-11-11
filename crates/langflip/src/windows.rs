//! Windows: low-level keyboard and mouse hooks in this process, `SendInput` for the fix.
//!
//! The hooks run on the main thread. A hook decides at once and posts the fix to the same
//! thread's message loop, so the hook returns before Windows times it out.

use std::cell::{Cell, RefCell};
use std::os::windows::ffi::OsStrExt;
use std::time::{Duration, Instant};

use langflip_core::{Engine, Fix, Key, Lang, Stroke};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
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
    KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, VIRTUAL_KEY, VK_BACK,
    VK_CAPITAL, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_PACKET,
    VK_PAUSE, VK_RCONTROL, VK_RETURN, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
};
use windows::Win32::UI::Shell::ShellExecuteW;

use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, EVENT_OBJECT_FOCUS, EVENT_SYSTEM_FOREGROUND, GetClassNameW,
    GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, KBDLLHOOKSTRUCT, KillTimer,
    LLKHF_EXTENDED, MSG, PostMessageW, PostQuitMessage, PostThreadMessageW, STATE_SYSTEM_PROTECTED,
    SW_SHOWNORMAL, SetForegroundWindow, SetTimer, SetWindowsHookExW, TranslateMessage,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP,
    WM_INPUTLANGCHANGEREQUEST, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN,
    WM_SYSKEYDOWN, WM_TIMER, WM_XBUTTONDOWN,
};
use windows::core::{PCWSTR, PWSTR, w};

use crate::autostart;
use crate::caret::Caret;
use crate::language;
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
const WM_APP_CARET: u32 = WM_APP + 3;

/// A key press or release swallowed while a fix was being typed.
#[derive(Clone, Copy)]
struct Held {
    vk: u16,
    scan: u16,
    up: bool,
    extended: bool,
}

/// A layout switch from the tray, waiting for its window to take the front.
struct Switch {
    window: HWND,
    to: Lang,
    timer: usize,
    ticks: u32,
}

struct State {
    engine: Engine,
    settings: Settings,
    self_elevated: bool,
    /// The window in front is excluded or elevated, or its focus is a password field.
    skip_window: bool,
    password: bool,
    /// A key went by while the state was in use, so the screen holds more of the word than the
    /// engine. The word gets no fix until its end key.
    skip_word: bool,
    /// The layout the last fix switched to, and when. A busy window takes it late, and until then
    /// the keys typed in it already land in that layout.
    expect: Option<(Lang, Instant)>,
    /// The fix and the end key to type after it, if any.
    queued: Option<(Fix, Option<u16>)>,
    /// The end key of the last finished word, for Undo and the manual fix.
    last_end_vk: VIRTUAL_KEY,
    last_focus: (isize, i32, i32),
    /// The last window in front other than the taskbar, which a tray click brings to the front.
    target: HWND,
    switch: Option<Switch>,
    tray: Option<Tray>,
    /// Reads the word before the caret once the word being typed got lost.
    caret: Caret,
    /// The latest read asked for. An older answer comes too late and is dropped.
    caret_id: u64,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    /// A hook call found the state in use, so the word being typed missed a key.
    static MISSED: Cell<bool> = const { Cell::new(false) };
    /// A fix is on its way. Until its end marker arrives, typed keys wait in `HELD`.
    /// These live outside the state, so the hook holds keys even while the state is in use.
    static PENDING: Cell<bool> = const { Cell::new(false) };
    /// When the fix was queued, so a fix that got stuck lets the keys go.
    static PENDING_AT: Cell<Option<Instant>> = const { Cell::new(None) };
    static HELD: RefCell<Vec<Held>> = const { RefCell::new(Vec::new()) };
    /// End markers on their way, counting a queued fix. Keys wait until the last one arrives.
    static MARKERS: Cell<u32> = const { Cell::new(0) };
    /// The timer that checks whether the window took the new layout, so held keys can replay.
    static REPLAY_TIMER: Cell<usize> = const { Cell::new(0) };
}

/// Runs `f` on the state, or returns None when the state is in use.
///
/// Windows calls the hooks on this thread while a call such as `Shell_NotifyIconW` waits on
/// another process. A second borrow there would panic, and a panic in a hook ends Langflip.
/// The word that missed the key starts again at the next call.
fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| {
        let Ok(mut state) = cell.try_borrow_mut() else {
            MISSED.set(true);
            return None;
        };
        let s = state.as_mut()?;
        if MISSED.take() {
            drop_word(s, "the hook missed a key");
            s.skip_word = true;
        }
        Some(f(s))
    })
}

impl State {
    fn new(engine: Engine, settings: Settings, tray: Option<Tray>) -> State {
        let main_thread = unsafe { GetCurrentThreadId() };
        State {
            engine,
            settings,
            caret: Caret::start(main_thread, WM_APP_CARET),
            caret_id: 0,
            self_elevated: process_elevated(unsafe { GetCurrentProcess() }),
            skip_window: false,
            password: false,
            skip_word: false,
            expect: None,
            queued: None,
            last_end_vk: VK_SPACE,
            last_focus: (0, 0, 0),
            target: HWND::default(),
            switch: None,
            tray,
        }
    }
}

pub fn run(engine: Engine, settings: Settings, first_start: bool) {
    unsafe {
        // A second copy would fix each word twice. After an update the old copy
        // still holds the mutex for a moment, so the new one waits up to 10 seconds.
        let mut tries = if std::env::args().any(|a| a == "--updated") {
            50
        } else {
            0
        };
        let _mutex = loop {
            let mutex = CreateMutexW(None, true, w!("Local\\Langflip"));
            if GetLastError() != ERROR_ALREADY_EXISTS {
                break mutex;
            }
            if tries == 0 {
                return;
            }
            tries -= 1;
            if let Ok(handle) = mutex {
                let _ = CloseHandle(handle);
            }
            std::thread::sleep(Duration::from_millis(200));
        };
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        // Autostart turns on by itself after install. Later starts keep the person's choice,
        // and rewrite the path in case langflip.exe moved.
        let autostart = first_start || autostart::enabled();
        if autostart {
            autostart::set(true);
        }
        let tray = Tray::new(settings.enabled, autostart);
        tray.set_language(settings.language.as_deref(), &settings.keep_words);
        STATE.with(|s| *s.borrow_mut() = Some(State::new(engine, settings, Some(tray))));

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
        crate::update::start(GetCurrentThreadId());
        // Windows tells no other process when a window switches its layout, so the tray asks.
        SetTimer(None, 0, 250, None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            match msg.message {
                WM_APP_FIX => apply_queued_fix(),
                WM_APP_REPLAY => replay_held(),
                WM_TIMER if msg.wParam.0 == REPLAY_TIMER.get() => replay_held(),
                WM_APP_CARET => go_on_with_caret_word(),
                WM_TIMER
                    if with_state(|s| s.switch.as_ref().map(|w| w.timer)).flatten()
                        == Some(msg.wParam.0) =>
                {
                    finish_switch()
                }
                WM_TIMER => {
                    release_stuck_fix();
                    // Until the switch lands, the window still reports the old layout.
                    if with_state(|s| s.switch.is_none()) == Some(true) {
                        let lang = target().and_then(window_lang);
                        with_state(|s| s.tray.as_ref().map(|t| t.set_lang(lang)));
                    }
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
                s.engine.reset();
            });
        }
        Action::SwitchLayout => {
            let Some(target) = target() else { return };
            let to = window_lang(target).map_or(Lang::En, Lang::other);
            unsafe {
                // The click brought the taskbar to the front, so the window gets it back.
                let _ = SetForegroundWindow(target);
            }
            with_state(|s| {
                if let Some(old) = s.switch.take() {
                    let _ = unsafe { KillTimer(None, old.timer) };
                }
                let timer = unsafe { SetTimer(None, 0, SWITCH_TICK_MS, None) };
                s.switch = Some(Switch {
                    window: target,
                    to,
                    timer,
                    ticks: 0,
                });
                s.tray.as_ref().map(|t| t.set_lang(Some(to)))
            });
        }
        Action::Autostart(on) => autostart::set(on),
        Action::RemoveKeepWord(word) => {
            with_state(|s| {
                s.settings.remove_keep_word(&word);
                s.engine.keep_words = s.settings.keep_set();
                show_keep_words(s);
            });
        }
        Action::SetLanguage(choice) => {
            with_state(|s| {
                s.settings.language = choice;
                s.settings.save();
                show_language(s);
            });
        }
        Action::OpenSettings => open_settings(),
        Action::Quit => unsafe { PostQuitMessage(0) },
    }
}

/// How often a tray switch checks whether its window is in front, and for how many ticks.
const SWITCH_TICK_MS: u32 = 20;
const SWITCH_TICKS: u32 = 25;

/// Sends the tray switch once its window is in front.
///
/// Windows hands the front over a moment after `SetForegroundWindow`. A window asked to switch
/// before then switches alone, and takes the shared layout back at the next focus change.
fn finish_switch() {
    let ready = with_state(|s| {
        let w = s.switch.as_mut()?;
        w.ticks += 1;
        let front = unsafe { GetForegroundWindow() } == w.window;
        if !front && w.ticks < SWITCH_TICKS {
            return None;
        }
        debug(format_args!(
            "tray switch after {} ticks, in front {front}",
            w.ticks
        ));
        let w = s.switch.take()?;
        let _ = unsafe { KillTimer(None, w.timer) };
        Some((w.window, w.to))
    });
    if let Some((window, to)) = ready.flatten() {
        unsafe { switch_layout(window, to) };
    }
}

/// Picks up an edit of the settings file made by hand.
fn reload_settings(s: &mut State) {
    if !s.settings.reload() {
        return;
    }
    s.engine.keep_words = s.settings.keep_set();
    if let Some(tray) = &s.tray {
        tray.set_enabled(s.settings.enabled);
    }
    show_language(s);
}

/// Sets the menu language from the settings and relabels the tray menu in it.
fn show_language(s: &State) {
    language::apply(s.settings.language.as_deref());
    if let Some(tray) = &s.tray {
        tray.set_language(s.settings.language.as_deref(), &s.settings.keep_words);
    }
}

/// Shows the words Langflip keeps in the tray menu.
fn show_keep_words(s: &State) {
    if let Some(tray) = &s.tray {
        tray.set_keep_words(&s.settings.keep_words);
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
        0x02 => Key::Digit1,
        0x03 => Key::Digit2,
        0x04 => Key::Digit3,
        0x05 => Key::Digit4,
        0x06 => Key::Digit5,
        0x07 => Key::Digit6,
        0x08 => Key::Digit7,
        0x09 => Key::Digit8,
        0x0A => Key::Digit9,
        0x0B => Key::Digit0,
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

fn is_modifier(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_SHIFT
            | VK_CONTROL
            | VK_MENU
            | VK_CAPITAL
            | VK_LWIN
            | VK_RWIN
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_LMENU
            | VK_RMENU
    )
}

fn pressed(vk: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(vk.0.into()) < 0 }
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
        let swallow = hold_key(info, down).or_else(|| with_state(|s| on_key(s, info, down)));
        if swallow == Some(true) {
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Lets the events of a fix through and holds typed keys while a fix is on its way.
/// Returns true to swallow the event, or None for a key the engine reads.
fn hold_key(info: &KBDLLHOOKSTRUCT, down: bool) -> Option<bool> {
    match info.dwExtraInfo {
        TAG_OWN => return Some(false),
        TAG_END => {
            // Keys held until now go out with a marker of their own, and keys pressed while they
            // replay wait behind it. Otherwise a busy PC types them ahead of the held ones.
            let left = MARKERS.get().saturating_sub(1);
            MARKERS.set(left);
            if left == 0 {
                post(WM_APP_REPLAY);
            }
            return Some(true);
        }
        // A held key that started a fix of its own. The keys replayed after it would reach the
        // app ahead of that fix, so they wait for it too. The replay's own marker counts as one.
        TAG_REPLAY if MARKERS.get() <= 1 => return None,
        _ => {}
    }
    if !PENDING.get() {
        return None;
    }
    HELD.with_borrow_mut(|h| {
        h.push(Held {
            vk: info.vkCode as u16,
            scan: info.scanCode as u16,
            up: !down,
            extended: info.flags.0 & LLKHF_EXTENDED.0 != 0,
        })
    });
    Some(true)
}

/// Drops the word being typed, and logs why when it had keys.
fn drop_word(s: &mut State, why: &str) {
    let keys = s.engine.word_len();
    if keys > 0 {
        debug(format_args!("drop a word of {keys} keys: {why}"));
    }
    s.engine.reset();
}

/// Returns true to swallow the event.
fn on_key(s: &mut State, info: &KBDLLHOOKSTRUCT, down: bool) -> bool {
    if !down {
        return false;
    }
    let vk = VIRTUAL_KEY(info.vkCode as u16);
    if vk == VK_PAUSE {
        debug(format_args!("pause, pending {}", PENDING.get()));
        let fix = if pressed(VK_SHIFT) {
            let lang = layout(s);
            s.engine.convert(lang)
        } else {
            s.engine.undo()
        };
        if let Some(fix) = fix {
            if s.settings.store_keep_words(&s.engine.keep_words) {
                show_keep_words(s);
            }
            // Only a finished word had an end key, which the fix erased and types again.
            let end = fix.retype_end.then_some(s.last_end_vk.0);
            queue(s, fix, end);
            return true;
        }
        return false;
    }
    if is_modifier(vk) {
        return false;
    }
    // A character another app types with `KEYEVENTF_UNICODE`. Its scan code holds the character:
    // read as a scan code, the character "1" would be the N key.
    if vk == VK_PACKET {
        drop_word(s, "text from another app");
        return false;
    }
    if !s.settings.enabled || s.skip_window || s.password {
        s.engine.reset();
        return false;
    }
    if pressed(VK_CONTROL) || pressed(VK_MENU) || pressed(VK_LWIN) || pressed(VK_RWIN) {
        lose_word(s, "shortcut");
        return false;
    }
    if s.skip_word {
        s.skip_word = !matches!(vk, VK_SPACE | VK_RETURN | VK_TAB);
        s.engine.reset();
        return false;
    }
    if vk == VK_BACK {
        s.engine.backspace(s.last_end_vk == VK_SPACE);
        return false;
    }
    let extended = info.flags.0 & LLKHF_EXTENDED.0 != 0;
    let shift = pressed(VK_SHIFT);
    // A digit with Shift types other signs in each layout, so it clears the word below.
    let key = key_for_scan(info.scanCode).filter(|k| !extended && !(k.is_digit() && shift));
    if let Some(key) = key {
        let caps = unsafe { GetKeyState(VK_CAPITAL.0.into()) } & 1 != 0;
        let lang = layout(s);
        if let Some(fix) = s.engine.key(Stroke::new(key, shift, caps), lang) {
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
        s.last_end_vk = vk;
        let lang = layout(s);
        let keys = s.engine.word_len();
        let fix = s.engine.end_word(lang);
        // The log keeps the decision, not the word.
        let erase = fix.as_ref().map(|f| f.erase);
        debug(format_args!(
            "word end in {lang:?}: {keys} keys, fix of {erase:?} keys"
        ));
        if let Some(fix) = fix {
            queue(s, fix, Some(vk.0));
            return true;
        }
        return false;
    }
    lose_word(s, "other key");
    false
}

/// The caret moved, or a sign or a shortcut ended the word. The app then tells which word, if
/// any, sits before the caret.
fn lose_word(s: &mut State, why: &str) {
    drop_word(s, why);
    if s.settings.enabled && !s.skip_window && !s.password {
        s.caret_id += 1;
        s.caret.request(s.caret_id);
    }
}

/// The text before the caret arrived. The word it ends with is the start of the next keys.
fn go_on_with_caret_word() {
    with_state(|s| {
        let Some((id, text)) = s.caret.take() else {
            return;
        };
        if id == s.caret_id {
            // The log keeps the size and the layout, not the word.
            let word = s.engine.continue_word(&text);
            debug(format_args!("word before the caret: {word:?}"));
        }
    });
}

fn queue(s: &mut State, fix: Fix, end_vk: Option<u16>) {
    PENDING.set(true);
    PENDING_AT.set(Some(Instant::now()));
    MARKERS.set(MARKERS.get() + 1);
    s.queued = Some((fix, end_vk));
    post(WM_APP_FIX);
}

/// Posts to this thread's message loop. The hooks and the loop share the main thread.
fn post(msg: u32) {
    unsafe {
        let _ = PostThreadMessageW(GetCurrentThreadId(), msg, WPARAM(0), LPARAM(0));
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
    // Some apps drop a Backspace without the scan code a physical keyboard sends.
    let scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) } as u16;
    inputs.push(key_input(vk, scan, KEYBD_EVENT_FLAGS(0), TAG_OWN));
    inputs.push(key_input(vk, scan, KEYEVENTF_KEYUP, TAG_OWN));
}

fn apply_queued_fix() {
    let Some((fix, end_vk)) = with_state(|s| s.queued.take()).flatten() else {
        return;
    };
    let mut inputs = Vec::new();
    for _ in 0..fix.erase {
        tap(&mut inputs, VK_BACK.0);
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
        debug(format_args!(
            "fix: {} Backspaces, {} letters, end {end_vk:?}, sent {sent} of {}",
            fix.erase,
            fix.text.chars().count(),
            inputs.len()
        ));
        if sent as usize != inputs.len() {
            // Windows blocked the input, often for an elevated window. Stop waiting for the marker.
            MARKERS.set(MARKERS.get().saturating_sub(1));
            post(WM_APP_REPLAY);
        }
        with_state(|s| s.expect = Some((fix.switch_to, Instant::now())));
        switch_layout(GetForegroundWindow(), fix.switch_to);
    }
}

/// How long a fix may wait for its end marker before the keys held behind it go out.
const STUCK_FIX: Duration = Duration::from_secs(1);

/// Lets the keyboard go when a fix got stuck, since the hook holds each key until the fix ends.
///
/// A menu's message loop drops the posted fix, and a switch to the secure desktop drops the
/// end marker.
fn release_stuck_fix() {
    let late = PENDING_AT.get().is_none_or(|t| t.elapsed() >= STUCK_FIX);
    if !PENDING.get() || !late {
        return;
    }
    let stuck = with_state(|s| {
        debug(format_args!("stuck fix, queued {}", s.queued.is_some()));
        // The queued fix sends the one marker still to come.
        MARKERS.set(u32::from(s.queued.is_some()));
        s.queued.is_some()
    });
    match stuck {
        // The held keys kept the screen as the fix expects it, so the fix still fits.
        Some(true) => apply_queued_fix(),
        Some(false) => replay_held(),
        None => {}
    }
}

/// How long held keys wait for the window to take the layout of the fix. A busy app takes longer.
const LAYOUT_WAIT: Duration = Duration::from_millis(500);

/// Types the held keys again, then an end marker, once the last marker came back.
fn replay_held() {
    if MARKERS.get() > 0 {
        return;
    }
    if HELD.with_borrow(Vec::is_empty) {
        PENDING.set(false);
        return;
    }
    // The window switches its layout on its own thread. Keys sent before that land in the old one.
    // The wait runs on a timer, so the hook keeps holding the keys typed meanwhile.
    let waiting = with_state(|s| {
        s.expect
            .is_some_and(|(lang, at)| current_lang() != Some(lang) && at.elapsed() < LAYOUT_WAIT)
    });
    if waiting == Some(true) {
        if REPLAY_TIMER.get() == 0 {
            REPLAY_TIMER.set(unsafe { SetTimer(None, 0, 10, None) });
        }
        return;
    }
    let timer = REPLAY_TIMER.replace(0);
    if timer != 0 {
        let _ = unsafe { KillTimer(None, timer) };
    }
    let held = HELD.take();
    debug(format_args!("replay {} held key events", held.len()));
    let mut inputs: Vec<INPUT> = held.iter().map(replay_input).collect();
    inputs.push(key_input(VK_MARKER, 0, KEYEVENTF_KEYUP, TAG_END));
    MARKERS.set(1);
    PENDING_AT.set(Some(Instant::now()));
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        MARKERS.set(0);
        PENDING.set(false);
    }
}

/// The event that types a held key again.
fn replay_input(h: &Held) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if h.up {
        flags |= KEYEVENTF_KEYUP;
    }
    if h.extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    // Another app's character goes out as that character, not as a key.
    if h.vk == VK_PACKET.0 {
        return key_input(0, h.scan, flags | KEYEVENTF_UNICODE, TAG_REPLAY);
    }
    key_input(h.vk, h.scan, flags, TAG_REPLAY)
}

unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0
        && matches!(
            wparam.0 as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        with_state(|s| {
            lose_word(s, "click");
            s.skip_word = false;
        });
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
                lose_word(s, "focus");
                s.skip_word = false;
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
        drop_word(s, "window");
        s.skip_word = false;
        s.password = false;
        let excluded = app.as_deref().is_some_and(|a| s.settings.is_excluded(a));
        s.skip_window = excluded || (elevated && !s.self_elevated);
        debug(format_args!(
            "front {hwnd:?} {app:?}, elevated {elevated}, skip {}",
            s.skip_window
        ));
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

/// The layout the keys typed now land in. A window that hasn't taken the layout of the last fix
/// yet still types in it, since Windows hands it the switch before the keys.
fn layout(s: &mut State) -> Option<Lang> {
    let now = current_lang();
    match s.expect {
        Some((lang, at)) if now != Some(lang) && at.elapsed() < LAYOUT_WAIT => Some(lang),
        _ => {
            s.expect = None;
            now
        }
    }
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

/// Appends a line to the decision log. Tests write none.
fn debug(line: std::fmt::Arguments) {
    if !cfg!(test) {
        crate::log::write(line);
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

#[cfg(test)]
mod tests {
    use super::*;
    use langflip_core::Model;
    use std::collections::HashSet;

    /// A fresh state on this test's thread, with no tray.
    fn start() {
        let model = Model::from_bytes(include_bytes!("../../../model/langflip.model")).unwrap();
        let engine = Engine::new(model, HashSet::new());
        STATE.with(|s| *s.borrow_mut() = Some(State::new(engine, Settings::default(), None)));
    }

    /// True when the engine holds keys of a word being typed.
    fn word_in_progress() -> bool {
        with_state(|s| s.engine.convert(Some(Lang::En)).is_some()).unwrap()
    }

    #[test]
    fn text_from_other_apps_is_no_key() {
        start();
        let digit_one = KBDLLHOOKSTRUCT {
            vkCode: VK_PACKET.0.into(),
            scanCode: u32::from(b'1'),
            ..Default::default()
        };
        assert!(!with_state(|s| on_key(s, &digit_one, true)).unwrap());
        assert!(!word_in_progress());
    }

    #[test]
    fn text_from_other_apps_replays_as_text() {
        let held = Held {
            vk: VK_PACKET.0,
            scan: u16::from(b'1'),
            up: false,
            extended: false,
        };
        let ki = unsafe { replay_input(&held).Anonymous.ki };
        assert_eq!((ki.wVk.0, ki.wScan), (0, u16::from(b'1')));
        assert_eq!(ki.dwFlags, KEYEVENTF_UNICODE);
    }

    #[test]
    fn a_stuck_fix_lets_the_keys_go() {
        start();
        PENDING.set(true);
        MARKERS.set(1);
        PENDING_AT.set(Some(Instant::now()));
        release_stuck_fix();
        assert!(PENDING.get());
        PENDING_AT.set(Some(Instant::now() - STUCK_FIX));
        release_stuck_fix();
        assert!(!PENDING.get());
        assert_eq!(MARKERS.get(), 0);
    }

    /// The end marker of a fix or a replay.
    fn marker() -> KBDLLHOOKSTRUCT {
        KBDLLHOOKSTRUCT {
            vkCode: VK_MARKER.into(),
            dwExtraInfo: TAG_END,
            ..Default::default()
        }
    }

    #[test]
    fn keys_wait_until_the_held_keys_replay() {
        start();
        PENDING.set(true);
        MARKERS.set(1);
        hold_key(&press(0x47, 0x22), true);
        // The fix's marker came back, but the held G hasn't replayed yet.
        assert_eq!(hold_key(&marker(), false), Some(true));
        assert!(PENDING.get());
        assert_eq!(hold_key(&press(0x48, 0x23), true), Some(true));
        assert_eq!(HELD.with_borrow(Vec::len), 2);
        HELD.take();
        replay_held();
        assert!(!PENDING.get());
    }

    /// A key typed again from the held ones.
    fn replayed(vk: u32, scan: u32) -> KBDLLHOOKSTRUCT {
        KBDLLHOOKSTRUCT {
            dwExtraInfo: TAG_REPLAY,
            ..press(vk, scan)
        }
    }

    #[test]
    fn replayed_keys_wait_behind_a_fix_one_of_them_started() {
        start();
        PENDING.set(true);
        MARKERS.set(1);
        assert_eq!(hold_key(&replayed(0x47, 0x22), true), None);
        // The replayed key queued a fix: the keys replayed after it wait for that fix.
        MARKERS.set(2);
        assert_eq!(hold_key(&replayed(0x48, 0x23), true), Some(true));
        assert_eq!(HELD.take().len(), 1);
        MARKERS.set(0);
        PENDING.set(false);
    }

    #[test]
    fn a_marker_keeps_the_keys_waiting_for_the_next_fix() {
        start();
        PENDING.set(true);
        MARKERS.set(2);
        hold_key(&marker(), false);
        assert!(PENDING.get());
        assert_eq!(MARKERS.get(), 1);
        MARKERS.set(0);
        PENDING.set(false);
    }

    #[test]
    fn keys_after_a_fix_read_in_its_layout_until_the_window_takes_it() {
        start();
        let other = current_lang().map_or(Lang::Ru, Lang::other);
        let lang = with_state(|s| {
            s.expect = Some((other, Instant::now()));
            layout(s)
        });
        assert_eq!(lang, Some(Some(other)));
        let lang = with_state(|s| {
            s.expect = Some((other, Instant::now() - LAYOUT_WAIT));
            layout(s)
        });
        assert_eq!(lang, Some(current_lang()));
    }

    #[test]
    fn a_hook_call_inside_another_drops_the_word() {
        start();
        let inner = with_state(|s| {
            s.engine
                .key(Stroke::new(Key::G, false, false), Some(Lang::En));
            with_state(|_| ())
        });
        assert_eq!(inner, Some(None));
        assert!(!word_in_progress());
    }

    /// A key press from the keyboard.
    fn press(vk: u32, scan: u32) -> KBDLLHOOKSTRUCT {
        KBDLLHOOKSTRUCT {
            vkCode: vk,
            scanCode: scan,
            ..Default::default()
        }
    }

    #[test]
    fn a_missed_key_leaves_the_word_as_typed() {
        start();
        with_state(|_| with_state(|_| ()));
        assert_eq!(with_state(|s| s.skip_word), Some(true));
        // The G key: the engine skips it, since the screen holds a key it missed.
        assert_eq!(
            with_state(|s| on_key(s, &press(0x47, 0x22), true)),
            Some(false)
        );
        assert!(!word_in_progress());
        with_state(|s| on_key(s, &press(VK_SPACE.0.into(), 0x39), true));
        assert_eq!(with_state(|s| s.skip_word), Some(false));
    }

    #[test]
    fn keys_wait_behind_a_fix_while_the_state_is_in_use() {
        start();
        PENDING.set(true);
        let held = with_state(|_| hold_key(&press(0x47, 0x22), true));
        assert_eq!(held, Some(Some(true)));
        assert_eq!(HELD.take().len(), 1);
        PENDING.set(false);
    }
}
