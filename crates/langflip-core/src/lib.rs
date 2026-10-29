//! The platform-free part of Langflip: the word buffer and the decision to fix a word.
//!
//! The platform layer feeds key strokes in and applies each [`Fix`] it gets back.

pub mod decide;
pub mod layout;
pub mod model;

use std::collections::HashSet;

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
    last: Option<LastWord>,
    pub keep_words: HashSet<String>,
}

impl Engine {
    pub fn new(model: Model, keep_words: HashSet<String>) -> Engine {
        Engine {
            model,
            buffer: Vec::new(),
            last: None,
            keep_words,
        }
    }

    pub fn key(&mut self, stroke: Stroke) {
        self.last = None;
        self.buffer.push(stroke);
    }

    pub fn backspace(&mut self) {
        self.last = None;
        self.buffer.pop();
    }

    /// The caret moved, the window changed or a shortcut ran: the word is gone.
    pub fn reset(&mut self) {
        self.last = None;
        self.buffer.clear();
    }

    /// An end key finished the word. Returns the fix to apply before the end key goes through.
    pub fn end_word(&mut self, current: Option<Lang>) -> Option<Fix> {
        let strokes = std::mem::take(&mut self.buffer);
        self.last = None;
        let current = current?;
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

    /// Puts back the word Langflip just fixed, and keeps that word as typed from now on.
    pub fn undo(&mut self) -> Option<Fix> {
        let last = self.last.take()?;
        let fixed = last.fixed?;
        let typed = layout::text_for(&last.strokes, last.typed_in);
        self.keep_words.insert(typed.to_lowercase());
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

    fn type_word(engine: &mut Engine, word: &str, lang: Lang) -> Option<Fix> {
        for s in layout::strokes_for(word, lang).unwrap() {
            engine.key(s);
        }
        engine.end_word(Some(lang))
    }

    fn fixed(word: &str, lang: Lang) -> Option<String> {
        type_word(&mut engine(), word, lang).map(|f| f.text)
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
    fn keeps_short_words() {
        assert_eq!(fixed("rfr", Lang::En), None);
        assert_eq!(fixed("ytn", Lang::En), None);
    }

    #[test]
    fn undo_puts_the_word_back_and_keeps_it() {
        let mut e = engine();
        type_word(&mut e, "ghbdtn", Lang::En).unwrap();
        let undo = e.undo().unwrap();
        assert_eq!(undo.text, "ghbdtn");
        assert_eq!(undo.erase, 7);
        assert_eq!(undo.switch_to, Lang::En);
        assert_eq!(type_word(&mut e, "ghbdtn", Lang::En), None);
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
