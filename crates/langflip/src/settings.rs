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
    /// When the file changed as this copy last read or saved it, so an edit by hand gets picked up.
    #[serde(skip)]
    mtime: Option<SystemTime>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            enabled: true,
            excluded_apps: Vec::new(),
            keep_words: Vec::new(),
            language: None,
            read_only: false,
            mtime: None,
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

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl Settings {
    /// Reads the file, or writes the defaults when there is none yet. True when there was none:
    /// the first start after install.
    pub fn load_first() -> (Settings, bool) {
        let first = !path().exists();
        (Settings::load_from(&path()), first)
    }

    /// Reads the file again when it changed since this copy read or saved it. True when it did.
    pub fn reload(&mut self) -> bool {
        self.reload_from(&path())
    }

    fn reload_from(&mut self, path: &Path) -> bool {
        if modified(path) == self.mtime {
            return false;
        }
        // An editor that holds the file while it saves: keep this copy and try at the next change.
        if let Err(err) = std::fs::File::open(path)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            report(format_args!("can't read {} yet: {err}", path.display()));
            return false;
        }
        *self = Settings::load_from(path);
        true
    }

    fn load_from(path: &Path) -> Settings {
        let mut settings = match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|err| {
                report(format_args!(
                    "{} is not valid, using defaults until it is fixed: {}",
                    path.display(),
                    problem(&text, &err)
                ));
                Settings {
                    read_only: true,
                    ..Settings::default()
                }
            }),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let mut settings = Settings::default();
                settings.save_to(path);
                settings
            }
            // Not UTF-8, or locked by another program: the person's text stays as it is.
            Err(err) => {
                report(format_args!(
                    "can't read {}, using defaults until it is fixed: {err}",
                    path.display()
                ));
                Settings {
                    read_only: true,
                    ..Settings::default()
                }
            }
        };
        settings.mtime = modified(path);
        settings
    }

    pub fn save(&mut self) {
        self.save_to(&path());
    }

    fn save_to(&mut self, path: &Path) {
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
                match written {
                    Ok(()) => self.mtime = modified(path),
                    Err(err) => {
                        let _ = std::fs::remove_file(&temp);
                        report(format_args!("can't save {}: {err}", path.display()));
                    }
                }
            }
            Err(err) => report(format_args!("can't encode settings: {err}")),
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

/// Where the file breaks and why. The full error quotes the line, which can hold a kept word.
fn problem(text: &str, err: &toml::de::Error) -> String {
    let message = without_strings(err.message());
    match err.span() {
        Some(span) => {
            let line = text[..span.start.min(text.len())].matches('\n').count() + 1;
            format!("line {line}, {message}")
        }
        None => message,
    }
}

/// A wrong type quotes the value, as `string "ghbdtn"`, so each quoted string loses its text.
fn without_strings(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let (mut quoted, mut escaped) = (false, false);
    for c in message.chars() {
        if quoted && escaped {
            escaped = false;
        } else if quoted && c == '\\' {
            escaped = true;
        } else if c == '"' {
            quoted = !quoted;
            out.push(c);
        } else if !quoted {
            out.push(c);
        }
    }
    out
}

/// The release exe on Windows has no console, so a problem goes to the decision log there.
fn report(line: std::fmt::Arguments) {
    #[cfg(all(windows, not(test)))]
    crate::log::write(line);
    #[cfg(any(not(windows), test))]
    eprintln!("langflip: {line}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file in the temp folder, removed when the test ends.
    struct TempFile(PathBuf);

    impl TempFile {
        fn new(name: &str) -> TempFile {
            let file = format!("langflip-test-{}-{name}", std::process::id());
            TempFile(std::env::temp_dir().join(file))
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn keeps_a_file_it_cannot_read() {
        let file = TempFile::new("broken.toml");
        let path = &file.0;
        let text = "keep_words = [\"ghbdtn\"]
excluded_apps = [\"wow.exe\"
";
        std::fs::write(path, text).unwrap();
        let mut settings = Settings::load_from(path);
        assert!(settings.keep_words.is_empty());
        settings.enabled = false;
        settings.keep_words.push("руддщ".to_owned());
        settings.save_to(path);
        assert_eq!(std::fs::read_to_string(path).unwrap(), text);
    }

    #[test]
    fn keeps_a_file_that_is_not_utf8() {
        let file = TempFile::new("cp1251.toml");
        let path = &file.0;
        // An editor that saved the Russian word 'руддщ' in Windows-1251.
        let bytes = b"keep_words = [\"\xf0\xf3\xe4\xe4\xf9\"]\n";
        std::fs::write(path, bytes).unwrap();
        let mut settings = Settings::load_from(path);
        settings.enabled = false;
        settings.save_to(path);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn writes_the_defaults_when_there_is_no_file() {
        let file = TempFile::new("missing.toml");
        let path = &file.0;
        let _ = std::fs::remove_file(path);
        Settings::load_from(path);
        assert!(path.exists());
    }

    #[test]
    fn names_the_broken_line_without_its_words() {
        let text = "keep_words = [\"ghbdtn\"]
excluded_apps = [\"wow.exe\" \"x\"]
";
        let err = toml::from_str::<Settings>(text).err().unwrap();
        let problem = problem(text, &err);
        assert!(problem.starts_with("line 2, "), "{problem}");
        assert!(
            !problem.contains("ghbdtn") && !problem.contains("wow.exe"),
            "{problem}"
        );
    }

    #[test]
    fn names_a_wrong_type_without_its_word() {
        for text in ["keep_words = \"ghbdtn\"\n", "enabled = \"ghbdtn\"\n"] {
            let err = toml::from_str::<Settings>(text).err().unwrap();
            let problem = problem(text, &err);
            assert!(problem.starts_with("line 1, "), "{problem}");
            assert!(!problem.contains("ghbdtn"), "{problem}");
        }
    }

    #[test]
    fn saves_and_reads_back() {
        let file = TempFile::new("saved.toml");
        let path = &file.0;
        let mut settings = Settings {
            enabled: false,
            keep_words: vec!["руддщ".to_owned()],
            language: Some("de".to_owned()),
            ..Settings::default()
        };
        settings.save_to(path);
        settings.keep_words.push("ghbdtn".to_owned());
        settings.save_to(path);
        let read = Settings::load_from(path);
        assert!(!read.enabled);
        assert_eq!(read.keep_words, ["руддщ", "ghbdtn"]);
        assert_eq!(read.language.as_deref(), Some("de"));
        assert!(!path.with_extension("toml.tmp").exists());
    }

    #[cfg(windows)]
    #[test]
    fn keeps_its_words_while_an_editor_locks_the_file() {
        use std::os::windows::fs::OpenOptionsExt;
        let file = TempFile::new("locked.toml");
        let path = &file.0;
        std::fs::write(
            path,
            "keep_words = [\"ghbdtn\"]
",
        )
        .unwrap();
        let mut settings = Settings::load_from(path);
        let later = SystemTime::now() + std::time::Duration::from_secs(10);
        let edit = std::fs::File::options().write(true).open(path).unwrap();
        edit.set_modified(later).unwrap();
        drop(edit);
        // No sharing: the next open fails as it does while an editor saves.
        let lock = std::fs::File::options()
            .read(true)
            .share_mode(0)
            .open(path)
            .unwrap();
        assert!(!settings.reload_from(path));
        assert_eq!(settings.keep_words, ["ghbdtn"]);
        drop(lock);
        assert!(settings.reload_from(path));
    }

    #[test]
    fn reloads_an_edit_by_hand_only() {
        let file = TempFile::new("edited.toml");
        let path = &file.0;
        let mut settings = Settings::load_from(path);
        settings.enabled = false;
        settings.save_to(path);
        assert!(!settings.reload_from(path));
        std::fs::write(
            path,
            "enabled = true
",
        )
        .unwrap();
        // A later time than the save, however coarse the file system clock.
        let later = SystemTime::now() + std::time::Duration::from_secs(10);
        let edited = std::fs::File::options().write(true).open(path).unwrap();
        edited.set_modified(later).unwrap();
        assert!(settings.reload_from(path));
        assert!(settings.enabled);
        assert!(!settings.reload_from(path));
    }
}
