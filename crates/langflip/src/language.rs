//! Picks the menu language from the system language at start.

/// The languages `locales/app.yml` holds.
const LANGUAGES: [&str; 9] = ["en", "ru", "de", "fr", "es", "zh", "ja", "ko", "nb"];

/// The menu language for a system locale such as `ru-RU` or `nb_NO`. English when the app lacks it.
pub fn menu_language(system: Option<&str>) -> &'static str {
    let code = system
        .and_then(|tag| tag.split(['-', '_']).next())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    // Norwegian comes as "no" on some systems and "nb" on others.
    let code = if code == "no" || code == "nn" {
        "nb".to_owned()
    } else {
        code
    };
    LANGUAGES
        .into_iter()
        .find(|lang| *lang == code)
        .unwrap_or("en")
}

/// Sets the menu language for each `t!` call that follows.
pub fn init() {
    rust_i18n::set_locale(menu_language(sys_locale::get_locale().as_deref()));
}

#[cfg(test)]
mod tests {
    use super::menu_language;

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
}
