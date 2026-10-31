//! Langflip: fixes words typed in the wrong keyboard layout, Russian and English.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod autostart;
mod language;
mod settings;
mod tray;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

use langflip_core::{Engine, Model};

rust_i18n::i18n!("locales", fallback = "en");

static MODEL: &[u8] = include_bytes!("../../../model/langflip.model");

fn main() {
    let (settings, first_start) = settings::Settings::load_first();
    language::apply(settings.language.as_deref());
    let model = Model::from_bytes(MODEL).expect("the embedded model is valid");
    let engine = Engine::new(model, settings.keep_set());

    #[cfg(windows)]
    windows::run(engine, settings, first_start);
    #[cfg(target_os = "macos")]
    macos::run(engine, settings, first_start);
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (engine, settings, first_start);
        eprintln!("langflip runs on Windows and macOS only");
    }
}
