//! The decision log: `%LOCALAPPDATA%\Langflip\langflip.log`, in every build.
//!
//! It holds decisions, key counts and app names. The keys of a word stay out.

use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::mpsc::{Sender, SyncSender, channel, sync_channel};

use windows::Win32::System::SystemInformation::GetLocalTime;

/// At this size the log moves to `langflip.old.log` and starts again.
const MAX_BYTES: u64 = 1024 * 1024;

enum Entry {
    Line(String),
    /// Answers once every line sent before it is in the file.
    Flush(SyncSender<()>),
}

/// The keyboard hook logs each word end, and a file write there takes 1 to 500 ms.
/// So the lines go to a thread of their own, which writes them in order.
static WRITER: OnceLock<Sender<Entry>> = OnceLock::new();

fn writer() -> &'static Sender<Entry> {
    WRITER.get_or_init(|| {
        let (send, receive) = channel();
        std::thread::spawn(move || {
            for entry in receive {
                match entry {
                    Entry::Line(line) => append(&line),
                    Entry::Flush(done) => {
                        let _ = done.send(());
                    }
                }
            }
        });
        send
    })
}

fn dir() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("Langflip")
}

/// Queues a line with the local time in front.
pub fn write(line: std::fmt::Arguments) {
    let t = unsafe { GetLocalTime() };
    let line = format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02}.{:03} {line}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    );
    let _ = writer().send(Entry::Line(line));
}

/// Waits until the queued lines are in the file, so the last ones survive a quit.
pub fn flush() {
    let Some(writer) = WRITER.get() else {
        return;
    };
    let (done, wait) = sync_channel(1);
    if writer.send(Entry::Flush(done)).is_ok() {
        let _ = wait.recv();
    }
}

fn append(line: &str) {
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
    let _ = writeln!(f, "{line}");
}
