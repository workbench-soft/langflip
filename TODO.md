# TODO

## Smoke test

- [ ] Run the smoke test once. 5 merged changes to `windows.rs` went in without it: the replayed keys, the log off the hook, the digit keys, the faster model load and the tray refresh after Undo.
  1. Close Punto Switcher if it runs.
  2. Paste the command below into PowerShell.
  3. Keep your hands off the keyboard for 10 seconds.
  4. If it fails, send its last lines to the agent.

  ```powershell
  Stop-Process -Name langflip -ErrorAction SilentlyContinue; Set-Location "$env:WBS_ROOTlangflip"; git pull; mise run smoke; Start-Process "$env:LOCALAPPDATAProgramsLangfliplangflip.exe"
  ```

## Checks by hand on Windows

- [ ] Type each line below in Notepad with Langflip on, and tell the agent which one goes wrong.
  1. Type `ghbdtn hello` fast in one go. It comes out as «привет hello» with no letters lost.
  2. Type `ytn`, Space, Space, Backspace, `hello`. `hello` stays English.
  3. Type `z` and Space. It gives «я». Type `ьз3` and Space. It gives `mp3`.
  4. Type `ghb` and press Shift+Pause. It gives «при» with no Space after it.
  5. Open the tray menu, type a word, close the menu. The keyboard works within 1 second.
  6. Undo the fix of one word twice. The word shows under «Исключения» in the tray menu, and a click on it removes it.
- [ ] Find the app that turned `ghbdtn` into `ghbdпривет` on 2026-11-01.
  1. Type `ghbdtn` and Space in each app you used that day.
  2. Tell the agent the name of the app where the word comes out wrong.

## Update from inside the app

- [ ] Check that Update in the dialog works. Release `2026.11.13.1` from the Dependabot merges is newer than the installed `2026.11.12.2`.
  1. Quit Langflip from the tray and start it again.
  2. Wait one minute. The dialog «Langflip `<version>` is out» shows.
  3. Press Update. The tray icon comes back.
  4. Open `%LOCALAPPDATA%Langfliplangflip.log`. Its last lines say `updated to <version>, restarting`.

## Mac

- [ ] Check on a Mac, since CI only compiles `macos.rs` and the audit of 2026-11-10 swapped its key codes.
  1. Install the latest release on the Mac.
  2. Press Ctrl+Shift+F12 in the middle of a word. No Space is added.
  3. Type `ghbdtn` and Space. It comes out as «привет».

## Dependabot

- [ ] Decide on Dependabot security updates. Security alerts are on, the automatic fix PRs are off.
  1. Open <https://github.com/workbench-soft/langflip/settings/security_analysis>.
  2. Press Enable next to «Dependabot security updates» if you want the fix PRs.

## Known gaps from the app audits

These need a product call, a Mac or a device. Each one stays here as a note.

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
