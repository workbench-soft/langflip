# TODO

## Local install on Windows

- [ ] Allow `%LOCALAPPDATA%\Programs\Langflip\langflip.exe` in Windows Security. Defender flags it as `Trojan:MSIL/ClipBanker.GE!MTB` and blocks the start (2026-10-30).
- [ ] Start `langflip.exe` once after that. The first start turns on start at login.
- [ ] Send `langflip.exe` to Microsoft as a false positive at https://www.microsoft.com/wdsi/filesubmission, so the release exe isn't blocked for other people.

## build/exe-metadata

- [ ] Merge https://github.com/workbench-soft/langflip/pull/5 once CI is green, then remove worktree `langflip-build-exe-metadata` and its branch.
