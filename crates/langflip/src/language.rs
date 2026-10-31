//! Picks the menu language: the one the person chose, or the system language.

/// The languages `locales/app.yml` holds, each with its name in itself.
pub const LANGUAGES: [(&str, &str); 9] = [
    ("en", "English"),
    ("ru", "Русский"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("es", "Español"),
    ("zh", "中文"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("nb", "Norsk bokmål"),
];

/// The code in [`LANGUAGES`] that matches `code`, if the app has it.
fn known(code: &str) -> Option<&'static str> {
    LANGUAGES
        .into_iter()
        .map(|(lang, _)| lang)
        .find(|lang| *lang == code)
}

/// The language the person chose, or `None` when the menu follows the system.
pub fn chosen(choice: Option<&str>) -> Option<&'static str> {
    choice.and_then(known)
}

/// The menu language for a system locale such as `ru-RU` or `nb_NO`. English when the app lacks it.
pub fn menu_language(system: Option<&str>) -> &'static str {
    let code = system
        .and_then(|tag| tag.split(['-', '_']).next())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    // Norwegian comes as "no" on some systems and "nb" on others.
    let code = if code == "no" || code == "nn" {
        "nb"
    } else {
        &code
    };
    known(code).unwrap_or("en")
}

/// The language to show: the person's choice from the settings, or the system language.
fn pick(choice: Option<&str>, system: Option<&str>) -> &'static str {
    chosen(choice).unwrap_or_else(|| menu_language(system))
}

/// Sets the menu language for each `t!` call that follows. `None` follows the system.
pub fn apply(choice: Option<&str>) {
    rust_i18n::set_locale(pick(choice, sys_locale::get_locale().as_deref()));
}

#[cfg(test)]
mod tests {
    use super::{menu_language, pick};

    #[test]
    fn takes_the_language_part_of_the_tag() {
        assert_eq!(menu_language(Some("ru-RU")), "ru");
        assert_eq!(menu_language(Some("zh-Hans-CN")), "zh");
        assert_eq!(menu_language(Some("nb_NO")), "nb");
    }

    #[test]
    fn maps_norwegian_to_bokmal() {
        assert_eq!(menu_language(Some("no")), "nb");
        assert_eq!(menu_language(Some("nn-NO")), "nb");
    }

    #[test]
    fn falls_back_to_english() {
        assert_eq!(menu_language(Some("pt-BR")), "en");
        assert_eq!(menu_language(None), "en");
    }

    #[test]
    fn the_choice_beats_the_system() {
        assert_eq!(pick(Some("de"), Some("ru-RU")), "de");
        assert_eq!(pick(None, Some("ru-RU")), "ru");
    }

    #[test]
    fn an_unknown_choice_follows_the_system() {
        assert_eq!(pick(Some("pt"), Some("ja-JP")), "ja");
    }

    #[test]
    fn each_string_has_every_language() {
        let all: Vec<&str> = super::LANGUAGES.iter().map(|(lang, _)| *lang).collect();
        let file = include_str!("../locales/app.yml");
        let mut strings: Vec<(&str, Vec<&str>)> = Vec::new();
        for line in file.lines().filter(|l| !l.starts_with('_')) {
            match line.strip_prefix("  ") {
                Some(entry) => strings.last_mut().unwrap().1.push(&entry[..2]),
                None => strings.push((line.trim_end_matches(':'), Vec::new())),
            }
        }
        for (key, langs) in strings {
            assert_eq!(langs, all, "{key}");
        }
    }
}
