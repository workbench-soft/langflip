//! Windows: asks GitHub for a newer Langflip at start and about once a day, and swaps the exe on Update.

use std::time::{Duration, SystemTime};

use rust_i18n::t;
use serde::Deserialize;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::Security::Cryptography::{BCRYPT_SHA256_ALG_HANDLE, BCryptHash};
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
/// A stalled connection gives up after this long without a byte.
const STALL: Duration = Duration::from_secs(30);
/// The 6 MB exe gets this long in all, so a link of 64 kbit/s still finishes.
const TOTAL: Duration = Duration::from_secs(15 * 60);
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
    /// GitHub works it out on upload, as `sha256:<hex>`.
    #[serde(default)]
    digest: Option<String>,
}

/// A release newer than the running Langflip.
struct Update {
    version: String,
    size: u64,
    url: String,
    digest: Option<String>,
}

fn numbers(version: &str) -> Vec<u32> {
    version.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// The exe the release workflow attaches to each release.
fn exe_name(version: &str) -> String {
    format!("Langflip-{version}-windows-x64.exe")
}

/// The newest release with the x64 exe the release workflow names, when it beats `current`.
/// The version also names a file in %TEMP%, so it holds only digits and dots.
fn newest(releases: Vec<Release>, current: &str) -> Option<Update> {
    releases
        .into_iter()
        .filter(|r| !r.draft)
        .filter_map(|r| {
            let version = r.tag_name.strip_prefix(TAG)?.to_owned();
            if version.is_empty() || !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return None;
            }
            let name = exe_name(&version);
            let exe = r.assets.into_iter().find(|a| a.name == name)?;
            Some(Update {
                version,
                size: exe.size,
                url: exe.browser_download_url,
                digest: exe.digest,
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

/// GitHub sends the exe through a redirect. A redirect to plain http fails instead of downloading.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .https_only(true)
        .timeout_connect(Some(STALL))
        .timeout_per_read(Some(STALL))
        .timeout_global(Some(TOTAL))
        .build()
        .into()
}

fn run(main_thread: u32) {
    let agent = agent();
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
    verify(&bytes, update.digest.as_deref())?;
    let file = std::env::temp_dir().join(format!("langflip-{}.exe", update.version));
    std::fs::write(&file, bytes)?;
    // Windows locks a running exe. self-replace moves it aside first.
    let swapped = self_replace::self_replace(&file);
    let _ = std::fs::remove_file(&file);
    swapped?;
    std::process::Command::new(exe).arg("--updated").spawn()?;
    Ok(())
}

/// The bytes match the SHA-256 GitHub lists for the asset. A release without one installs nothing.
fn verify(bytes: &[u8], digest: Option<&str>) -> Result<(), String> {
    let want = digest
        .and_then(|d| d.strip_prefix("sha256:"))
        .ok_or("the release lists no SHA-256")?;
    let mut hash = [0u8; 32];
    unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, None, bytes, &mut hash) }
        .ok()
        .map_err(|e| e.to_string())?;
    let got: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    if got.eq_ignore_ascii_case(want) {
        Ok(())
    } else {
        Err(format!("SHA-256 {got} is not {want}"))
    }
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
                digest: None,
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
        let releases = || {
            vec![release(
                "langflip-v2026.11.02.1",
                false,
                "Langflip-2026.11.02.1-windows-x64.exe",
            )]
        };
        assert!(newest(releases(), "2026.11.02.1").is_none());
        assert!(newest(releases(), "2026.11.10.1").is_none());
    }

    #[test]
    fn skips_another_exe_and_a_tag_that_is_no_version() {
        let releases = vec![
            release(
                "langflip-v2026.11.10.1",
                false,
                "Langflip-2026.11.10.1-windows-arm64.exe",
            ),
            release(
                "langflip-v../../evil",
                false,
                "Langflip-../../evil-windows-x64.exe",
            ),
        ];
        assert!(newest(releases, "2026.11.02.1").is_none());
    }

    #[test]
    fn installs_only_bytes_with_the_listed_sha256() {
        let abc = "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify(b"abc", Some(abc)).is_ok());
        assert!(verify(b"abd", Some(abc)).is_err());
        assert!(verify(b"abc", None).is_err());
        assert!(verify(b"abc", Some("md5:900150983cd24fb0d6963f7d28e17f72")).is_err());
    }

    #[test]
    fn refuses_plain_http() {
        let err = agent().get("http://127.0.0.1:9/langflip.exe").call();
        assert!(matches!(err, Err(ureq::Error::RequireHttpsOnly(_))));
    }

    #[test]
    fn a_slow_link_finishes_and_a_stalled_one_gives_up() {
        let config = agent().config().clone();
        let t = config.timeouts();
        // 5,903,360 bytes, the size of the 2026.11.10.2 exe, at 64 kbit/s.
        let slow = Duration::from_secs(5_903_360 * 8 / 64_000);
        assert!(t.global.is_some_and(|g| g > slow));
        assert_eq!(config.timeout_per_read(), Some(STALL));
    }

    /// A rename on one side alone would leave every installed copy without updates.
    #[test]
    fn the_release_workflow_publishes_what_the_check_looks_for() {
        let workflow = include_str!("../../../.github/workflows/release.yml");
        let tag = TAG.strip_suffix('v').unwrap();
        let create = format!(
            "gh release create \"{tag}$TAG\" \"{}\"",
            exe_name("${TAG#v}")
        );
        assert!(workflow.contains(&create), "{create}");
        assert!(workflow.contains("tag=v$("));
    }

    #[test]
    fn compares_numbers_not_text() {
        let releases = || {
            vec![release(
                "langflip-v2026.11.03.1",
                false,
                "Langflip-2026.11.03.1-windows-x64.exe",
            )]
        };
        // `mise run install` writes the day without a leading zero and a 0 counter.
        assert!(newest(releases(), "2026.11.3.0").is_some());
        assert!(newest(releases(), "2026.11.3").is_some());
        assert!(newest(releases(), "2026.9.30.4").is_some());
    }
}
