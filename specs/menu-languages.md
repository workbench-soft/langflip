# Menu languages

## What it does for the person

The tray menu and its tooltip speak the system language when it is English,
Russian, German, French, Spanish, Chinese, Japanese, Korean or Norwegian. Any
other system language gets English.

## How it works

- **Strings** `crates/langflip/locales/app.yml` holds each string in the 9
  languages. `rust-i18n` builds them into the binary.
- **Pick** `crates/langflip/src/language.rs` reads the system language with
  `sys-locale` at start.

## Limits

- **Fixing** Langflip still fixes only Russian and English layouts.
- **Switch** the menu follows the system language. It has no picker of its own.

## Changes

### 2026-10-31 9 languages

- **Why** the owner wants each workbench-soft app in 9 languages. The menu
  spoke only Russian.
- **Steps** this spec, then the strings and the language pick, then the docs.
