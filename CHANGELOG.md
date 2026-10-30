# Changelog

## Unreleased
- **Languages** the tray menu and tooltip speak the system language: English, Russian, German, French, Spanish, Chinese, Japanese, Korean or Norwegian. Other languages get English.
- **Tray icon** the tray shows the new art deco icon with a small `EN` or `RU` in the corner, in place of the blue and red square. Off still turns it grey.
- **App icon** Langflip has an icon of its own: a gold А and Я in the art deco style. The desktop and Start menu shortcuts and Settings, Apps show it. The tray still shows `EN` or `RU`.
- **Exceptions** a word becomes an exception after you undo its fix twice, not once. The tray menu lists the exceptions under «Исключения», and a click on a word removes it.
- **Install with a double click** `Langflip.cmd` in the repo builds Langflip, installs it and starts it. Langflip then sits on the desktop, in the Start menu and in Settings, Apps, with an Uninstall button.
- **Releases** a release built on the PC carries the Sydney date again. Before 11:00 in Sydney it got the day before.
- **Faster start** the language model loads in about 20 ms again. A sorted word list replaced the set of every word start, which took 0.4 s to build.
- **Settings kept** a `settings.toml` with a mistake in it stays as it is. Before, the next Undo or tray click saved the defaults over it. A crash while saving keeps the old file whole.
- **No crash on typing** Langflip no longer closes on Windows when you type while the tray icon changes.
- **Keyboard stays free** a fix typed while the tray menu was open could hold every key on Windows. A stuck fix now lets the keys go after 1 s.
- **Other apps' text** text a text expander types as characters no longer counts as your keys, and stays as typed when it arrives during a fix.
- **Manual fix** Shift+Pause (Ctrl+Shift+F12 on macOS) on a word being typed no longer adds a Space or Enter after it.
- **Credit** the file details of `langflip.exe` credit the wordfreq data of the model, CC BY-SA 4.0.
- **CI** the build jobs run on the PC's 2 heavy runners, so at most 2 Gradle or Cargo builds share it. 8 at once made a release take 50 minutes.
- **CI** `.github/secret-rotation.json` lists each Actions secret with its provider page, so the `rotate-secrets` skill renews them one at a time.

- **Install from source** `mise run install` builds Langflip and installs it on this PC in `%LOCALAPPDATA%\Programs\Langflip`, in place of the running copy.
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
