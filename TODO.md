# TODO

## Local install on Windows

- [ ] Check the Microsoft verdict for submission `ea1ee24f-7698-4888-a42b-9879b72e15aa` at <https://www.microsoft.com/en-us/wdsi/submission/ea1ee24f-7698-4888-a42b-9879b72e15aa>. If it says clean, rescan `langflip.exe` with Defender (sent 2026-11-01).

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

## fix/fast-model-load

- [ ] Type `ghbdtn` in English in the app that gave `ghbdпривет` and tell which app it is. Backspace now carries its scan code (`fix/backspace-scan-code`), the likely cause, but the bug wasn't reproduced (2026-11-01).
- [ ] Run `mise run smoke` with nobody typing and the installed Langflip quit: the faster model load and the Backspace scan code merged without it (2026-11-01).

## feat/short-words (PR #13)

- [ ] Run `mise run smoke` with Punto Switcher closed: `windows.rs` now maps the digit keys, and the smoke test wasn't run (2026-10-30).
- [ ] Install the next release and check by hand: `z` + Space gives «я», `ьз3` + Space gives `mp3`.

## chore/install-task (PR #14)

## feat/keep-after-two-undos (PR #23)

- [ ] Check by hand on Windows: undo the fix of one word twice, and it shows under «Исключения» in the tray menu. A click on it removes it (2026-10-31).
- [ ] Run `mise run smoke` with Punto Switcher closed: `windows.rs` now refreshes the tray menu after Undo, and the smoke test wasn't run (2026-10-31).

## feat/self-update (PR #50)

- [ ] Run `mise run install` from the latest `main`, so the installed Langflip has the update check. Auto mode denies it, since it stops the running Langflip (2026-11-03).
- [ ] Once a release newer than the installed one is out, wait a minute after a Langflip start: the dialog «Langflip `<version>` is out» shows. Press Update and check that the tray icon comes back and `%LOCALAPPDATA%\Langflip\langflip.log` says `updated to <version>, restarting`. The swap and the restart were not run on this PC, since the installed copy holds the single instance mutex (2026-11-03).
