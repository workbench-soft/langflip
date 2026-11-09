# Changelog

## Unreleased

- **Docs** the README and the CI spec describe Release building beside Checks on GitHub runners.

- **CI** Release builds beside Checks again and publishes once they pass, through `release-gate.sh`. A failed Checks run cancels the build. No job waits on a runner for the other workflow.

- **CI** Release starts on `workflow_run` once Checks pass on `main`, one at a time. It no longer holds a PC runner while it waits, which starved Checks and timed releases out.

- **Parallel release builds** releases build beside checks after a push. Only publication waits for matching checks; open PRs hold no build.
- **CI** the Release workflow reads like the `repo-ci` template again, and the docs lose the agent rules and the keystore line pasted in by the CI rollout.

- **Updates** Langflip on Windows checks GitHub for a newer release at start and about once a day. A dialog offers it with Update and Later, and Update swaps the exe and restarts Langflip.
- **Agent rules** `AGENTS.md` holds the lessons of the past week: what fails for an agent here, and what it does instead.
- **Docs** the architecture page lists each CI secret, where a new value comes from and what breaks after a change.
- **CI** the release gate runs on a GitHub runner, since the repo is public.

- **CI** Release waits while a PR of an agent session is open, so the last merge of a batch releases them all.
- **Screenshots** the README shows a word fixed into Russian and one into English. `mise run demo` takes them again.
- **Docs** each heading of `docs/features.md` links its spec, and each feature that had no spec got one.
- **Markdown** markdownlint-cli2 lints the docs in `mise run lint`.

- **Agent docs** `AGENTS.md` states each rule with MUST, SHOULD or MAY.
- **Busy PC** letters typed right after a fix keep their order when the PC is slow, and the next word no longer flips back to the old layout.
- **Go on with a word** letters typed onto a word already on screen take that word's language. Langflip no longer switches them as if they started a new word. Backspace after Space brings the last word back, and on Windows a click or an arrow next to a word does too.
- **No English letters left in a word** a key Windows hands to Langflip while the tray icon changes no longer ends up in front of the fixed word. A word that missed a key stays as typed, and a key pressed during a fix waits for it.
- **Log** Langflip on Windows writes its decisions to `%LOCALAPPDATA%\Langflip\langflip.log`: key counts, the app in front, why a word was dropped. Your words stay out of it.
- **Language menu** the tray menu has a Language submenu. Pick System language or one of 9 languages, each under its own name, and the menu switches at once. Langflip remembers the choice.
- **Open source** the repo is public. Checks and Release run on GitHub Windows runners, and Checks runs on each PR too.
- **Backspace** the Backspaces a fix sends carry the scan code a real key sends, so apps that drop them no longer keep `ghbd` in front of «привет».
- **Releases** the Release workflow has its `RELEASES_TOKEN` secret and publishes again.
- **Start at sign in** on Windows a Task Scheduler task starts Langflip too, then each minute for 15 minutes. Langflip now starts when Explorer skips its Run key value after a reboot.
- **Docs** `docs/architecture.md` has a Decisions section: each choice, why it won, what it costs, the rivals and why they lost.
- **Lint and format** `mise run lint` checks the Rust with rustfmt and clippy, the Python with ruff, the scripts with shellcheck and shfmt, and the docs with oxfmt. `mise run check` runs it first.

- **Languages** the tray menu and tooltip speak the system language: English, Russian, German, French, Spanish, Chinese, Japanese, Korean or Norwegian. Other languages get English.
- **Tray icon** the tray shows a navy `EN` or `RU` on a holo foil tile, in place of the blue and red square. Off still turns it grey.
- **App icon** Langflip has an icon of its own: a navy А and Я on an iridescent holo foil card. The desktop and Start menu shortcuts and Settings, Apps show it.
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
