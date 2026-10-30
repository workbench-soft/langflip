# TODO

## Local install on Windows

- [ ] Turn on «Запускать при входе» in the Langflip tray menu. `settings.toml` existed before the install, so the start didn't turn it on (2026-10-30).
- [ ] Send `langflip.exe` to Microsoft as a false positive at https://www.microsoft.com/wdsi/filesubmission, so the release exe isn't blocked for other people.

## app-audit

- [ ] Run `mise run install` from the latest `main` (AGENTS.md asks for it after an app change). The audit agent's permission check blocked it, since it stops the running Langflip and replaces its exe (PR #17, 2026-10-30).
- [ ] Run `mise run smoke` with Punto Switcher closed: the audit changed `windows.rs` (state borrow, stuck fix, Unicode input, Shift+Pause) and didn't run it (2026-10-30).
- [ ] Check by hand on Windows: type a word with the tray menu open, close the menu, and the keyboard works within 1 s.
- [ ] Check by hand on Windows: Shift+Pause in the middle of `ghb` gives «при» with no Space after it.
- [ ] Check on a Mac: Ctrl+Shift+F12 in the middle of a word adds no Space. CI only compiles `macos.rs`.
- Left: Undo before Space after a switch while typed does nothing. It needs a kept-word state in `Engine` and a save at word end, in the code `fix/fast-model-load` is debugging.
- Left: `/`, `-` and Shift+digit drop the word with no decision, so «еру.» in the Russian layout stays. Ending the word on them changes what Langflip fixes and needs the smoke test.
- Left: the key handling is written twice, in `windows.rs` and `macos.rs`. Moving it into `langflip-core` is a refactor of both platforms, and macOS can't run here.
- Left: macOS doesn't retry the event tap after access is granted, so a restart is needed. No Mac here to test it.
- Left: Windows drops a slow keyboard hook with no sign, and Langflip doesn't set it again. Needs a test on a device.
- Left: Undo saves `settings.toml` inside the keyboard hook. A slow disk could make Windows drop the hook. Low risk on a local disk.
- Left: `actions/checkout@v7` and `workbench-soft/ci/actions/setup@main` aren't pinned to a commit in the release job that reads `RELEASES_TOKEN`. The org setup action is shared, so pin it across all repos at once.
- Left: no in-app update and an unsigned exe. Signing costs money, and the update flow is a product call (see the `app-update` skill).
