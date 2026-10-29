# Features

## Fix the layout of a word

Langflip types a word again in the right layout once the person ends it with Space, Enter or Tab. It then switches the layout for the next words.

- **Where** in any app, from the moment Langflip runs.
- **Which words** 4 letters or more, Russian ЙЦУКЕН or English US QWERTY.
- **When it keeps a word** the word as typed is in the dictionary of its layout, or in `keep_words`.
- **Limits** a rare word outside the dictionary misses about 2% of the time in English and 4% in Russian. About 1 word in 1000 that was typed right gets changed. `mise run eval` measures both.

## Undo a fix

Pause on Windows, Ctrl+F12 on macOS, right after a fix. The word comes back as typed, the layout switches back, and the word goes into `keep_words`.

- **Limits** works only before the next key press, click or window switch.

## Convert by hand

Shift+Pause on Windows, Ctrl+Shift+F12 on macOS. Langflip converts the word being typed, or the last word, to the other layout. Short words convert too.

## Skip apps and fields

- **Excluded apps** `excluded_apps` in the settings file.
- **Password fields** skipped on Windows (fields marked protected) and on macOS (secure input).
- **Admin windows** on Windows, skipped unless Langflip itself runs as admin.

## Pause with a click

A left click on the tray icon turns fixing off, and the icon turns grey. The next click turns it on. The tooltip says which state Langflip is in.

## Start at login

The first start turns on start at login, with no question.

- **Windows** a `Langflip` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. Each start rewrites the path, so a moved `langflip.exe` still starts.
- **macOS** `~/Library/LaunchAgents/com.workbenchsoft.langflip.plist`. macOS shows a "Background item added" notice once.
- **Limits** unchecking «Запускать при входе» turns it off for good. Deleting `settings.toml` makes the next start count as the first again.

## Tray menu

A right click on the icon opens it.

- **Исправлять раскладку** fixing on or off, the same as a left click on the icon.
- **Запускать при входе** start at login on or off.
- **Открыть настройки** opens `settings.toml` in the text editor. Langflip reads the change at the next window switch.
- **Выход** quits.

## Settings file

`%APPDATA%\Langflip\settings.toml` on Windows, `~/Library/Application Support/Langflip/settings.toml` on macOS.

| Key | Meaning |
| --- | --- |
| `enabled` | fixing on or off |
| `excluded_apps` | executable names on Windows (`wow.exe`), app names on macOS (`Terminal`) |
| `keep_words` | lowercase words to leave as typed |
