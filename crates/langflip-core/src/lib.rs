//! The platform-free part of Langflip: the word buffer and the decision to fix a word.
//!
//! The platform layer feeds key strokes in and applies each [`Fix`] it gets back.

pub mod decide;
pub mod layout;
pub mod model;

use std::collections::{HashMap, HashSet};

pub use layout::{Key, Lang, Stroke};
pub use model::{Model, ModelError};

/// What the platform layer types: `erase` Backspaces, then `text`, then a layout switch.
#[derive(Debug, PartialEq, Eq)]
pub struct Fix {
    pub erase: usize,
    pub text: String,
    pub switch_to: Lang,
    /// The end key the person pressed is part of `erase`, and the platform types it again after `text`.
    pub retype_end: bool,
}

/// The last finished word, kept for Undo and for the manual fix hotkey.
struct LastWord {
    strokes: Vec<Stroke>,
    typed_in: Lang,
    /// Some when Langflip fixed the word: the text it typed.
    fixed: Option<String>,
}

pub struct Engine {
    model: Model,
    buffer: Vec<Stroke>,
    /// Some once Langflip switched the word being typed: the layout it was typed in before.
    switched_from: Option<Lang>,
    last: Option<LastWord>,
    pub keep_words: HashSet<String>,
    /// Words undone once since start. The second Undo moves a word into `keep_words`.
    undone: HashMap<String, u8>,
}

/// How many times the person undoes a word's fix before Langflip keeps it.
pub const UNDOS_TO_KEEP: u8 = 2;

impl Engine {
    pub fn new(model: Model, keep_words: HashSet<String>) -> Engine {
        Engine {
            model,
            buffer: Vec::new(),
            switched_from: None,
            last: None,
            keep_words,
            undone: HashMap::new(),
        }
    }

    /// A letter key, before it goes through. Returns a fix when the word so far is in the wrong
    /// layout: the platform swallows the key, and the fix types it in the other layout.
    pub fn key(&mut self, stroke: Stroke, current: Option<Lang>) -> Option<Fix> {
        self.last = None;
        self.buffer.push(stroke);
        if self.switched_from.is_some() {
            return None;
        }
        let current = current?;
        let text = decide::decide_prefix(&self.model, &self.buffer, current, &self.keep_words)?;
        self.switched_from = Some(current);
        Some(Fix {
            erase: self.buffer.len() - 1,
            text,
            switch_to: current.other(),
            retype_end: false,
        })
    }

    pub fn backspace(&mut self) {
        self.last = None;
        self.buffer.pop();
        if self.buffer.is_empty() {
            self.switched_from = None;
        }
    }

    /// How many keys the word being typed has.
    pub fn word_len(&self) -> usize {
        self.buffer.len()
    }

    /// The caret moved, the window changed or a shortcut ran: the word is gone.
    pub fn reset(&mut self) {
        self.last = None;
        self.buffer.clear();
        self.switched_from = None;
    }

    /// An end key finished the word. Returns the fix to apply before the end key goes through.
    pub fn end_word(&mut self, current: Option<Lang>) -> Option<Fix> {
        let strokes = std::mem::take(&mut self.buffer);
        self.last = None;
        let current = current?;
        if let Some(typed_in) = self.switched_from.take() {
            // Switched while typed: a second decision could flip it back.
            let fixed = Some(layout::text_for(&strokes, typed_in.other()));
            self.last = Some(LastWord {
                strokes,
                typed_in,
                fixed,
            });
            return None;
        }
        let choice = decide::decide(&self.model, &strokes, current, &self.keep_words);
        let fix = choice.map(|c| Fix {
            erase: strokes.len(),
            text: c.text,
            switch_to: current.other(),
            retype_end: false,
        });
        let fixed = fix.as_ref().map(|f| f.text.clone());
        self.last = Some(LastWord {
            strokes,
            typed_in: current,
            fixed,
        });
        fix
    }

    /// Puts back the word Langflip just fixed. The second Undo of a word keeps it as typed from now on.
    pub fn undo(&mut self) -> Option<Fix> {
        let last = self.last.take()?;
        let fixed = last.fixed?;
        let typed = layout::text_for(&last.strokes, last.typed_in);
        let word = typed.to_lowercase();
        let count = self.undone.entry(word.clone()).or_default();
        *count += 1;
        if *count >= UNDOS_TO_KEEP {
            self.undone.remove(&word);
            self.keep_words.insert(word);
        }
        Some(Fix {
            erase: fixed.chars().count() + 1,
            text: typed,
            switch_to: last.typed_in,
            retype_end: true,
        })
    }

    /// Converts the word being typed, or the last word, to the other layout.
    pub fn convert(&mut self, current: Option<Lang>) -> Option<Fix> {
        if !self.buffer.is_empty() {
            let current = current?;
            self.switched_from = None;
            let strokes = std::mem::take(&mut self.buffer);
            let text = layout::text_for(&strokes, current.other());
            return Some(Fix {
                erase: strokes.len(),
                text,
                switch_to: current.other(),
                retype_end: false,
            });
        }
        let last = self.last.take()?;
        if last.fixed.is_some() {
            self.last = Some(last);
            return self.undo();
        }
        let target = last.typed_in.other();
        Some(Fix {
            erase: last.strokes.len() + 1,
            text: layout::text_for(&last.strokes, target),
            switch_to: target,
            retype_end: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let bytes = include_bytes!("../../../model/langflip.model");
        Engine::new(Model::from_bytes(bytes).unwrap(), HashSet::new())
    }

    /// Types the word key by key and ends it with Space. A switch while typed changes the layout
    /// of the keys that follow, as on a real keyboard. Returns the last fix.
    fn type_word(engine: &mut Engine, word: &str, lang: Lang) -> Option<Fix> {
        let mut current = lang;
        let mut last = None;
        for s in layout::strokes_for(word, lang).unwrap() {
            if let Some(fix) = engine.key(s, Some(current)) {
                current = fix.switch_to;
                last = Some(fix);
            }
        }
        engine.end_word(Some(current)).or(last)
    }

    /// The word on screen after Space, or None when it stays as typed.
    fn fixed(word: &str, lang: Lang) -> Option<String> {
        let mut e = engine();
        type_word(&mut e, word, lang)?;
        let last = e.last.as_ref().unwrap();
        last.fixed.clone()
    }

    #[test]
    fn fixes_russian_typed_in_english() {
        assert_eq!(fixed("ghbdtn", Lang::En).as_deref(), Some("привет"));
        assert_eq!(fixed("hf,jnf", Lang::En).as_deref(), Some("работа"));
        assert_eq!(fixed("Vjcrdf", Lang::En).as_deref(), Some("Москва"));
        assert_eq!(fixed("k.,k.", Lang::En).as_deref(), Some("люблю"));
    }

    #[test]
    fn fixes_english_typed_in_russian() {
        assert_eq!(fixed("руддщ", Lang::Ru).as_deref(), Some("hello"));
        assert_eq!(fixed("цщкдв", Lang::Ru).as_deref(), Some("world"));
        assert_eq!(fixed("вщтэе", Lang::Ru).as_deref(), Some("don't"));
    }

    #[test]
    fn keeps_right_words() {
        for (word, lang) in [
            ("hello", Lang::En),
            ("hello,", Lang::En),
            ("don't", Lang::En),
            ("const", Lang::En),
            ("привет", Lang::Ru),
            ("работа", Lang::Ru),
        ] {
            assert_eq!(fixed(word, lang), None, "{word}");
        }
    }

    #[test]
    fn fixes_short_words() {
        for (typed, lang, word) in [
            ("z", Lang::En, "я"),
            ("Z", Lang::En, "Я"),
            ("yt", Lang::En, "не"),
            ("nj", Lang::En, "то"),
            ("rfr", Lang::En, "как"),
            ("ytn", Lang::En, "нет"),
            ("z,", Lang::En, "я,"),
            ("ш", Lang::Ru, "i"),
            ("шт", Lang::Ru, "in"),
            ("еру", Lang::Ru, "the"),
        ] {
            assert_eq!(fixed(typed, lang).as_deref(), Some(word), "{typed}");
        }
    }

    #[test]
    fn keeps_short_words() {
        for (word, lang) in [
            ("a", Lang::En),
            ("i", Lang::En),
            ("is", Lang::En),
            ("the", Lang::En),
            ("я", Lang::Ru),
            ("в", Lang::Ru),
            ("не", Lang::Ru),
            ("как", Lang::Ru),
            ("x", Lang::En),
        ] {
            assert_eq!(fixed(word, lang), None, "{word}");
        }
    }

    #[test]
    fn words_with_digits() {
        assert_eq!(fixed("ьз3", Lang::Ru).as_deref(), Some("mp3"));
        assert_eq!(fixed("зы4", Lang::Ru).as_deref(), Some("ps4"));
        for (word, lang) in [("mp3", Lang::En), ("2026", Lang::En), ("2026", Lang::Ru)] {
            assert_eq!(fixed(word, lang), None, "{word}");
        }
    }

    #[test]
    fn switches_while_typed() {
        let mut e = engine();
        let strokes = layout::strokes_for("ghbdtn", Lang::En).unwrap();
        let fixes: Vec<_> = strokes
            .iter()
            .map(|&s| e.key(s, Some(Lang::En)).map(|f| (f.erase, f.text)))
            .collect();
        assert_eq!(fixes[..3], [None, None, None]);
        assert_eq!(fixes[3], Some((3, "прив".to_owned())));
        assert_eq!(fixes[4..], [None, None]);
        assert_eq!(e.end_word(Some(Lang::Ru)), None);
    }

    #[test]
    fn waits_on_punctuation_and_known_starts() {
        let mut e = engine();
        for s in layout::strokes_for("hello,", Lang::En).unwrap() {
            assert_eq!(e.key(s, Some(Lang::En)), None);
        }
    }

    #[test]
    fn undo_after_a_switch_while_typed() {
        let mut e = engine();
        for _ in 0..UNDOS_TO_KEEP {
            assert!(type_word(&mut e, "ghbdtn", Lang::En).is_some());
            let undo = e.undo().unwrap();
            assert_eq!(undo.text, "ghbdtn");
            assert_eq!(undo.erase, 7);
            assert_eq!(undo.switch_to, Lang::En);
        }
        for s in layout::strokes_for("ghbdtn", Lang::En).unwrap() {
            assert_eq!(e.key(s, Some(Lang::En)), None);
        }
    }

    #[test]
    fn undo_puts_the_word_back() {
        let mut e = engine();
        type_word(&mut e, "ghbdtn", Lang::En).unwrap();
        let undo = e.undo().unwrap();
        assert_eq!(undo.text, "ghbdtn");
        assert_eq!(undo.erase, 7);
        assert_eq!(undo.switch_to, Lang::En);
        assert!(e.keep_words.is_empty());
    }

    #[test]
    fn the_second_undo_keeps_the_word() {
        let mut e = engine();
        type_word(&mut e, "ghbdtn", Lang::En).unwrap();
        e.undo().unwrap();
        type_word(&mut e, "ghbdtn", Lang::En).unwrap();
        e.undo().unwrap();
        assert!(e.keep_words.contains("ghbdtn"));
        assert_eq!(type_word(&mut e, "ghbdtn", Lang::En), None);
    }

    #[test]
    fn undos_of_different_words_add_up_apart() {
        let mut e = engine();
        type_word(&mut e, "ghbdtn", Lang::En).unwrap();
        e.undo().unwrap();
        type_word(&mut e, "hf,jnf", Lang::En).unwrap();
        e.undo().unwrap();
        assert!(e.keep_words.is_empty());
        assert!(type_word(&mut e, "ghbdtn", Lang::En).is_some());
    }

    #[test]
    fn convert_turns_the_word_being_typed() {
        let mut e = engine();
        for s in layout::strokes_for("ghb", Lang::En).unwrap() {
            assert_eq!(e.key(s, Some(Lang::En)), None);
        }
        let fix = e.convert(Some(Lang::En)).unwrap();
        assert_eq!(fix.text, "при");
        assert_eq!(fix.erase, 3);
        assert!(!fix.retype_end);
    }

    #[test]
    fn convert_turns_the_last_word() {
        let mut e = engine();
        assert_eq!(type_word(&mut e, "hello", Lang::En), None);
        let fix = e.convert(Some(Lang::En)).unwrap();
        assert_eq!(fix.text, "руддщ");
        assert_eq!(fix.erase, 6);
        assert!(fix.retype_end);
    }
}
