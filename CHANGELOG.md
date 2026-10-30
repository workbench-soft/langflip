# Changelog

## Unreleased

- **Short words and digits** words of 1 to 3 letters change on Space when only the other layout gives a word: `z` turns into «я», `yt` into «не». Digits stay in the word, so `ьз3` turns into `mp3`.
- **Layout stays** the layout picked with a double click on the tray icon stays when you go to another window. Before, Windows could put the old one back.
- **Fix while typing** Langflip switches a word in the wrong layout from its 4th letter, as Punto Switcher does, and the rest of the word comes out in the right layout. Rare words it can't judge early still get fixed on Space.
- **Double click** a double click on the tray icon switches the layout between Russian and English. A single click now pauses after 0.5 s.
- **Layout on the icon** the tray icon shows `EN` on blue or `RU` on red for the window in front, and turns grey when fixing is off.
- **CI** the Rust build folder moved into the runner work folder, since Defender blocked a test exe under LOCALAPPDATA.
- **File details** `langflip.exe` shows its name, publisher and version in Explorer and Task Manager. Defender still flags the unsigned exe.
- **CI** runs on pushes to main only. PR runs are off, and the workflow says how to turn them back on.
- **CI speed** the checks and the release build into a folder that lasts between runs, so only changed crates compile again.
- **Download** each change on `main` publishes `Langflip-<version>-windows-x64.exe` in the workbench-soft.github.io releases.
- **Pause** a click on the tray icon turns fixing off and on. The right click opens the menu.
- **Autostart** Langflip starts at login from the first start, on Windows and macOS, with no question.
- **Fix the layout** a word of 4 letters or more typed in the wrong layout turns into Russian or English on Space, Enter or Tab, and the layout switches.
- **Undo** Pause (Ctrl+F12 on macOS) puts the last fixed word back and keeps it as typed.
- **Convert by hand** Shift+Pause (Ctrl+Shift+F12 on macOS) converts the last word.
- **Tray menu** turns fixing off, starts Langflip with Windows, opens the settings file.
- **macOS** the first build compiles for Apple Silicon. It has not run on a Mac yet.
