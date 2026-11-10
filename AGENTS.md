# Agent Rules

The keywords MUST, MUST NOT, SHOULD, SHOULD NOT and MAY follow [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

## Tooling

- **mise** each command MUST run through mise: `mise run check`, `mise x -- cargo build -p langflip`.
- **Checks** `mise run check` MUST pass before a commit. It runs markdownlint, rustfmt, clippy, ruff, shellcheck, shfmt, oxfmt, the tests and the macOS compile, as the `check` and `lint` tasks in `mise.toml` list.
- **Model** a change to `scripts/build_model.py` or `decide.rs` MUST come with `mise run model` (when the model changes) and `mise run eval`. The commit body MUST give the false fix and miss rates.
- **Smoke test** a change to `windows.rs` SHOULD pass `mise run smoke`. It types into a test window, so Punto Switcher MUST be closed and the person warned first.

## Change Workflow

- **Spec** a feature change MUST add an entry to its file under `specs/`.
- **Commits** MUST follow Conventional Commits: `feat: ...`, `fix: ...`. The subject and first paragraph MUST talk to the person who uses Langflip.
- **Changelog** each PR MUST add a line under `## Unreleased` in `CHANGELOG.md`.
- **Install** a change that touches the app MUST end with the new build installed on this PC, or in auto mode with the command in `TODO.md` as **Auto mode** says. After the PR merges, run `mise run install` from a checkout of the latest `main`, such as the PR worktree before its removal. The task stops the running Langflip, puts the release build in `%LOCALAPPDATA%\Programs\Langflip` and starts it. After an install, the final answer MUST give the version it printed.
- **Autostart** Langflip MUST start after each reboot and sign in. After a change or an install, the agent MUST check that `langflip.exe` runs, the `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value `Langflip` exists, and the Task Scheduler task `Langflip` exists. `mise run install` prints both. Both paths MUST stay, since Explorer once skipped the Run key.
- **Auto mode** `mise run install` stops the running Langflip, and auto mode denies it. The agent MUST put the command in `TODO.md` for the person.
- **Privacy** Langflip MUST NOT write typed text to disk or send it anywhere. Debug logs MUST hold only decisions and window handles, and MUST NOT hold the keys of a word that was kept.
