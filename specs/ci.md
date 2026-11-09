# CI on GitHub runners

## What it does for the person

Each push to `main` runs Checks and builds a release beside them. The release publishes once Checks pass, so a fix reaches the updater without a person in the loop.

## How it works: modules, files, data

- **Runners** the repo is public, so its jobs run on GitHub runners at no cost and fork PRs stay off the PC. Checks and the build run on `windows-latest`, publish and cancel on `ubuntu-latest`.
- **Checks** `.github/workflows/checks.yml` runs format, clippy, the tests and the macOS compile on pushes to `main` and on PRs. A push of Markdown or `docs/` only skips it.
- **Release build** `.github/workflows/release.yml` starts on the same push and builds `Langflip-<version>-windows-x64.exe`. The build hands the exe and the tag to publish through the Actions cache. Its last step asks `release-gate.sh checks` for the Checks result once.
- **Publish** runs right after the build when Checks already passed. Otherwise the end of Checks starts Release on `workflow_run`, and `release-gate.sh build` finds the finished build of the commit. One publish runs at a time.
- **Cancel** a failed Checks run starts `release-gate.sh cancel`, which stops the unfinished build of the same commit.

## Limits

- **Docs only** a push of Markdown or `docs/` only builds, then publishes nothing, since no Checks run ends for it.
- **Token** the publish needs the `RELEASES_TOKEN` secret.

## Changes

## Update 2026-10-30: heavy builds on the heavy runners

- **Why** the PC runs 8 runners, and each Gradle, Cargo or Tauri build takes every core. 8 such builds at once crawled: the keeppad release took 50 minutes where a build alone takes 10. The first 2 runners now carry the `heavy` label (workbench-soft/local-ci), so at most 2 such builds share the PC.
- **Workflows** `checks.yml` and `release.yml` ask for `runs-on: [self-hosted, heavy]`. Lint, test and bundle jobs stay on `local`.
- **Docs** `CHANGELOG.md`.

## Update 2026-11-09: week review

- **Why** the parallel release change left stale comments, a bash copy of the guard script with mode 100644, and agent rules pasted into README and docs.
- **Changes** `release.yml` reads like the `repo-ci` template, `wait-for-checks.sh` is the template script with its run bit, and the pasted sections and the keystore line go.
- **Checks** `mise run lint` and `mise run check`.

## Update 2026-11-10: docs follow the release gate

- **Why** the repo went public and moved to GitHub runners, and Release now builds beside Checks through `release-gate.sh`. The top of this spec still described the PC runners and `wait-for-checks.sh`.
- **Changes** the top of this spec and the README describe the current shape. A stray comment in the `on:` block of `release.yml` goes.
