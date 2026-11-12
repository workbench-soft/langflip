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

## Update 2026-11-10: a docs push builds no release

- **Why** Checks skip a push of Markdown or docs, but Release still built and waited for a Checks run that is missing.
- **Change** the push trigger of `release.yml` takes the same paths as Checks. `release-gate.sh` comes from the repo-ci template again.

## Update 2026-11-12: a faster release build

- **Why** each release built every crate from scratch on a fresh GitHub runner. The run of 2026-11-11 (42961878209) took 3 min 36 s: the cargo build 2 min 5 s, the mise setup 37 s with every lint tool, the checkout 7 s, the publish 16 s.
- **Rust cache** the build restores the compiled crates through `Swatinem/rust-cache` under the key `release`, as Checks do. A change that leaves `Cargo.lock` alone builds only the two Langflip crates.
- **Tools** the build installs only Rust through mise, since the lint tools run in Checks. `MISE_EXEC_AUTO_INSTALL=false` keeps `mise x` from installing the other 9 tools, which took 22 s.
- **Profile** the release profile takes `lto = "thin"` with the default codegen units, in place of fat LTO on one unit. Fat LTO ran on one core over all 267 crates on each build. The exe grew from 5.87 MB to 6.41 MB.
- **Timings** cold is the first run after the change, warm the next one with the cache filled. After #62 is warm with the fix of `mise x`. The 10x goal is out of reach on a fresh GitHub runner: setup, checkout, mise and the cache restore take 55 s before cargo starts.

| Step               | Before     | Cold       | Warm       | After #62  |
| ------------------ | ---------- | ---------- | ---------- | ---------- |
| mise setup         | 37 s       | 28 s       | 28 s       | 30 s       |
| Rust cache restore | none       | 5 s        | 13 s       | 11 s       |
| cargo build step   | 2 min 5 s  | 1 min 28 s | 50 s       | 39 s       |
| Build job          | 3 min 4 s  | 2 min 38 s | 1 min 51 s | 1 min 43 s |
| Whole run          | 3 min 36 s | 3 min 7 s  | 2 min 3 s  | 1 min 46 s |
