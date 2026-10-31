//! The decision log: `%LOCALAPPDATA%\Langflip\langflip.log`, in every build.
//!
//! It holds decisions, key counts and app names. The keys of a word stay out.

use std::io::Write;
use std::path::PathBuf;

use windows::Win32::System::SystemInformation::GetLocalTime;

/// At this size the log moves to `langflip.old.log` and starts again.
const MAX_BYTES: u64 = 1024 * 1024;

fn dir() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("Langflip")
}

/// Appends a line with the local time in front.
pub fn write(line: std::fmt::Arguments) {
    let dir = dir();
    let path = dir.join("langflip.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() >= MAX_BYTES) {
        let _ = std::fs::rename(&path, dir.join("langflip.old.log"));
    }
    let _ = std::fs::create_dir_all(&dir);
    let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let t = unsafe { GetLocalTime() };
    let _ = writeln!(
        f,
        "{}-{:02}-{:02} {:02}:{:02}:{:02}.{:03} {line}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    );
}
