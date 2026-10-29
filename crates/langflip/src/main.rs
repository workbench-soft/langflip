//! Langflip: fixes words typed in the wrong keyboard layout, Russian and English.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod settings;
mod tray;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

use langflip_core::{Engine, Model};

static MODEL: &[u8] = include_bytes!("../../../model/langflip.model");

fn main() {
    let settings = settings::Settings::load();
    let model = Model::from_bytes(MODEL).expect("the embedded model is valid");
    let engine = Engine::new(model, settings.keep_set());

    #[cfg(windows)]
    windows::run(engine, settings);
    #[cfg(target_os = "macos")]
    macos::run(engine, settings);
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (engine, settings);
        eprintln!("langflip runs on Windows and macOS only");
    }
}
