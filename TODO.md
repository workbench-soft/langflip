# TODO

## Local install on Windows

- [ ] Check the Microsoft verdict for submission `ea1ee24f-7698-4888-a42b-9879b72e15aa` at <https://www.microsoft.com/en-us/wdsi/submission/ea1ee24f-7698-4888-a42b-9879b72e15aa>. If it says clean, rescan `langflip.exe` with Defender (sent 2026-11-01).

## app-audit

- [ ] Run `mise run install` from the latest `main` after the audit PR of 2026-11-11 merges, then check that `langflip.exe` runs and that the Run value `Langflip` and the task `Langflip` exist. Auto mode denies it, since it stops the running Langflip.
- [ ] Run `mise run smoke` with Punto Switcher closed: the audit of 2026-11-11 makes replayed keys in `windows.rs` wait behind a fix one of them started, and didn't run it.
- [ ] Check by hand on Windows: type `ghbdtn hello` fast in one go, and it comes out as «привет hello» with no letters lost.
- [ ] Check by hand on Windows: type `ytn`, Space, Space, Backspace, `hello`, and `hello` stays English.
- [ ] Merge the Dependabot PRs by hand once they open, each merge ships a release. Turn on Dependabot security updates in the repo settings if wanted.
- [ ] Check the first Release run on `main` after the audit PR of 2026-11-11: the per-job permissions and `persist-credentials: false` ran only through actionlint.
- Left: a rerun of Release after the tag push worked but `gh release create` failed sees the tag and publishes nothing. A pipeline change only a live publish proves.
- Left: Undo or Convert after Enter or Tab erases the word plus the end key, so in a chat Undo retypes and sends again. Limiting Undo to Space is a product call.
- Left: macOS has no stuck-fix timeout, so a failed `post_key` holds every key. No Mac here.
- Left: a caret read from before a fast Space or Backspace can be applied to the next word. A guess, the window is about 40 ms.
- Left: `LoadLibraryW("comctl32.dll")` by bare name in `dialog.rs`. Only the same user can plant a DLL there.
- Left: the `known && gain > 0.0` rule in `decide.rs` has no unit test. `mise run eval` owns it.
- Left: macOS has two Apple binding stacks, `core-graphics` beside `objc2`. A migration, no Mac here.
- Left: macOS has no decision log, so settings and event tap errors go to stderr no one reads. A new UI sign is a product call.
- Left: unnamed timer numbers in `windows.rs`, two thin log wrappers, two XML escapes. Low value, tied to the platform duplication already Left.
- Left: the mouse hook runs on every mouse move. Changing it changes how a click drops the word, needs a device.
- Left: the model loads before the single-copy check, about 9 ms for the second copy at sign-in.
- Left: no `test` or `build` mise task. CI and the hook already run `check` and `lint`.
- Left: `excluded_apps` on macOS matches the localized app name. A guess, no Mac here.
- Left: no Pause key on many laptops, and the Undo key can't be changed. A new setting is a product call.
- Left: `zh-TW` and `zh-Hant` get the Simplified Chinese menu. A new language is a product call.
- Left: tray-icon doesn't call `NIM_SETVERSION`, so keyboard use of the tray runs on the legacy path. Upstream, a guess.
- Left: the topmost update dialog may cover a full-screen game. A guess and a product call.
- [ ] Run `mise run install` from the latest `main` after the audit PR of 2026-11-10 merges. Auto mode denies it, since it stops the running Langflip and replaces its exe.
- [ ] Run `mise run smoke` with Punto Switcher closed: the audit of 2026-11-10 moved the log off the keyboard hook and swapped the key codes in `windows.rs` for the `windows` crate names, and didn't run it.
- [ ] Check by hand on Windows: type a word with the tray menu open, close the menu, and the keyboard works within 1 s.
- [ ] Check by hand on Windows: Shift+Pause in the middle of `ghb` gives «при» with no Space after it.
- [ ] Check on a Mac: Ctrl+Shift+F12 in the middle of a word adds no Space, and the keys still map right. CI only compiles `macos.rs`, and the audit of 2026-11-10 swapped its 43 key codes for `core-graphics` names.
- [ ] Once a release after the audit of 2026-11-10 is out, press Update in the dialog and check that `%LOCALAPPDATA%\Langflip\langflip.log` says `updated to <version>, restarting`. The SHA-256 check and the https only redirect ran in tests, not on a real update.
- [ ] Check the second Checks run on `main` after the audit PR: the rust-cache step should cut `mise run check` from about 3 min. Check the first run of the new Docs workflow.
- Left: Undo before Space after a switch while typed does nothing. It needs a kept-word state in `Engine` and a save at word end.
- Left: `/`, `-` and Shift+digit drop the word with no decision, so «еру.» in the Russian layout stays. Ending the word on them changes what Langflip fixes and needs the smoke test.
- Left: the key handling is written twice, in `windows.rs` and `macos.rs`, and so are `reload_settings`, `show_language`, `show_keep_words` and three tray arms. Moving them into `langflip-core` or `tray.rs` is a refactor of both platforms, and macOS can't run here.
- Left: the keep list has two owners, `Engine.keep_words` and `Settings.keep_words`, synced by hand. It ties into the Undo save inside the hook.
- Left: macOS doesn't retry the event tap after access is granted, so a restart is needed. No Mac here to test it.
- Left: Windows drops a slow keyboard hook with no sign, and Langflip doesn't set it again. Needs a test on a device.
- Left: Undo saves `settings.toml` inside the keyboard hook. A slow disk could make Windows drop the hook. Low risk on a local disk.
- Left: a broken or unreadable `settings.toml` now leaves a line in `langflip.log`, but Langflip shows no sign. A dialog or tray mark is a product call and new UI text.
- Left: a tray toggle or Undo saves the copy in memory without a reload first, so a hand edit with no focus change in between can be lost. A guess, the window is narrow.
- Left: the "update failed" dialog shows when the swap worked and only the restart failed. Changing it changes the message.
- Left: two pushes to `main` inside one build window take the same version, and the second publish fails at the tag push. The fix picks the version at publish, but the exe has it baked in.
- Left: the release workflow keeps 3 releases, so one can vanish under an open Update dialog and the download 404s. A pipeline change.
- Left: `per_page=100` on the shared releases repo stops updates past about 34 apps (55 of 100 slots used today). A `langflip-latest` release, as `reddit-desk-latest` does, changes the release format.
- Left: no rollback when a new exe is broken, since self-replace keeps no old copy. The SHA-256 check lowers the risk.
- Left: no third-party licence notices ship beside the exe. It needs a tool such as cargo-about and a new release asset.
- Left: rustls, ring and ureq are about half the code (1.1 of 2.3 MiB `.text`), and ureq ignores the Windows proxy. WinHTTP through the `windows` crate is a library swap.
- Left: Open settings file may do nothing on a PC with no `.toml` app. A Notepad fallback needs a run of the app to check.
- Left: a 250 ms timer polls the layout forever. A small battery gain, can't measure here.
- Left: `mise run eval` scores a copy of the Engine flow, so continued words and keep words go unmeasured. Engine takes no threshold for the sweep, effort M.
- Left: the `auto-launch` crate could replace the macOS LaunchAgent code, but its plist name differs and old installs would keep a stray agent.
- Left: no PowerShell lint for `scripts/*.ps1`. A new tool needs the `stack` skill.
- Left: `actions/checkout@v7` and `workbench-soft/ci/actions/setup@main` aren't pinned to a commit in the release job that reads `RELEASES_TOKEN`. The org setup action is shared, so pin it across all repos at once.
- Left: an unsigned exe. Signing costs money.

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

## Faster release build

- [ ] Run `mise run install` from the latest `main`: the release profile moved to thin LTO on 2026-11-12. Then check that `langflip.exe` runs and that the Run value `Langflip` and the task `Langflip` exist. Auto mode denies it, since it stops the running Langflip.
