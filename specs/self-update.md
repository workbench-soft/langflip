# Self update

## What it does for the person

Langflip on Windows checks GitHub for a newer release at start and then about once a day. When one is out, a dialog names the version and its size, with Update and Later. Update downloads the new `langflip.exe`, puts it in place of the running one and restarts Langflip. The settings stay.

## User flows

1. Langflip starts, waits 30 seconds, then asks GitHub for the releases.
2. GitHub lists a `langflip-v*` release newer than the running version, with an `.exe`.
3. A dialog shows «Langflip 2026.11.05.1 is out» and the size in MB, with Update and Later.
4. **Update** Langflip downloads the exe, checks its size, swaps it for the running one and starts the new copy. The old copy quits and its tray icon goes.
5. **Later** Langflip stays quiet about that version until its next start. A newer release still shows a dialog at the next daily check.

Offline, on a GitHub error or with no newer release, Langflip shows nothing and asks again in an hour, or a day after the last answer.

## How it works: modules, files, data

- **Check** `crates/langflip/src/update.rs` runs on a thread of its own. It asks `https://api.github.com/repos/workbench-soft/workbench-soft.github.io/releases?per_page=100` with `ureq`, then picks the highest non-draft `langflip-v*` tag that has an `.exe` asset.
- **Version** `build.rs` passes the version into the code as `LANGFLIP_VERSION`: the date version the Release workflow sets, such as `2026.11.02.1`, or the Cargo one in a local build. A version whose first number is under 2000 is a dev build and skips the check, so `cargo run` and `mise run smoke` show no dialog.
- **Compare** both versions split on dots into numbers, and the longer list wins on a tie, so `2026.11.3.0` from `mise run install` sits below `2026.11.03.1`.
- **Dialog** a Windows task dialog on the update thread, so the keyboard hook and the fix messages on the main thread keep running. The exe manifest asks for Common Controls 6, where the task dialog lives.
- **Swap** the exe downloads to `%TEMP%\langflip-<version>.exe`. A size other than the one GitHub lists fails the update. `self-replace` puts the file in place of the running exe, which Windows keeps locked.
- **Restart** the update thread starts the new exe with `--updated`, then tells the main thread to quit. The new copy waits up to 10 seconds for the old copy's single instance mutex to go.
- **Failure** a failed download or swap shows «Langflip could not update» and offers the same version again at the next check. The log gets the reason.
- **Log** `%LOCALAPPDATA%\Langflip\langflip.log` gets each check result and each error. No typed text goes there.

## Limits

- **macOS** has no release, so the check runs on Windows only.
- **Signature** the exe is unsigned. HTTPS and the size check guard the download.
- **Progress** the dialog closes during the download, a few MB that take seconds. The new copy appears in the tray when it is done.
- **Rate limit** GitHub allows 60 calls an hour without a token. Langflip makes one call a day.

## Changes

### 2026-11-03 Langflip updates itself from GitHub

- **Why** the owner wants each Workbench Soft app installed from GitHub to update itself. An audit on 2026-11-03 found Langflip without an update check.
- **Changes** `update.rs` for the check, the dialog, the download and the swap. `build.rs` for the version and the manifest. `windows.rs` starts the update thread and waits for the mutex after an update. `locales/app.yml` gets the dialog strings in the 9 languages. `Cargo.toml` adds `ureq`, `serde_json` and `self-replace`.
- **Steps** this spec, then the code, the strings and the docs.
- **Docs** `docs/features.md`, `docs/architecture.md`, `README.md`, `CHANGELOG.md`.
