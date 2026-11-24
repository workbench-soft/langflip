# TODO

## Short words and undo exceptions

- [ ] Install the build with short words left alone and Backspace exceptions (denied: auto mode blocks the install, since it stops the running Langflip). Run in PowerShell:

  ```powershell
  Set-Location "$env:WBS_ROOTlangflip"; git pull; mise run install
  ```

## Smoke test

- [ ] Run the smoke test while the PC is idle (wait). On 2026-11-17 at 14:00 the agent's runs failed with the CPU at 100%, from other sessions' `rustc` and `clang++` builds. Letters came late or out of order after each fix. The old one-case script failed the same way on builds from before #64, so the cause is the load, not that change. The test also types fast, Space Space Backspace, `z` and `mp3`, and Shift+Pause mid word. It covers every `windows.rs` change merged without it.
  1. Close Punto Switcher if it runs, and wait until no other session builds or drives Chrome. The CPU in Task Manager stays under 30%.
  2. Paste the command below into PowerShell.
  3. Keep your hands off the keyboard and mouse for 15 seconds.
  4. If it fails, send the agent the file `%WBS_ROOT%\langflip\target\smoke.txt`.

  ```powershell
  Stop-Process -Name langflip -ErrorAction SilentlyContinue; Set-Location "$env:WBS_ROOT\langflip"; git pull; mise run smoke; Start-Process "$env:LOCALAPPDATA\Programs\Langflip\langflip.exe"
  ```

## Checks by hand on Windows

- [ ] Check the tray menu by hand in Notepad (wait). The agent can't open the tray, since the desktop grant was declined.
  1. Open the tray menu, type a word, close the menu. The keyboard works within 1 second.
  2. Undo the fix of one word twice. The word shows under «Исключения» in the tray menu, and a click on it removes it.

## Update from inside the app

- [ ] Press Update in the dialog once a release newer than `2026.11.16.3` is out (wait). The installed copy is the newest release, so no dialog shows today.
  1. Wait for the dialog «Langflip `<version>` is out». It shows within a minute of the release.
  2. Press Update. The tray icon comes back.
  3. Open `%LOCALAPPDATA%\Langflip\langflip.log`. Its last lines say `updated to <version>, restarting`.

## Mac

- [ ] Check on a Mac (wait), since CI only compiles `macos.rs` and the audit of 2026-11-10 swapped its key codes.
  1. Install the latest release on the Mac.
  2. Press Ctrl+Shift+F12 in the middle of a word. No Space is added.
  3. Type `ghbdtn` and Space. It comes out as «привет».

## Known gaps from the app audits

These need a product call, a Mac or a device. Each one stays here as a note.

- Left: a rerun of Release after the tag push worked but `gh release create` failed sees the tag and publishes nothing. A pipeline change only a live publish proves.
- Left: Undo or Convert after Enter or Tab erases the word plus the end key, so in a chat Undo retypes and sends again. Limiting Undo to Space is a product call.
- Left: macOS has no stuck-fix timeout, so a failed `post_key` holds every key. No Mac here.
- Left: with the CPU at 100%, keys typed 40 ms apart come out late, out of order or lost after a fix, on builds from 2026-11-16 too. #40 only covers a slow PC. Proving a fix needs `mise run smoke` on an idle PC first, to tell a regression from the load.
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
- Left: `mise run eval` scores a copy of the Engine flow, so continued words and keep words go unmeasured. Engine takes no threshold for the sweep, effort M.
- Left: the `auto-launch` crate could replace the macOS LaunchAgent code, but its plist name differs and old installs would keep a stray agent.
- Left: no PowerShell lint for `scripts/*.ps1`. A new tool needs the `stack` skill.
- Left: `actions/checkout@v7` and `workbench-soft/ci/actions/setup@main` aren't pinned to a commit in the release job that reads `RELEASES_TOKEN`. The org setup action is shared, so pin it across all repos at once.
- Left: an unsigned exe. Signing costs money.

## UX audit

- [ ] Look at the new tray UX once, since computer use was denied in the audit run (denied)
  1. Right-click the Langflip icon. With a dark Windows theme the menu is dark.
  2. Point at the icon. The tooltip ends with «Layout: EN» or «Layout: RU».
  3. Rename `%APPDATA%\Langflip\settings.toml` to `settings.old.toml`, quit Langflip from the tray and start it again. The welcome dialog shows. Press OK, quit Langflip, delete the new `settings.toml` and rename `settings.old.toml` back, then start Langflip again.
- [ ] Check the shortcut while Langflip runs, a click test that computer use could not do (denied)
  1. Double-click the Langflip shortcut on the desktop. `settings.toml` opens in the text editor, in front.
  2. Right-click the Langflip icon by the clock and press O. The settings open the same way.
- Left: the welcome and update dialogs stay light in dark mode. The Windows task dialog has no dark theme.
- Left: the tray icon ships one 32 px bitmap, so Windows scales it at 125% (20 px) and 250% (40 px). A size per DPI needs a new icon per scale from `tray-icon`.
- Left: Open the settings shows raw TOML with no comment on its keys. The keys sit in `docs/features.md`. Comments in the file would get lost on each save by `toml`.

## Performance audit

- [ ] Run the smoke test once more for the idle timer change in `windows.rs` (PR #64), as the Smoke test section above says. It takes the keyboard for 10 s, so it waits for you at the PC (wait)
- [ ] Check the tray icon after a layout switch in build `2026.11.16.0` (wait). Press Alt+Shift in Notepad, and the icon turns EN or RU within a second. Wait 10 s with hands off, press Alt+Shift again, and the icon follows again.
- Left: `SendInput` on a thread apart from the keyboard hook. In a test program a 15 key fix took 23.5 ms in place of 33.6 ms, and a 35 key fix 57 ms in place of 172 ms. It reorders the fix and the layout switch, and only `mise run smoke` proves that, which needs the running Langflip stopped.
- Left: the release workflow starts no build before it publishes, as the launch gate of `repo-ci` asks. A Windows tray app on the self-hosted runner shares this PC with the installed Langflip, so a gate needs its own mutex name. A CI change for its own PR.
- Fine (2026-11-16): the keyboard hook takes 2 to 3 µs per key (median, p99 under 16 µs). Memory and handles: 9.7 MB private and 340 handles after 16 h, 7.8 MB and 274 handles at start. The installed exe is 6.5 MB.
