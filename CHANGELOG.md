# Changelog

## Unreleased

- **File details** `langflip.exe` shows its name, publisher and version in Explorer and Task Manager. Defender still flags the unsigned exe.
- **CI speed** the checks and the release build into a folder that lasts between runs, so only changed crates compile again.
- **Download** each change on `main` publishes `Langflip-<version>-windows-x64.exe` in the workbench-soft.github.io releases.
- **Pause** a click on the tray icon turns fixing off and on. The right click opens the menu.
- **Autostart** Langflip starts at login from the first start, on Windows and macOS, with no question.
- **Fix the layout** a word of 4 letters or more typed in the wrong layout turns into Russian or English on Space, Enter or Tab, and the layout switches.
- **Undo** Pause (Ctrl+F12 on macOS) puts the last fixed word back and keeps it as typed.
- **Convert by hand** Shift+Pause (Ctrl+Shift+F12 on macOS) converts the last word.
- **Tray menu** turns fixing off, starts Langflip with Windows, opens the settings file.
- **macOS** the first build compiles for Apple Silicon. It has not run on a Mac yet.
