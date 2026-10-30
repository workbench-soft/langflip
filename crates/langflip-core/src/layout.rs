//! Key positions and the letters the US QWERTY and Russian ЙЦУКЕН layouts put on them.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    Ru,
}

impl Lang {
    pub fn other(self) -> Lang {
        match self {
            Lang::En => Lang::Ru,
            Lang::Ru => Lang::En,
        }
    }
}

/// A key that types a letter in at least one of the two layouts, named after the US key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    BracketLeft,
    BracketRight,
    Semicolon,
    Quote,
    Comma,
    Period,
    Backquote,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Digit0,
}

impl Key {
    pub fn is_digit(self) -> bool {
        row(self).1.is_ascii_digit()
    }
}

/// One key press: the key and the modifier state that picks its case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stroke {
    pub key: Key,
    pub shift: bool,
    pub caps: bool,
}

impl Stroke {
    pub fn new(key: Key, shift: bool, caps: bool) -> Stroke {
        Stroke { key, shift, caps }
    }
}

// Each row: key, English lower, English shifted, Russian lower.
const TABLE: [(Key, char, char, char); 43] = [
    (Key::A, 'a', 'A', 'ф'),
    (Key::B, 'b', 'B', 'и'),
    (Key::C, 'c', 'C', 'с'),
    (Key::D, 'd', 'D', 'в'),
    (Key::E, 'e', 'E', 'у'),
    (Key::F, 'f', 'F', 'а'),
    (Key::G, 'g', 'G', 'п'),
    (Key::H, 'h', 'H', 'р'),
    (Key::I, 'i', 'I', 'ш'),
    (Key::J, 'j', 'J', 'о'),
    (Key::K, 'k', 'K', 'л'),
    (Key::L, 'l', 'L', 'д'),
    (Key::M, 'm', 'M', 'ь'),
    (Key::N, 'n', 'N', 'т'),
    (Key::O, 'o', 'O', 'щ'),
    (Key::P, 'p', 'P', 'з'),
    (Key::Q, 'q', 'Q', 'й'),
    (Key::R, 'r', 'R', 'к'),
    (Key::S, 's', 'S', 'ы'),
    (Key::T, 't', 'T', 'е'),
    (Key::U, 'u', 'U', 'г'),
    (Key::V, 'v', 'V', 'м'),
    (Key::W, 'w', 'W', 'ц'),
    (Key::X, 'x', 'X', 'ч'),
    (Key::Y, 'y', 'Y', 'н'),
    (Key::Z, 'z', 'Z', 'я'),
    (Key::BracketLeft, '[', '{', 'х'),
    (Key::BracketRight, ']', '}', 'ъ'),
    (Key::Semicolon, ';', ':', 'ж'),
    (Key::Quote, '\'', '"', 'э'),
    (Key::Comma, ',', '<', 'б'),
    (Key::Period, '.', '>', 'ю'),
    (Key::Backquote, '`', '~', 'ё'),
    // Digits type the same in both layouts. With Shift they give other signs, and the
    // platform layer clears the word on them.
    (Key::Digit1, '1', '!', '1'),
    (Key::Digit2, '2', '@', '2'),
    (Key::Digit3, '3', '#', '3'),
    (Key::Digit4, '4', '$', '4'),
    (Key::Digit5, '5', '%', '5'),
    (Key::Digit6, '6', '^', '6'),
    (Key::Digit7, '7', '&', '7'),
    (Key::Digit8, '8', '*', '8'),
    (Key::Digit9, '9', '(', '9'),
    (Key::Digit0, '0', ')', '0'),
];

fn row(key: Key) -> &'static (Key, char, char, char) {
    TABLE
        .iter()
        .find(|r| r.0 == key)
        .expect("every key has a row")
}

/// True when the key types a letter in this layout.
pub fn is_letter(key: Key, lang: Lang) -> bool {
    match lang {
        Lang::Ru => !key.is_digit(),
        Lang::En => row(key).1.is_ascii_alphabetic(),
    }
}

/// True when the key can sit inside a word of this layout: a letter, a digit, or the apostrophe
/// of "don't".
pub fn in_word(key: Key, lang: Lang) -> bool {
    is_letter(key, lang) || key.is_digit() || (lang == Lang::En && key == Key::Quote)
}

/// The character the stroke types in this layout. Caps Lock flips the case of letters only.
pub fn char_for(stroke: Stroke, lang: Lang) -> char {
    let &(_, en, en_shift, ru) = row(stroke.key);
    let letter = is_letter(stroke.key, lang);
    let upper = if letter {
        stroke.shift ^ stroke.caps
    } else {
        stroke.shift
    };
    match lang {
        Lang::En if upper => en_shift,
        Lang::En => en,
        Lang::Ru if upper => ru.to_uppercase().next().unwrap_or(ru),
        Lang::Ru => ru,
    }
}

/// The text the strokes type in this layout.
pub fn text_for(strokes: &[Stroke], lang: Lang) -> String {
    strokes.iter().map(|&s| char_for(s, lang)).collect()
}

/// The strokes that type this word in this layout, or None when a character has no key.
pub fn strokes_for(word: &str, lang: Lang) -> Option<Vec<Stroke>> {
    word.chars()
        .map(|c| {
            TABLE.iter().find_map(|&(key, en, en_shift, ru)| {
                let (lower, upper) = match lang {
                    Lang::En => (en, en_shift),
                    Lang::Ru => (ru, ru.to_uppercase().next().unwrap_or(ru)),
                };
                if c == lower {
                    Some(Stroke::new(key, false, false))
                } else if c == upper {
                    Some(Stroke::new(key, true, false))
                } else {
                    None
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_keys_in_both_layouts() {
        let strokes = strokes_for("ghbdtn", Lang::En).unwrap();
        assert_eq!(text_for(&strokes, Lang::Ru), "привет");
        let strokes = strokes_for("hf,jnf", Lang::En).unwrap();
        assert_eq!(text_for(&strokes, Lang::Ru), "работа");
    }

    #[test]
    fn keeps_case() {
        let strokes = strokes_for("Ghbdtn", Lang::En).unwrap();
        assert_eq!(text_for(&strokes, Lang::Ru), "Привет");
        let caps = Stroke::new(Key::Comma, false, true);
        assert_eq!(char_for(caps, Lang::Ru), 'Б');
        assert_eq!(char_for(caps, Lang::En), ',');
    }
}
