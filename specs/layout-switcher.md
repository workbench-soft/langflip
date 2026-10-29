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
4. **Pause it** the tray menu turns fixing off and on, opens the settings file and quits.
5. **First run on macOS** Langflip asks for Accessibility and Input Monitoring access and opens System Settings at that page.

## How it works: modules, files, data

The repo is a Cargo workspace.

- **`crates/langflip-core`** the platform-free part: key positions, the two layout tables, the word buffer, the model and the decision. Tests run on any OS.
- **`crates/langflip`** the app: the Windows layer (`src/windows`), the macOS layer (`src/macos`), the tray, the settings file.
- **`model/langflip.model`** the model file. `scripts/build_model.py` builds it from the `wordfreq` word lists. The app embeds it at build time.

### Keys, not letters

The platform layer turns each key event into a key position named after the US key (`KeyQ`, `Semicolon`) plus Shift. The core maps a position to a letter in each layout from static tables. `Semicolon` gives `;` in English and `ж` in Russian, so `hf,jnf` stays one word, «работа».

A word ends on a key that is a letter in neither layout: Space, Enter, Tab, a digit, `/`, `-`, `=`, `\`. Arrows, Home, End, Esc, a mouse click, a shortcut with Ctrl, Alt or Win, and a switch to another window clear the word.

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
- **Start with Windows** the tray toggles a value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

### macOS

- **Hook** a `CGEventTap` on the session. The tap comes back on after `kCGEventTapDisabledByTimeout`.
- **Input** `CGEventKeyboardSetUnicodeString`. The tag sits in `kCGEventSourceUserData`.
- **Layout** Text Input Sources (`TISCopyCurrentKeyboardInputSource`, `TISSelectInputSource`).
- **Skipped** secure input (`IsSecureEventInputEnabled`) and apps in the exclusion list.
- **App changes** the frontmost app's process id changes, and the word clears.

### Settings

`%APPDATA%\Langflip\settings.toml` on Windows, `~/Library/Application Support/Langflip/settings.toml` on macOS.

- **`enabled`** fixing on or off.
- **`excluded_apps`** executable names Langflip leaves alone, such as `wow.exe`.
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
