# CI on the local runner

## What it does for the person

Each push to `main` runs the checks, the release and the deploy on the Workbench Soft PC, so the private repo spends no GitHub Actions minutes.

## How it works: modules, files, data

- **Runners** the PC registers 8 runners with the `local` label and 2 of them with `heavy` as well (workbench-soft/local-ci).
- **Workflows** `.github/workflows/` ask for `[self-hosted, local]`, or `[self-hosted, heavy]` for a job that builds with Gradle or Cargo.

## Changes

## Update 2026-10-30: heavy builds on the heavy runners

- **Why** the PC runs 8 runners, and each Gradle, Cargo or Tauri build takes every core. 8 such builds at once crawled: the keeppad release took 50 minutes where a build alone takes 10. The first 2 runners now carry the `heavy` label (workbench-soft/local-ci), so at most 2 such builds share the PC.
- **Workflows** `checks.yml` and `release.yml` ask for `runs-on: [self-hosted, heavy]`. Lint, test and bundle jobs stay on `local`.
- **Docs** `CHANGELOG.md`.
