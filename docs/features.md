# Features

## Fix the layout of a word

Langflip types a word again in the right layout while the person types it, and switches the layout for the rest of the word. When the first letters leave doubt, it waits for Space, Enter or Tab.

- **Where** in any app, from the moment Langflip runs.
- **Which words** any length, digits included, Russian ЙЦУКЕН or English US QWERTY.
- **While typed** from the 4th letter, once the typed letters start no dictionary word of their layout and the same keys in the other layout start one there and score higher.
- **Short words** under 4 letters change on Space, Enter or Tab, only when the typed letters are no word of their layout and the other layout gives a word from its dictionary: `z` turns into «я», `yt` into «не», `шт` into `in`.
- **Digits** stay in the word: `ьз3` turns into `mp3`. A digit with Shift clears the word.
- **When it keeps a word** the word as typed is in the dictionary of its layout, or in `keep_words`.
- **Limits** a rare word outside the dictionary misses about 1.5% of the time in each language. About 1 rare word in 1000 typed right gets changed. About 2 rare short words in 100 typed right get changed, such as «ем» into `tv`, and one letters `c`, `d`, `j` typed alone in English turn Russian. `mise run eval` measures both.

## Undo a fix

Pause on Windows, Ctrl+F12 on macOS, right after a fix. The word comes back as typed and the layout switches back. The second Undo of the same word puts it into `keep_words`, and Langflip leaves it as typed from then on.

- **Limits** works only before the next key press, click or window switch.
- **Count** Langflip counts the first Undo in memory only. A restart sets it back to 0.

## Convert by hand

Shift+Pause on Windows, Ctrl+Shift+F12 on macOS. Langflip converts the word being typed, or the last word, to the other layout. Short words convert too.

## Skip apps and fields

- **Excluded apps** `excluded_apps` in the settings file.
- **Password fields** skipped on Windows (fields marked protected) and on macOS (secure input).
- **Admin windows** on Windows, skipped unless Langflip itself runs as admin.

## The current layout on the icon

The tray icon shows `EN` or `RU` in black marker on a yellow tile, for the layout of the window in front. A click on the taskbar keeps the last app window. A layout other than these 2 shows the app icon alone.

- **Limits** a switch reaches the icon within 250 ms on Windows and 0.2 s on macOS.

## The app icon

A Z and a Я in black marker over a yellow highlighter stripe on cream. Z and Я share a key on the keyboard. Windows shows it on `langflip.exe`, the desktop and Start menu shortcuts and in Settings, Apps.

- **Source** `crates/langflip/assets/icon.svg`. Both rounds of drafts sit in `design/icons/`.
- **Tray** the tray shows the layout in the same marker on yellow.
- **Limits** macOS has no app bundle yet, so it shows no app icon.

## Pause with a click

A left click on the tray icon turns fixing off, and the whole icon turns grey with the layout letters kept. The next click turns it on. The tooltip says which state Langflip is in.

- **Limits** the pause waits 0.5 s after the click, in case a second click makes it a double click.

## Switch the layout with a double click

A double click on the tray icon switches the layout between Russian and English. Any other layout turns English.

- **Windows** the click puts the taskbar in front, so Langflip brings back the window that was in front and switches its layout. The icon shows that window's layout, not the taskbar's. The new layout stays in the other windows too, as with the Windows hotkey.
- **macOS** switches the current input source.

## Install on Windows

A double click on `Langflip.cmd` in the repo builds Langflip and installs it for the current user.

- **Shortcuts** `Langflip.lnk` on the desktop and in the Start menu.
- **Installed apps** Langflip shows in Settings, Apps with its version and an Uninstall button.
- **Uninstall** stops Langflip and removes the program folder, the shortcuts and the autostart. `settings.toml` stays.
- **Limits** needs mise and the Rust toolchain. The release download has no installer.

## Start at login

The first start turns on start at login, with no question.

- **Windows** a `Langflip` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. Each start rewrites the path, so a moved `langflip.exe` still starts.
- **macOS** `~/Library/LaunchAgents/com.workbenchsoft.langflip.plist`. macOS shows a "Background item added" notice once.
- **Limits** unchecking Start at sign in turns it off for good. Deleting `settings.toml` makes the next start count as the first again.

## Tray menu

A right click on the icon opens it.

- **Fix the layout** fixing on or off, the same as a left click on the icon.
- **Start at sign in** start at login on or off.
- **Exceptions (N)** the words in `keep_words`. A click on a word removes it, and Langflip fixes it again.
- **Open the settings** opens `settings.toml` in the text editor. Langflip reads the change at the next window switch.
- **Quit** quits.
- **Language** the menu and the tooltip speak the system language when it is English, Russian, German, French, Spanish, Chinese, Japanese, Korean or Norwegian, and English otherwise.

## Settings file

`%APPDATA%\Langflip\settings.toml` on Windows, `~/Library/Application Support/Langflip/settings.toml` on macOS.

| Key | Meaning |
| --- | --- |
| `enabled` | fixing on or off |
| `excluded_apps` | executable names on Windows (`wow.exe`), app names on macOS (`Terminal`) |
| `keep_words` | lowercase words to leave as typed |

- **Limits** a file with a mistake in it stays as it is. Langflip runs on the defaults and saves no change until the file reads again.
