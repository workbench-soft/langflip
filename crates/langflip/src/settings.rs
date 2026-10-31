//! The settings file: `settings.toml` in the per-user app data folder.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Fixing on or off. The tray menu flips it.
    pub enabled: bool,
    /// Executable names Langflip leaves alone, compared without case: `wow.exe`, `Terminal`.
    pub excluded_apps: Vec<String>,
    /// Words to leave as typed. The second Undo of a word adds it.
    pub keep_words: Vec<String>,
    /// The menu language, such as `de`. None follows the system language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// The file didn't parse. Saving would replace the person's text with the defaults.
    #[serde(skip)]
    read_only: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            enabled: true,
            excluded_apps: Vec::new(),
            keep_words: Vec::new(),
            language: None,
            read_only: false,
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
        Settings::load_first().0
    }

    /// [`Settings::load`], plus true when the file did not exist: the first start after install.
    pub fn load_first() -> (Settings, bool) {
        let first = !path().exists();
        (Settings::load_existing(), first)
    }

    fn load_existing() -> Settings {
        Settings::load_from(&path())
    }

    fn load_from(path: &Path) -> Settings {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|err| {
                eprintln!(
                    "langflip: {} is not valid, using defaults until it is fixed: {err}",
                    path.display()
                );
                Settings {
                    read_only: true,
                    ..Settings::default()
                }
            }),
            Err(_) => {
                let settings = Settings::default();
                settings.save_to(path);
                settings
            }
        }
    }

    pub fn save(&self) {
        self.save_to(&path());
    }

    fn save_to(&self, path: &Path) {
        if self.read_only {
            return;
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match toml::to_string_pretty(self) {
            Ok(text) => {
                // A crash halfway through a write leaves the old file whole, not half of the new one.
                let temp = path.with_extension("toml.tmp");
                let written =
                    std::fs::write(&temp, text).and_then(|()| std::fs::rename(&temp, path));
                if let Err(err) = written {
                    let _ = std::fs::remove_file(&temp);
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

    /// Copies the words Undo added back into the file. Returns true when the list changed.
    pub fn store_keep_words(&mut self, words: &HashSet<String>) -> bool {
        let mut list: Vec<String> = words.iter().cloned().collect();
        list.sort();
        if list == self.keep_words {
            return false;
        }
        self.keep_words = list;
        self.save();
        true
    }

    /// Lets Langflip fix the word again: the person removed it in the tray menu.
    pub fn remove_keep_word(&mut self, word: &str) {
        let word = word.to_lowercase();
        self.keep_words.retain(|w| w.to_lowercase() != word);
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("langflip-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn keeps_a_file_it_cannot_read() {
        let path = temp_file("broken.toml");
        let text = "keep_words = [\"ghbdtn\"]
excluded_apps = [\"wow.exe\"
";
        std::fs::write(&path, text).unwrap();
        let mut settings = Settings::load_from(&path);
        assert!(settings.keep_words.is_empty());
        settings.enabled = false;
        settings.keep_words.push("руддщ".to_owned());
        settings.save_to(&path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }

    #[test]
    fn saves_and_reads_back() {
        let path = temp_file("saved.toml");
        let mut settings = Settings {
            enabled: false,
            keep_words: vec!["руддщ".to_owned()],
            language: Some("de".to_owned()),
            ..Settings::default()
        };
        settings.save_to(&path);
        settings.keep_words.push("ghbdtn".to_owned());
        settings.save_to(&path);
        let read = Settings::load_from(&path);
        assert!(!read.enabled);
        assert_eq!(read.keep_words, ["руддщ", "ghbdtn"]);
        assert_eq!(read.language.as_deref(), Some("de"));
        assert!(!path.with_extension("toml.tmp").exists());
    }
}
