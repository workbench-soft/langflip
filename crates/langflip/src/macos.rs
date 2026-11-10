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
    CGEventTapProxy, CGEventType, CallbackResult, EventField, KeyCode,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use langflip_core::{Engine, Fix, Key, Lang, Stroke};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSWorkspace};

use crate::language;
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
    tray.set_language(settings.language.as_deref(), &settings.keep_words);
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            engine,
            settings,
            settings_mtime: settings::modified(),
            front_pid: 0,
            skip_app: false,
            pending: false,
            held: Vec::new(),
            last_end_key: KeyCode::SPACE,
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
        Action::RemoveKeepWord(word) => {
            with_state(|s| {
                s.settings.remove_keep_word(&word);
                s.settings_mtime = settings::modified();
                s.engine.keep_words = s.settings.keep_set();
                show_keep_words(s);
            });
        }
        Action::SetLanguage(choice) => {
            with_state(|s| {
                s.settings.language = choice;
                s.settings.save();
                s.settings_mtime = settings::modified();
                show_language(s);
            });
        }
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

/// The key at this US position. Mac key codes follow the key, not the layout.
fn key_for_code(code: u16) -> Option<Key> {
    Some(match code {
        KeyCode::ANSI_1 => Key::Digit1,
        KeyCode::ANSI_2 => Key::Digit2,
        KeyCode::ANSI_3 => Key::Digit3,
        KeyCode::ANSI_4 => Key::Digit4,
        KeyCode::ANSI_5 => Key::Digit5,
        KeyCode::ANSI_6 => Key::Digit6,
        KeyCode::ANSI_7 => Key::Digit7,
        KeyCode::ANSI_8 => Key::Digit8,
        KeyCode::ANSI_9 => Key::Digit9,
        KeyCode::ANSI_0 => Key::Digit0,
        KeyCode::ANSI_A => Key::A,
        KeyCode::ANSI_S => Key::S,
        KeyCode::ANSI_D => Key::D,
        KeyCode::ANSI_F => Key::F,
        KeyCode::ANSI_H => Key::H,
        KeyCode::ANSI_G => Key::G,
        KeyCode::ANSI_Z => Key::Z,
        KeyCode::ANSI_X => Key::X,
        KeyCode::ANSI_C => Key::C,
        KeyCode::ANSI_V => Key::V,
        KeyCode::ANSI_B => Key::B,
        KeyCode::ANSI_Q => Key::Q,
        KeyCode::ANSI_W => Key::W,
        KeyCode::ANSI_E => Key::E,
        KeyCode::ANSI_R => Key::R,
        KeyCode::ANSI_Y => Key::Y,
        KeyCode::ANSI_T => Key::T,
        KeyCode::ANSI_RIGHT_BRACKET => Key::BracketRight,
        KeyCode::ANSI_O => Key::O,
        KeyCode::ANSI_U => Key::U,
        KeyCode::ANSI_LEFT_BRACKET => Key::BracketLeft,
        KeyCode::ANSI_I => Key::I,
        KeyCode::ANSI_P => Key::P,
        KeyCode::ANSI_L => Key::L,
        KeyCode::ANSI_J => Key::J,
        KeyCode::ANSI_QUOTE => Key::Quote,
        KeyCode::ANSI_K => Key::K,
        KeyCode::ANSI_SEMICOLON => Key::Semicolon,
        KeyCode::ANSI_COMMA => Key::Comma,
        KeyCode::ANSI_N => Key::N,
        KeyCode::ANSI_M => Key::M,
        KeyCode::ANSI_PERIOD => Key::Period,
        KeyCode::ANSI_GRAVE => Key::Backquote,
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
    if code == KeyCode::F12 && control {
        let fix = if shift {
            s.engine.convert(current_lang())
        } else {
            s.engine.undo()
        };
        if let Some(fix) = fix {
            if s.settings.store_keep_words(&s.engine.keep_words) {
                show_keep_words(s);
            }
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
    if code == KeyCode::DELETE {
        s.engine.backspace(s.last_end_key == KeyCode::SPACE);
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
    if matches!(code, KeyCode::SPACE | KeyCode::RETURN | KeyCode::TAB) {
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
        post_key(KeyCode::DELETE, true, none, TAG_OWN);
        post_key(KeyCode::DELETE, false, none, TAG_OWN);
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
