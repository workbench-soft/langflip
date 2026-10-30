# Layout switcher

## What it does for the person

Langflip fixes a word typed in the wrong keyboard layout. A person types `ghbdtn` with the English layout on, presses Space, and the word turns into «привет». Langflip then switches the layout to Russian for the next words.

- **Languages** Russian (ЙЦУКЕН) and English (US QWERTY).
- **Word length** words of 4 letters or more. Shorter words stay as typed.
- **Systems** Windows 10 and later, macOS 13 and later.
- **Privacy** no network, no telemetry, no typing log. The current word sits in memory until the next word starts.

## User flows

1. **Auto fix** the person types a word in the wrong layout and presses Space, Enter or Tab. Langflip erases the word, types it in the right layout, switches the layout and types the key the person pressed.
2. **Undo** right after a fix the person presses Pause (Ctrl+F12 on macOS). Langflip puts the word back as typed and stops fixing that word.
3. **Manual fix** the person presses Shift+Pause (Ctrl+Shift+F12 on macOS). Langflip converts the last word to the other layout, whatever the model says.
4. **Pause it** a left click on the tray icon turns fixing off and on, and the icon turns grey while off. The right click opens the menu: fixing on and off, start at login, the settings file, quit.
5. **Autostart** the first start turns on start at login, with no question. The menu turns it off.
6. **First run on macOS** Langflip asks for Accessibility and Input Monitoring access and opens System Settings at that page.

## How it works: modules, files, data

The repo is a Cargo workspace.

- **`crates/langflip-core`** the platform-free part: key positions, the two layout tables, the word buffer, the model and the decision. Tests run on any OS.
- **`crates/langflip`** the app: the Windows layer (`src/windows`), the macOS layer (`src/macos`), the tray, the settings file.
- **`model/langflip.model`** the model file. `scripts/build_model.py` builds it from the `wordfreq` word lists. The app embeds it at build time.

### Keys, not letters

The platform layer turns each key event into a key position named after the US key (`KeyQ`, `Semicolon`) plus Shift. The core maps a position to a letter in each layout from static tables. `Semicolon` gives `;` in English and `ж` in Russian, so `hf,jnf` stays one word, «работа».

Space, Enter and Tab end a word. Other keys that are a letter in neither layout (digits, `/`, `-`, arrows, Esc) clear it, and so do a mouse click, a shortcut with Ctrl, Alt or Win, and a switch to another window or field.

### The decision

For each finished word the core builds 2 strings: as typed, and as the same keys give in the other layout.

1. Fewer than 4 letters in the other layout, or the typed word in the person's own word list: keep it.
2. The typed string is in the dictionary of its layout: keep it.
3. Score both strings: the mean log probability per letter from the letter-triple table of each language, with start and end marks.
4. Fix it when the other string is in its dictionary and scores higher, or when it scores higher by the threshold.

A trailing key that is punctuation in the current layout (`hello,`) is scored both as part of the word and apart from it.

### Fixing

1. The key hook decides inside the callback. The model answers in microseconds.
2. A word to fix: the hook swallows the end key and posts the fix to the main thread.
3. The main thread sends Backspaces, the new word as Unicode text, the end key, and a closing marker key. Each injected event carries the Langflip tag, and the hook passes tagged events through.
4. Keys the person presses before the closing marker arrives are held by the hook and replayed after it, so fast typing keeps its order.
5. The main thread asks the window in focus to switch the layout.

### Windows

- **Hooks** `WH_KEYBOARD_LL` and `WH_MOUSE_LL` in the Langflip process, no DLL in other processes.
- **Input** `SendInput` with `KEYEVENTF_UNICODE`. The tag sits in `dwExtraInfo`.
- **Layout** `GetKeyboardLayout` of the focus window's thread reads it. `WM_INPUTLANGCHANGEREQUEST` to the focus window switches it.
- **Skipped windows** password fields (`STATE_SYSTEM_PROTECTED` of the focused object), elevated windows when Langflip runs without admin rights, and apps in the exclusion list.
- **Start with Windows** the first start writes a value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, and the tray toggles it.

### macOS

- **Hook** a `CGEventTap` on the session. The tap comes back on after `kCGEventTapDisabledByTimeout`.
- **Input** `CGEventKeyboardSetUnicodeString`. The tag sits in `kCGEventSourceUserData`.
- **Layout** Text Input Sources (`TISCopyCurrentKeyboardInputSource`, `TISSelectInputSource`).
- **Skipped** secure input (`IsSecureEventInputEnabled`) and apps in the exclusion list.
- **App changes** the frontmost app's process id changes, and the word clears.
- **Start at login** a LaunchAgent plist with `RunAtLoad`, written on the first start. macOS shows its own "Background item added" notice, with no question.

### Settings

`%APPDATA%\Langflip\settings.toml` on Windows, `~/Library/Application Support/Langflip/settings.toml` on macOS.

- **`enabled`** fixing on or off.
- **`excluded_apps`** apps Langflip leaves alone: the executable name on Windows (`wow.exe`), the app name on macOS (`Terminal`).
- **`keep_words`** words to leave as typed. Undo adds to it.

## Limits

- **Layouts** only US QWERTY and Russian ЙЦУКЕН. On the macOS `Russian` layout (not `Russian - PC`) the `ё` key sits elsewhere, so `ё` words may miss.
- **Replayed keys** keys typed during a fix replay before the layout switch lands, so they keep the old layout.
- **Admin windows** Langflip skips them unless it runs as admin.
- **macOS testing** the macOS layer compiles on CI. It has had no run on a Mac yet.

## Launch

- **Distribution** GitHub Releases, then `winget` and a Homebrew cask.
- **Signing** a Windows code signing certificate against SmartScreen and antivirus flags. Apple Developer ID and notarization for macOS. Outside the App Store: its sandbox blocks input to other apps.
- **Updates** a check of GitHub Releases, through the `app-update` skill.
- **Site** a Langflip page on `workbench-soft.github.io` with the downloads. The `seo-setup` skill sets up search.
- **Ads** AdSense on the site only. The app sees each key press, and ads in it would cost trust.
- **Marketing** people who type in Russian and English daily: developers, translators, students. First channels: Habr, Telegram channels about Windows and Mac tools, the `marketing` skill.

## Changes

### 2026-10-30 First version

- **Why** the person wants a Punto Switcher of their own: Russian and English only, words of 4 letters or more, Windows and macOS. The plan came from a reverse engineering pass over Punto Switcher 4.5.0.583.
- **Changes**
  - `crates/langflip-core`: key positions, layout tables, buffer, model loader, decision, tests.
  - `scripts/build_model.py` and `model/langflip.model`: the model from `wordfreq`.
  - `crates/langflip`: Windows hooks, input, tray, settings, autostart. macOS event tap, input, tray.
  - CI: format, lint, tests and a macOS compile check on the self-hosted runner.
- **Steps** spec, workspace and core, model, Windows layer, macOS layer, docs and CI.
- **Docs** README, `docs/architecture.md`, `docs/user-flows.md`, `docs/features.md`, `CHANGELOG.md`.
- **Course changes from the first plan**
  - Static layout tables replace `ToUnicodeEx` and `UCKeyTranslate`. Langflip supports 2 fixed layouts, and the tables give the same letters on both systems.
  - A `tray-icon` menu and a settings file replace a Tauri settings window for the first version.
  - Only Space, Enter and Tab end a word. Retyping a digit or `/` after the layout switch could land in the old layout, so those keys clear the word.
  - The threshold is 1.0. `mise run eval` measured at most 0.08% false fixes and 1.6% (English) to 3.8% (Russian) misses on words outside the dictionary.
  - `mise run smoke` types into a test window on Windows. It needs Punto Switcher closed and nobody typing for 10 seconds.

### 2026-10-30 Click to pause, autostart from the first start

- **Why** the person wants a quick way to turn Langflip off for a while, and Langflip starting with the system right after install, with no question.
- **Changes**
  - `tray.rs`: a left click on the icon turns fixing off and on. The right click opens the menu. The tooltip says whether fixing is on.
  - `settings.rs`: `load` tells the caller when the settings file did not exist yet, which marks the first start.
  - `windows.rs`: the first start writes the Run key. Each later start with autostart on rewrites the path, so a moved `langflip.exe` still starts.
  - `macos.rs`: autostart through a LaunchAgent, `~/Library/LaunchAgents/com.workbenchsoft.langflip.plist` with `RunAtLoad`, written on the first start. The tray menu gets the «Запускать при входе» toggle on macOS too.
- **Steps** spec, tray click, autostart on both systems, docs.
- **Docs** README, `docs/features.md`, `docs/user-flows.md`, `CHANGELOG.md`.

### 2026-10-30 Windows releases

- **Why** the person wants a `langflip.exe` to download, without building it from source.
- **Changes**
  - `.github/workflows/release.yml`: after Checks pass on a push to `main`, the self-hosted runner builds the release `langflip.exe`, tags the commit with a date version (`v2026.10.30.1`) and publishes `Langflip-<version>-windows-x64.exe` as release `langflip-v<version>` in the public `workbench-soft/workbench-soft.github.io` repo. The last 3 releases stay.
  - `.github/scripts/next-version.sh` and `release-notes.sh`: the date version and the notes from `feat`, `fix` and `perf` commits, the same scripts as Tickalot.
- **Limits** the exe is unsigned, so SmartScreen warns on the first run. No macOS build: it needs a Mac runner. The `RELEASES_TOKEN` secret has to be set first, and the run stops before tagging without it.
- **Docs** README, `CHANGELOG.md`.

### 2026-10-30 The current layout on the tray icon

- **Why** the person wants to see the current layout on the tray icon, the way the Windows language bar shows it.
- **Changes**
  - `tray.rs`: the icon shows `EN` on blue or `RU` on red in white letters, drawn from a 5 by 7 pixel font. Off turns the background grey and keeps the letters. A layout other than Russian or English shows the old split square. The icon redraws only when the layout or the on state changes.
  - `windows.rs`: a thread timer every 250 ms reads the layout of the window in front and passes it to the tray.
  - `macos.rs`: the 0.2 s tray timer reads the current input source and passes it to the tray.
- **Limits** a switch shows up within 250 ms, since Windows sends no event to another process when the layout of a window changes.
- **Steps** spec, tray icon, timers on both systems, docs.
- **Docs** README, `docs/features.md`, `docs/user-flows.md`, `CHANGELOG.md`.

### 2026-10-30 Switch the layout with a double click on the tray icon

- **Why** the person wants to switch the layout from the tray icon with a double click.
- **Changes**
  - `tray.rs`: 2 left clicks within 0.5 s give `Action::SwitchLayout`. A single click waits those 0.5 s before it pauses fixing, so a double click leaves fixing as it was. Both systems report each click, so one rule covers them.
  - `windows.rs`: a click on the tray puts the taskbar in front. Langflip keeps the last window in front other than the taskbar, the tray overflow included. The double click brings it back and posts `WM_INPUTLANGCHANGEREQUEST` to it. The icon timer reads that window's layout.
  - `macos.rs`: the double click selects the input source of the other language.
- **Limits** Russian turns English and English turns Russian. Any other layout turns English. The pause comes 0.5 s after the click.
- **Docs** README, `docs/features.md`, `docs/user-flows.md`, `CHANGELOG.md`.

### 2026-10-30 Switch the layout while the word is typed

- **Why** the person wants Langflip to act like Punto Switcher: fix the word while it is typed, once it has 4 letters or more, and leave Space to the person.
- **Changes**
  - `decide.rs`: `decide_prefix` judges an unfinished word. It keeps the word while the typed letters start a word in the dictionary of their layout or a word in `keep_words`. It switches when the same keys in the other layout start a dictionary word there and score higher. A key that is punctuation in the current layout, `hello,`, waits for the next key.
  - `model.rs`: each language keeps the set of dictionary word starts, and scores a word start without the end mark.
  - `lib.rs`: `Engine::key` takes the current layout and returns a fix. The fix erases the typed letters, types them and the new key in the other layout, and switches the layout. One switch per word. Space then ends the word with no second decision, and Undo puts the whole word back.
  - `windows.rs`, `macos.rs`: a letter key can carry a fix, with no end key to retype. On Windows the held keys wait up to 100 ms for the layout switch to land before they replay.
  - `examples/evaluate.rs`: scores the whole path, typing each test word key by key.
- **Limits** a rare word whose first letters start a word of the other language can switch early. Undo puts it back and keeps it from then on.
- **Docs** README, `docs/features.md`, `docs/user-flows.md`, `docs/architecture.md`, `CHANGELOG.md`.
