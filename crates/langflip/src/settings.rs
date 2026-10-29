//! The settings file: `settings.toml` in the per-user app data folder.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Fixing on or off. The tray menu flips it.
    pub enabled: bool,
    /// Executable names Langflip leaves alone, compared without case: `wow.exe`, `Terminal`.
    pub excluded_apps: Vec<String>,
    /// Words to leave as typed. Undo adds to it.
    pub keep_words: Vec<String>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            enabled: true,
            excluded_apps: Vec::new(),
            keep_words: Vec::new(),
        }
    }
}

pub fn path() -> PathBuf {
    #[cfg(windows)]
    let base = PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default());
    #[cfg(not(windows))]
    let base = PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("Library/Application Support");
    base.join("Langflip").join("settings.toml")
}

/// When the file last changed, so an edit by hand gets picked up.
pub fn modified() -> Option<SystemTime> {
    std::fs::metadata(path()).and_then(|m| m.modified()).ok()
}

impl Settings {
    /// Reads the file, or writes the defaults when there is none yet.
    pub fn load() -> Settings {
        match std::fs::read_to_string(path()) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|err| {
                eprintln!(
                    "langflip: {} is not valid, using defaults: {err}",
                    path().display()
                );
                Settings::default()
            }),
            Err(_) => {
                let settings = Settings::default();
                settings.save();
                settings
            }
        }
    }

    pub fn save(&self) {
        let path = path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match toml::to_string_pretty(self) {
            Ok(text) => {
                if let Err(err) = std::fs::write(&path, text) {
                    eprintln!("langflip: can't save {}: {err}", path.display());
                }
            }
            Err(err) => eprintln!("langflip: can't encode settings: {err}"),
        }
    }

    pub fn keep_set(&self) -> HashSet<String> {
        self.keep_words.iter().map(|w| w.to_lowercase()).collect()
    }

    pub fn is_excluded(&self, app: &str) -> bool {
        self.excluded_apps
            .iter()
            .any(|a| a.eq_ignore_ascii_case(app))
    }

    /// Copies the words Undo added back into the file.
    pub fn store_keep_words(&mut self, words: &HashSet<String>) {
        let mut list: Vec<String> = words.iter().cloned().collect();
        list.sort();
        if list != self.keep_words {
            self.keep_words = list;
            self.save();
        }
    }
}
