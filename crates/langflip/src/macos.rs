//! macOS: a session event tap, events posted back to the session for the fix.
//!
//! The tap, the tray and the timer all run on the main run loop, so state sits in a
//! thread-local, as on Windows.

use std::cell::RefCell;
use std::ffi::c_void;

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::runloop::{
    CFRunLoop, CFRunLoopTimer, CFRunLoopTimerRef, kCFRunLoopCommonModes,
};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType, CallbackResult, EventField,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use langflip_core::{Engine, Fix, Key, Lang, Stroke};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSWorkspace};

use crate::settings::{self, Settings};
use crate::tray::{Action, Tray};

/// Marks the events Langflip posts, so the tap lets them through. "LF" and a kind.
const TAG_OWN: i64 = 0x4C46_0001;
/// The last event of a fix. Keys held during the fix replay once it arrives.
const TAG_END: i64 = 0x4C46_0002;
/// A held key posted again. The tap treats it as typing.
const TAG_REPLAY: i64 = 0x4C46_0003;
/// A key code no Mac keyboard sends, used only for the end marker. The tap drops it.
const KEY_MARKER: u16 = 0x7F;

const KEY_RETURN: u16 = 0x24;
const KEY_TAB: u16 = 0x30;
const KEY_SPACE: u16 = 0x31;
const KEY_DELETE: u16 = 0x33;
const KEY_F12: u16 = 0x6F;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardInputSource() -> *mut c_void;
    fn TISCreateInputSourceList(properties: CFDictionaryRef, include_all: bool) -> CFArrayRef;
    fn TISGetInputSourceProperty(source: *mut c_void, key: CFStringRef) -> *const c_void;
    fn TISSelectInputSource(source: *mut c_void) -> i32;
    fn IsSecureEventInputEnabled() -> bool;
    static kTISPropertyInputSourceLanguages: CFStringRef;
    static kTISPropertyInputSourceIsSelectCapable: CFStringRef;
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    fn CGRequestListenEventAccess() -> bool;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
}

struct State {
    engine: Engine,
    settings: Settings,
    settings_mtime: Option<std::time::SystemTime>,
    /// The front app and whether it is excluded.
    front_pid: i32,
    skip_app: bool,
    /// A fix is on its way. Until its end marker arrives, typed keys wait in `held`.
    pending: bool,
    held: Vec<(u16, bool, CGEventFlags)>,
    last_end_key: u16,
    tray: Option<Tray>,
    tap: Option<CGEventTap<'static>>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}

pub fn run(engine: Engine, settings: Settings, first_start: bool) {
    let mtm = MainThreadMarker::new().expect("Langflip starts on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    ask_for_access();

    // Autostart turns on by itself after install. Later starts keep the person's choice,
    // and rewrite the path in case the binary moved.
    let autostart = first_start || launch_agent().exists();
    if autostart {
        set_autostart(true);
    }
    let tray = Tray::new(settings.enabled, autostart);
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            engine,
            settings,
            settings_mtime: settings::modified(),
            front_pid: 0,
            skip_app: false,
            pending: false,
            held: Vec::new(),
            last_end_key: KEY_SPACE,
            tray: Some(tray),
            tap: None,
        })
    });

    let events = vec![
        CGEventType::KeyDown,
        CGEventType::KeyUp,
        CGEventType::LeftMouseDown,
        CGEventType::RightMouseDown,
        CGEventType::OtherMouseDown,
    ];
    match CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        events,
        on_event,
    ) {
        Ok(tap) => {
            let source = tap
                .mach_port()
                .create_runloop_source(0)
                .expect("the tap has a run loop source");
            CFRunLoop::get_current().add_source(&source, unsafe { kCFRunLoopCommonModes });
            tap.enable();
            with_state(|s| s.tap = Some(tap));
        }
        Err(()) => eprintln!("langflip: no event tap yet, allow Accessibility and restart"),
    }

    // The tray menu has no run loop hook of its own here, so a timer reads its clicks and the layout.
    let timer = CFRunLoopTimer::new(0.0, 0.2, 0, 0, poll_tray, std::ptr::null_mut());
    CFRunLoop::get_current().add_timer(&timer, unsafe { kCFRunLoopCommonModes });
    app.run();
}

/// Shows the system prompts for Accessibility and Input Monitoring on the first run.
fn ask_for_access() {
    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let options = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
        CGRequestListenEventAccess();
    }
}

extern "C" fn poll_tray(_timer: CFRunLoopTimerRef, _info: *mut c_void) {
    with_state(|s| s.tray.as_ref().map(|t| t.set_lang(current_lang())));
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
            let to = current_lang().map_or(Lang::En, Lang::other);
            switch_layout(to);
            with_state(|s| s.tray.as_ref().map(|t| t.set_lang(Some(to))));
        }
        Action::Autostart(on) => set_autostart(on),
        Action::OpenSettings => {
            let _ = std::process::Command::new("open")
                .arg("-t")
                .arg(settings::path())
                .spawn();
        }
        Action::Quit => std::process::exit(0),
    }
}

/// The LaunchAgent that starts Langflip at login. No prompt: macOS only shows a notice.
fn launch_agent() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("Library/LaunchAgents/com.workbenchsoft.langflip.plist")
}

fn set_autostart(on: bool) {
    let path = launch_agent();
    if !on {
        let _ = std::fs::remove_file(&path);
        return;
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let exe = exe
        .display()
        .to_string()
        .replace('&', "&amp;")
        .replace('<', "&lt;");
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.workbenchsoft.langflip</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"#
    );
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(err) = std::fs::write(&path, plist) {
        eprintln!("langflip: can't write {}: {err}", path.display());
    }
}

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

/// The key at this US position. Mac key codes follow the key, not the layout.
fn key_for_code(code: u16) -> Option<Key> {
    Some(match code {
        0x12 => Key::Digit1,
        0x13 => Key::Digit2,
        0x14 => Key::Digit3,
        0x15 => Key::Digit4,
        0x17 => Key::Digit5,
        0x16 => Key::Digit6,
        0x1A => Key::Digit7,
        0x1C => Key::Digit8,
        0x19 => Key::Digit9,
        0x1D => Key::Digit0,
        0x00 => Key::A,
        0x01 => Key::S,
        0x02 => Key::D,
        0x03 => Key::F,
        0x04 => Key::H,
        0x05 => Key::G,
        0x06 => Key::Z,
        0x07 => Key::X,
        0x08 => Key::C,
        0x09 => Key::V,
        0x0B => Key::B,
        0x0C => Key::Q,
        0x0D => Key::W,
        0x0E => Key::E,
        0x0F => Key::R,
        0x10 => Key::Y,
        0x11 => Key::T,
        0x1E => Key::BracketRight,
        0x1F => Key::O,
        0x20 => Key::U,
        0x21 => Key::BracketLeft,
        0x22 => Key::I,
        0x23 => Key::P,
        0x25 => Key::L,
        0x26 => Key::J,
        0x27 => Key::Quote,
        0x28 => Key::K,
        0x29 => Key::Semicolon,
        0x2B => Key::Comma,
        0x2D => Key::N,
        0x2E => Key::M,
        0x2F => Key::Period,
        0x32 => Key::Backquote,
        _ => return None,
    })
}

fn on_event(_proxy: CGEventTapProxy, kind: CGEventType, event: &CGEvent) -> CallbackResult {
    let drop = match kind {
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            with_state(|s| s.tap.as_ref().map(CGEventTap::enable));
            false
        }
        CGEventType::KeyDown | CGEventType::KeyUp => {
            let down = matches!(kind, CGEventType::KeyDown);
            with_state(|s| on_key(s, event, down)).unwrap_or(false)
        }
        _ => {
            with_state(|s| s.engine.reset());
            false
        }
    };
    if drop {
        CallbackResult::Drop
    } else {
        CallbackResult::Keep
    }
}

/// Returns true to drop the event.
fn on_key(s: &mut State, event: &CGEvent, down: bool) -> bool {
    let tag = event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA);
    let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
    let flags = event.get_flags();
    match tag {
        TAG_OWN => return false,
        TAG_END => {
            s.pending = false;
            for (code, down, flags) in std::mem::take(&mut s.held) {
                post_key(code, down, flags, TAG_REPLAY);
            }
            return true;
        }
        _ => {}
    }
    if s.pending && tag != TAG_REPLAY {
        s.held.push((code, down, flags));
        return true;
    }
    if !down {
        return false;
    }
    front_app_check(s);

    let control = flags.contains(CGEventFlags::CGEventFlagControl);
    let shift = flags.contains(CGEventFlags::CGEventFlagShift);
    if code == KEY_F12 && control {
        let fix = if shift {
            s.engine.convert(current_lang())
        } else {
            s.engine.undo()
        };
        if let Some(fix) = fix {
            s.settings.store_keep_words(&s.engine.keep_words);
            s.settings_mtime = settings::modified();
            // Only a finished word had an end key, which the fix erased and types again.
            let end = fix.retype_end.then_some(s.last_end_key);
            send_fix(s, fix, end);
            return true;
        }
        return false;
    }
    let secure = unsafe { IsSecureEventInputEnabled() };
    let shortcut = control
        || flags.contains(CGEventFlags::CGEventFlagCommand)
        || flags.contains(CGEventFlags::CGEventFlagAlternate);
    if !s.settings.enabled || s.skip_app || secure || shortcut {
        s.engine.reset();
        return false;
    }
    if code == KEY_DELETE {
        s.engine.backspace();
        return false;
    }
    // A digit with Shift types other signs in each layout, so it clears the word below.
    if let Some(key) = key_for_code(code).filter(|k| !(k.is_digit() && shift)) {
        let caps = flags.contains(CGEventFlags::CGEventFlagAlphaShift);
        if let Some(fix) = s.engine.key(Stroke::new(key, shift, caps), current_lang()) {
            send_fix(s, fix, None);
            return true;
        }
        return false;
    }
    if matches!(code, KEY_SPACE | KEY_RETURN | KEY_TAB) {
        s.last_end_key = code;
        if let Some(fix) = s.engine.end_word(current_lang()) {
            send_fix(s, fix, Some(code));
            return true;
        }
        return false;
    }
    s.engine.reset();
    false
}

/// A new app in front clears the word and rereads its exclusion.
fn front_app_check(s: &mut State) {
    let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
        return;
    };
    let pid = app.processIdentifier();
    if pid == s.front_pid {
        return;
    }
    s.front_pid = pid;
    s.engine.reset();
    reload_settings(s);
    let name = app
        .localizedName()
        .map(|n| n.to_string())
        .unwrap_or_default();
    s.skip_app = s.settings.is_excluded(&name);
}

fn source() -> Option<CGEventSource> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()
}

fn post_key(code: u16, down: bool, flags: CGEventFlags, tag: i64) {
    let Some(event) = source().and_then(|src| CGEvent::new_keyboard_event(src, code, down).ok())
    else {
        return;
    };
    event.set_flags(flags);
    event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, tag);
    event.post(CGEventTapLocation::Session);
}

fn post_text(units: &[u16]) {
    for down in [true, false] {
        let Some(event) = source().and_then(|src| CGEvent::new_keyboard_event(src, 0, down).ok())
        else {
            return;
        };
        event.set_string_from_utf16_unchecked(units);
        event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, TAG_OWN);
        event.post(CGEventTapLocation::Session);
    }
}

fn send_fix(s: &mut State, fix: Fix, end_key: Option<u16>) {
    s.pending = true;
    let none = CGEventFlags::CGEventFlagNull;
    for _ in 0..fix.erase {
        post_key(KEY_DELETE, true, none, TAG_OWN);
        post_key(KEY_DELETE, false, none, TAG_OWN);
    }
    // One event carries at most 20 UTF-16 units.
    let units: Vec<u16> = fix.text.encode_utf16().collect();
    for chunk in units.chunks(20) {
        post_text(chunk);
    }
    if let Some(key) = end_key {
        post_key(key, true, none, TAG_OWN);
        post_key(key, false, none, TAG_OWN);
    }
    post_key(KEY_MARKER, false, none, TAG_END);
    switch_layout(fix.switch_to);
}

fn lang_code(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "en",
        Lang::Ru => "ru",
    }
}

/// The first language a keyboard input source declares.
unsafe fn source_lang(source: *mut c_void) -> Option<String> {
    unsafe {
        let langs = TISGetInputSourceProperty(source, kTISPropertyInputSourceLanguages);
        if langs.is_null() {
            return None;
        }
        let langs: CFArray<CFType> = CFArray::wrap_under_get_rule(langs as CFArrayRef);
        let first = langs.get(0)?;
        first.downcast::<CFString>().map(|s| s.to_string())
    }
}

fn current_lang() -> Option<Lang> {
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return None;
        }
        let lang = source_lang(source);
        core_foundation::base::CFRelease(source);
        match lang.as_deref() {
            Some("en") => Some(Lang::En),
            Some("ru") => Some(Lang::Ru),
            _ => None,
        }
    }
}

/// Selects the first enabled keyboard source of this language.
fn switch_layout(lang: Lang) {
    unsafe {
        let key = CFString::wrap_under_get_rule(kTISPropertyInputSourceIsSelectCapable);
        let filter = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
        let list = TISCreateInputSourceList(filter.as_concrete_TypeRef(), false);
        if list.is_null() {
            return;
        }
        let list: CFArray<CFType> = CFArray::wrap_under_create_rule(list);
        for item in list.iter() {
            let source = item.as_CFTypeRef() as *mut c_void;
            if source_lang(source).as_deref() == Some(lang_code(lang)) {
                TISSelectInputSource(source);
                return;
            }
        }
    }
}
