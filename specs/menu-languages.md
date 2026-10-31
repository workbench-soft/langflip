# Menu languages

## What it does for the person

The tray menu and its tooltip speak the system language when it is English,
Russian, German, French, Spanish, Chinese, Japanese, Korean or Norwegian. Any
other system language gets English. The Language submenu picks another one.

## How it works

- **Strings** `crates/langflip/locales/app.yml` holds each string in the 9
  languages. `rust-i18n` builds them into the binary.
- **Pick** `crates/langflip/src/language.rs` reads the system language with
  `sys-locale`, unless `settings.toml` names a language.
- **Choice** the Language submenu of the tray menu lists «System language» and
  the 9 languages by their own names. A click saves the choice as `language` in
  `settings.toml` and relabels the menu at once.

## Limits

- **Fixing** Langflip still fixes only Russian and English layouts.
- **System change** a new system language shows after the next start.

## Changes

### 2026-10-31 9 languages

- **Why** the owner wants each workbench-soft app in 9 languages. The menu
  spoke only Russian.
- **Steps** this spec, then the strings and the language pick, then the docs.

### 2026-11-01 Language submenu

- **Why** the org rules say each app follows the system language and lets the
  person pick another one in its settings. Langflip had no picker.
- **What** a Language submenu in the tray menu: «System language» first, then
  English, Русский, Deutsch, Français, Español, 中文, 日本語, 한국어 and Norsk
  bokmål. The current choice has a check mark.
- **Saved** `language = "de"` in `settings.toml`. No key means the system
  language. An unknown code also gets the system language.
- **At once** the click sets the `rust-i18n` locale and relabels each menu item
  and the tooltip, with no restart. An edit of the file by hand does the same.
- **Steps** this spec, then the setting, the submenu and its strings, then the
  docs and the changelog.
