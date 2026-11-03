//! Windows: asks GitHub for a newer Langflip at start and about once a day, and swaps the exe on Update.

use std::time::{Duration, SystemTime};

use rust_i18n::t;
use serde::Deserialize;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Controls::{TD_INFORMATION_ICON, TD_WARNING_ICON};
use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};

use crate::dialog::{self, ID_LATER, ID_UPDATE};

const RELEASES: &str =
    "https://api.github.com/repos/workbench-soft/workbench-soft.github.io/releases?per_page=100";
/// The other apps publish to the same repo, so only these tags count.
const TAG: &str = "langflip-v";
const VERSION: &str = env!("LANGFLIP_VERSION");
const USER_AGENT: &str = concat!("Langflip/", env!("LANGFLIP_VERSION"));
const DAY: Duration = Duration::from_secs(24 * 60 * 60);
const HOUR: Duration = Duration::from_secs(60 * 60);
/// The exe is a few MB. The cap guards against a wrong file.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// A release newer than the running Langflip.
struct Update {
    version: String,
    size: u64,
    url: String,
}

fn numbers(version: &str) -> Vec<u32> {
    version.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// The newest release with an exe, when it beats `current`.
fn newest(releases: Vec<Release>, current: &str) -> Option<Update> {
    releases
        .into_iter()
        .filter(|r| !r.draft)
        .filter_map(|r| {
            let version = r.tag_name.strip_prefix(TAG)?.to_owned();
            let exe = r.assets.into_iter().find(|a| a.name.ends_with(".exe"))?;
            Some(Update {
                version,
                size: exe.size,
                url: exe.browser_download_url,
            })
        })
        .max_by_key(|u| numbers(&u.version))
        .filter(|u| numbers(&u.version) > numbers(current))
}

/// Starts the checks. A dev build has the Cargo version and skips them, so the smoke test sees no dialog.
pub fn start(main_thread: u32) {
    if numbers(VERSION)[0] >= 2000 {
        std::thread::spawn(move || run(main_thread));
    }
}

fn run(main_thread: u32) {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into();
    // Later stays quiet about its version until the next start.
    let mut declined = None;
    let mut next = SystemTime::now();
    loop {
        // Short naps, so a PC that slept through the day checks soon after it wakes.
        std::thread::sleep(Duration::from_secs(60));
        if SystemTime::now() < next {
            continue;
        }
        next = SystemTime::now() + check(&agent, &mut declined, main_thread);
    }
}

/// One check, and the wait until the next one. Offline or a GitHub error shows nothing.
fn check(agent: &ureq::Agent, declined: &mut Option<String>, main_thread: u32) -> Duration {
    let latest = agent
        .get(RELEASES)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .call()
        .and_then(|mut r| r.body_mut().read_json::<Vec<Release>>());
    let update = match latest {
        Ok(releases) => newest(releases, VERSION),
        Err(e) => {
            crate::log::write(format_args!("update check failed: {e}"));
            return HOUR;
        }
    };
    let Some(update) = update.filter(|u| declined.as_ref() != Some(&u.version)) else {
        return DAY;
    };
    crate::log::write(format_args!("update {} is out", update.version));
    let mb = format!("{:.1}", update.size as f64 / 1e6);
    let buttons = [
        (ID_UPDATE, t!("update.now").into_owned()),
        (ID_LATER, t!("update.later").into_owned()),
    ];
    let main = t!("update.found", version = update.version);
    if dialog::show(
        &main,
        &t!("update.details", size = mb),
        &buttons,
        TD_INFORMATION_ICON,
    ) != ID_UPDATE
    {
        *declined = Some(update.version);
        return DAY;
    }
    match install(agent, &update) {
        Ok(()) => {
            crate::log::write(format_args!("updated to {}, restarting", update.version));
            // The new copy waits for this one to quit and drop its mutex.
            let _ = unsafe { PostThreadMessageW(main_thread, WM_QUIT, WPARAM(0), LPARAM(0)) };
        }
        Err(e) => {
            crate::log::write(format_args!("update to {} failed: {e}", update.version));
            dialog::show(&t!("update.failed"), "", &[], TD_WARNING_ICON);
        }
    }
    DAY
}

/// Downloads the exe, puts it in place of the running one and starts it.
fn install(agent: &ureq::Agent, update: &Update) -> Result<(), Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let bytes = agent
        .get(&update.url)
        .header("User-Agent", USER_AGENT)
        .call()?
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()?;
    if bytes.len() as u64 != update.size {
        return Err(format!("got {} of {} bytes", bytes.len(), update.size).into());
    }
    let file = std::env::temp_dir().join(format!("langflip-{}.exe", update.version));
    std::fs::write(&file, bytes)?;
    // Windows locks a running exe. self-replace moves it aside first.
    let swapped = self_replace::self_replace(&file);
    let _ = std::fs::remove_file(&file);
    swapped?;
    std::process::Command::new(exe).arg("--updated").spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, draft: bool, asset: &str) -> Release {
        Release {
            tag_name: tag.to_owned(),
            draft,
            assets: vec![Asset {
                name: asset.to_owned(),
                size: 3,
                browser_download_url: format!("https://example.test/{asset}"),
            }],
        }
    }

    #[test]
    fn picks_the_newest_langflip_exe() {
        let releases = vec![
            release(
                "langflip-v2026.11.02.1",
                false,
                "Langflip-2026.11.02.1-windows-x64.exe",
            ),
            release(
                "langflip-v2026.11.10.1",
                false,
                "Langflip-2026.11.10.1-windows-x64.exe",
            ),
            release(
                "langflip-v2026.11.09.2",
                false,
                "Langflip-2026.11.09.2-windows-x64.exe",
            ),
            release("memoire-v2026.12.01.1", false, "Memoire.exe"),
            release(
                "langflip-v2026.12.01.1",
                true,
                "Langflip-2026.12.01.1-windows-x64.exe",
            ),
            release("langflip-v2026.12.02.1", false, "notes.txt"),
        ];
        let update = newest(releases, "2026.11.02.1").unwrap();
        assert_eq!(update.version, "2026.11.10.1");
        assert!(
            update
                .url
                .ends_with("Langflip-2026.11.10.1-windows-x64.exe")
        );
    }

    #[test]
    fn the_same_or_an_older_release_is_no_update() {
        let releases = || vec![release("langflip-v2026.11.02.1", false, "a.exe")];
        assert!(newest(releases(), "2026.11.02.1").is_none());
        assert!(newest(releases(), "2026.11.10.1").is_none());
    }

    #[test]
    fn compares_numbers_not_text() {
        let releases = || vec![release("langflip-v2026.11.03.1", false, "a.exe")];
        // `mise run install` writes the day without a leading zero and a 0 counter.
        assert!(newest(releases(), "2026.11.3.0").is_some());
        assert!(newest(releases(), "2026.11.3").is_some());
        assert!(newest(releases(), "2026.9.30.4").is_some());
    }
}
