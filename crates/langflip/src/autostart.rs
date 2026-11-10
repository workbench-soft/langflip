//! Windows autostart through 2 paths: the Run key and a Task Scheduler task.
//! Explorer can skip a Run value at sign in, and the task still starts Langflip then.

use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;
use windows::core::{PCWSTR, w};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const TASK: &str = "Langflip";

/// Either path counts, so the next `set(true)` puts back the one that went missing.
pub fn enabled() -> bool {
    let run = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            w!("Langflip"),
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        )
    };
    run.is_ok() || schtasks(&["/Query", "/TN", TASK])
}

pub fn set(on: bool) {
    let exe = std::env::current_exe()
        .unwrap_or_default()
        .display()
        .to_string();
    unsafe {
        if on {
            let value: Vec<u16> = format!("\"{exe}\"").encode_utf16().chain([0]).collect();
            let _ = RegSetKeyValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                w!("Langflip"),
                REG_SZ.0,
                Some(value.as_ptr() as *const _),
                (value.len() * 2) as u32,
            );
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, w!("Langflip"));
        }
    }
    if on {
        create_task(&exe);
    } else {
        schtasks(&["/Delete", "/TN", TASK, "/F"]);
    }
}

fn create_task(exe: &str) {
    let user = format!(
        "{}\\{}",
        std::env::var("USERDOMAIN").unwrap_or_default(),
        std::env::var("USERNAME").unwrap_or_default()
    );
    // schtasks reads the file as UTF-16 with a byte order mark.
    let bytes: Vec<u8> = std::iter::once(0xFEFF)
        .chain(task_xml(exe, &user).encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect();
    let path = std::env::temp_dir().join("langflip-task.xml");
    if std::fs::write(&path, bytes).is_ok() {
        schtasks(&[
            "/Create",
            "/TN",
            TASK,
            "/XML",
            &path.to_string_lossy(),
            "/F",
        ]);
        let _ = std::fs::remove_file(&path);
    }
}

fn schtasks(args: &[&str]) -> bool {
    Command::new("schtasks.exe")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // Keeps schtasks.exe from flashing a console window.
        .creation_flags(CREATE_NO_WINDOW.0)
        .status()
        .is_ok_and(|s| s.success())
}

/// Starts at sign in, then each minute for 15 minutes. A copy already running exits on the
/// mutex, and a Quit after the 15 minutes holds until the next sign in.
fn task_xml(exe: &str, user: &str) -> String {
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let (exe, user) = (esc(exe), esc(user));
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers><LogonTrigger><UserId>{user}</UserId>
    <Repetition><Interval>PT1M</Interval><Duration>PT15M</Duration></Repetition>
  </LogonTrigger></Triggers>
  <Principals><Principal id="Author"><UserId>{user}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author"><Exec><Command>{exe}</Command></Exec></Actions>
</Task>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_xml_escapes_the_path() {
        let xml = task_xml(r"C:\A&B\langflip.exe", r"PC\ann");
        assert!(xml.contains(r"<Command>C:\A&amp;B\langflip.exe</Command>"));
        assert!(xml.contains(r"<UserId>PC\ann</UserId>"));
    }
}
