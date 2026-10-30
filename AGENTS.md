# Agent Rules

The keywords MUST, MUST NOT, SHOULD, SHOULD NOT and MAY follow [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

## Tooling

- **mise** each command MUST run through mise: `mise run check`, `mise x -- cargo build -p langflip`.
- **Checks** `mise run check` MUST pass before a commit. It runs format, clippy, tests and the macOS compile.
- **Model** a change to `scripts/build_model.py` or `decide.rs` MUST come with `mise run model` (when the model changes) and `mise run eval`. The commit body gives the false fix and miss rates.
- **Smoke test** a change to `windows.rs` SHOULD pass `mise run smoke`. It types into a test window, so Punto Switcher has to be closed and the person warned first.

## Change Workflow

- **Spec** a feature change MUST add an entry to its file under `specs/`.
- **Commits** follow Conventional Commits: `feat: ...`, `fix: ...`. The subject and first paragraph talk to the person who uses Langflip.
- **Changelog** each PR MUST add a line under `## Unreleased` in `CHANGELOG.md`.
- **Install** a change that touches the app MUST end with the new build installed on this PC. After the PR merges, run `mise run install` from a checkout of the latest `main`, such as the PR worktree before its removal. The task stops the running Langflip, puts the release build in `%LOCALAPPDATA%\Programs\Langflip` and starts it. The final answer gives the version it printed.
- **Privacy** Langflip MUST NOT write typed text to disk or send it anywhere. Debug logs hold decisions and window handles, never the keys of a word that was kept.
