# Langflip

Langflip fixes a word typed in the wrong keyboard layout. Type `ghbdtn` with the English layout on, press Space, and it becomes «привет». The layout then switches to Russian.

- **Languages** Russian (ЙЦУКЕН) and English (US QWERTY).
- **Words** 4 letters or more. Shorter words stay as typed.
- **Systems** Windows 10 and later, macOS 13 and later.
- **Privacy** no network and no typing log. The current word lives in memory until the next one starts.

## Use it

- **Fix** happens on Space, Enter or Tab.
- **Undo** Pause on Windows, Ctrl+F12 on macOS. The word comes back as typed and stays that way from then on.
- **Convert by hand** Shift+Pause on Windows, Ctrl+Shift+F12 on macOS.
- **Layout** the tray icon shows `EN` on blue or `RU` on red.
- **Pause** click the tray icon to turn fixing off. The icon turns grey. Click again to turn it on.
- **Tray menu** a right click opens it: fixing on and off, start at login, the settings file, quit.
- **Autostart** the first start turns on start at login, with no question. Uncheck «Запускать при входе» to stop it.

The settings file is `%APPDATA%\Langflip\settings.toml` on Windows and `~/Library/Application Support/Langflip/settings.toml` on macOS. Its keys are in [docs/features.md](docs/features.md).

## Run it from source

Install [mise](https://mise.jdx.dev), then:

```bash
mise install
mise x -- cargo run -p langflip
```

On macOS, allow Langflip (or the terminal running it) in System Settings, Privacy & Security, under Accessibility and Input Monitoring.

## Test it

| Command | What it checks |
| --- | --- |
| `mise run check` | format, clippy, unit tests, macOS compile, the same steps CI runs |
| `mise run eval` | false fixes and misses of the model on `model/testset.tsv` |
| `mise run smoke` | Windows only: runs Langflip and types into a test window. Close Punto Switcher first, and don't type for 10 seconds. |

## Build and release

```bash
mise x -- cargo build --release -p langflip
```

The binary lands in `target/release/`. Set `LANGFLIP_VERSION` to give it a version other than the one in `Cargo.toml`.

Each push to `main` that passes Checks makes a release: `.github/workflows/release.yml` builds `langflip.exe` on the self-hosted runner and publishes `Langflip-<version>-windows-x64.exe` in the public [workbench-soft.github.io releases](https://github.com/workbench-soft/workbench-soft.github.io/releases), with a date version such as `v2026.10.30.1`. It needs the `RELEASES_TOKEN` secret. The exe is unsigned yet, and there is no macOS build.

## Rebuild the model

`mise run model` downloads the `wordfreq` word lists and writes `model/langflip.model` and `model/testset.tsv`. Run `mise run eval` after it.

## Docs

- [docs/features.md](docs/features.md) each feature and its limits.
- [docs/user-flows.md](docs/user-flows.md) the flows step by step.
- [docs/architecture.md](docs/architecture.md) modules, data and the path of a key press.
- [specs/layout-switcher.md](specs/layout-switcher.md) the plan and each change.

## License

The code is MIT. The model in `model/` comes from [wordfreq](https://github.com/rspeer/wordfreq) data and is under CC BY-SA 4.0, as that data is.
