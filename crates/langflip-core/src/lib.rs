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

/// The last finished word, kept for Undo, for the manual fix hotkey and for Backspace.
struct LastWord {
    strokes: Vec<Stroke>,
    /// `strokes[..kept]` were on screen before the person typed the rest.
    kept: usize,
    typed_in: Lang,
    /// Some when Langflip fixed the word: the text it typed.
    fixed: Option<String>,
    /// The layout the whole word is on screen in, None when its parts differ.
    shown_in: Option<Lang>,
}

pub struct Engine {
    model: Model,
    buffer: Vec<Stroke>,
    /// `buffer[..kept]` type the word that was on screen before the caret, in `word_lang`.
    kept: usize,
    word_lang: Option<Lang>,
    /// Some once Langflip switched the word being typed: the layout it was typed in before.
    switched_from: Option<Lang>,
    /// The person took back the switch of the word being typed: it joins `keep_words` at its end.
    rejected: bool,
    /// The last key was a fix: a Backspace or Ctrl+Z now takes it back.
    after_fix: bool,
    last: Option<LastWord>,
    pub keep_words: HashSet<String>,
    /// Words of 1 or 2 keys to fix. Shift+Pause adds them.
    pub fix_words: HashSet<String>,
    /// `keep_words` or `fix_words` changed since the platform layer last saved them.
    lists_changed: bool,
    /// Words undone once since start. The second Undo moves a word into `keep_words`.
    undone: HashMap<String, u8>,
}

/// How many times the person undoes a word's fix before Langflip keeps it.
pub const UNDOS_TO_KEEP: u8 = 2;

impl Engine {
    pub fn new(model: Model, keep_words: HashSet<String>, fix_words: HashSet<String>) -> Engine {
        Engine {
            model,
            buffer: Vec::new(),
            kept: 0,
            word_lang: None,
            switched_from: None,
            rejected: false,
            after_fix: false,
            last: None,
            keep_words,
            fix_words,
            lists_changed: false,
            undone: HashMap::new(),
        }
    }

    /// A letter key, before it goes through. Returns a fix when the word so far is in the wrong
    /// layout: the platform swallows the key, and the fix types it in the other layout.
    pub fn key(&mut self, stroke: Stroke, current: Option<Lang>) -> Option<Fix> {
        self.last = None;
        self.after_fix = false;
        self.buffer.push(stroke);
        // Letters typed onto a word already on screen stay as typed.
        if self.switched_from.is_some() || self.word_lang.is_some() {
            return None;
        }
        let current = current?;
        let text = decide::decide_prefix(&self.model, &self.buffer, current, &self.keep_words)?;
        self.switched_from = Some(current);
        self.after_fix = true;
        Some(Fix {
            erase: self.buffer.len() - 1,
            text,
            switch_to: current.other(),
            retype_end: false,
        })
    }

    /// Backspace. Right after a fix it takes the fix back for good. Right after Space, when
    /// `after_space`, it erases the Space and the person goes on with the last word.
    pub fn backspace(&mut self, after_space: bool) {
        self.reject();
        let last = self.last.take();
        if self.buffer.pop().is_none() {
            if let Some(last) = last.filter(|_| after_space) {
                self.go_on_with(last);
            }
            return;
        }
        self.kept = self.kept.min(self.buffer.len());
        if self.buffer.len() == self.kept {
            self.switched_from = None;
            self.rejected = false;
        }
        if self.buffer.is_empty() {
            self.word_lang = None;
        }
    }

    /// Backspace or Ctrl+Z right after a fix: the word as typed becomes an exception. Returns
    /// true when a fix came just before.
    pub fn reject(&mut self) -> bool {
        if !std::mem::take(&mut self.after_fix) {
            return false;
        }
        if self.switched_from.is_some() {
            self.rejected = true;
        } else if let Some(last) = self.last.as_ref().filter(|l| l.fixed.is_some()) {
            let word = layout::text_for(&last.strokes[last.kept..], last.typed_in);
            self.keep(word.to_lowercase());
        }
        true
    }

    /// The word the person rejected while typing it ends: all its keys, as typed, join `keep_words`.
    fn keep_rejected(&mut self, strokes: &[Stroke], kept: usize, typed_in: Option<Lang>) {
        if let Some(lang) = typed_in.filter(|_| std::mem::take(&mut self.rejected))
            && let Some(keys) = strokes.get(kept..).filter(|k| !k.is_empty())
        {
            self.keep(layout::text_for(keys, lang).to_lowercase());
        }
    }

    fn keep(&mut self, word: String) {
        self.undone.remove(&word);
        self.fix_words.remove(&word);
        self.keep_words.insert(word);
        self.lists_changed = true;
    }

    /// True once after `keep_words` or `fix_words` changed, so the platform layer saves them.
    pub fn take_lists_changed(&mut self) -> bool {
        std::mem::take(&mut self.lists_changed)
    }

    fn go_on_with(&mut self, last: LastWord) {
        let Some(lang) = last.shown_in else { return };
        self.kept = last.strokes.len();
        self.buffer = last.strokes;
        self.word_lang = Some(lang);
    }

    /// The text before the caret, read from the app after the caret moved. Its last word becomes
    /// the start of the word being typed. Returns the number of its keys and its layout.
    pub fn continue_word(&mut self, before_caret: &str) -> Option<(usize, Lang)> {
        if !self.buffer.is_empty() {
            return None;
        }
        let (strokes, lang) = layout::word_before(before_caret)?;
        self.kept = strokes.len();
        self.buffer = strokes;
        self.word_lang = Some(lang);
        Some((self.kept, lang))
    }

    /// How many keys the word being typed has.
    pub fn word_len(&self) -> usize {
        self.buffer.len()
    }

    /// The caret moved, the window changed or a shortcut ran: the word is gone.
    pub fn reset(&mut self) {
        let strokes = std::mem::take(&mut self.buffer);
        self.keep_rejected(&strokes, self.kept, self.switched_from);
        self.last = None;
        self.after_fix = false;
        self.kept = 0;
        self.word_lang = None;
        self.switched_from = None;
    }

    /// An end key finished the word. Returns the fix to apply before the end key goes through.
    pub fn end_word(&mut self, current: Option<Lang>) -> Option<Fix> {
        let strokes = std::mem::take(&mut self.buffer);
        let kept = std::mem::take(&mut self.kept);
        let word_lang = self.word_lang.take();
        let switched_from = self.switched_from.take();
        self.keep_rejected(&strokes, kept, switched_from);
        self.last = None;
        self.after_fix = false;
        // A second Space ends no word, and the word before it stays out of reach.
        if strokes.is_empty() {
            return None;
        }
        let current = current?;
        if let Some(typed_in) = switched_from {
            // Switched while typed: a second decision could flip it back.
            let fixed = Some(layout::text_for(&strokes[kept..], typed_in.other()));
            self.last = Some(LastWord {
                strokes,
                kept,
                typed_in,
                fixed,
                shown_in: Some(typed_in.other()),
            });
            return None;
        }
        let (text, shown_in) = match word_lang {
            // Letters typed onto a word already on screen stay as typed.
            Some(lang) => (
                None,
                (current == lang || kept == strokes.len()).then_some(lang),
            ),
            None => {
                let lists = (&self.keep_words, &self.fix_words);
                match decide::decide_with(&self.model, &strokes, current, lists, decide::THRESHOLD)
                {
                    Some(c) => {
                        let whole = c.convert == strokes.len();
                        (Some(c.text), whole.then_some(current.other()))
                    }
                    None => (None, Some(current)),
                }
            }
        };
        let fix = text.map(|text| Fix {
            erase: strokes.len(),
            text,
            switch_to: current.other(),
            retype_end: false,
        });
        let fixed = fix.as_ref().map(|f| f.text.clone());
        self.after_fix = fixed.is_some();
        self.last = Some(LastWord {
            strokes,
            kept,
            typed_in: current,
            fixed,
            shown_in,
        });
        fix
    }

    /// Puts back the word Langflip just fixed. The second Undo of a word keeps it as typed from now on.
    pub fn undo(&mut self) -> Option<Fix> {
        self.after_fix = false;
        let last = self.last.take()?;
        let fixed = last.fixed?;
        let typed = layout::text_for(&last.strokes[last.kept..], last.typed_in);
        let word = typed.to_lowercase();
        let count = self.undone.entry(word.clone()).or_default();
        *count += 1;
        if *count >= UNDOS_TO_KEEP {
            self.keep(word);
        }
        Some(Fix {
            erase: fixed.chars().count() + 1,
            text: typed,
            switch_to: last.typed_in,
            retype_end: true,
        })
    }

    /// A word of 1 or 2 keys the person converts by hand: Langflip fixes it from now on.
    fn fix_short(&mut self, strokes: &[Stroke], typed_in: Lang) {
        if strokes.len() < decide::SHORT_KEYS && decide::letters(strokes) > 0 {
            let word = layout::text_for(strokes, typed_in).to_lowercase();
            self.keep_words.remove(&word);
            self.fix_words.insert(word);
            self.lists_changed = true;
        }
    }

    /// Converts the keys typed of the word being typed, or of the last word, to the other layout.
    pub fn convert(&mut self, current: Option<Lang>) -> Option<Fix> {
        self.after_fix = false;
        if self.buffer.len() > self.kept {
            let current = current?;
            let kept = self.kept;
            let strokes = std::mem::take(&mut self.buffer);
            self.reset();
            if kept == 0 {
                self.fix_short(&strokes, current);
            }
            return Some(Fix {
                erase: strokes.len() - kept,
                text: layout::text_for(&strokes[kept..], current.other()),
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
        let added = &last.strokes[last.kept..];
        if last.kept == 0 {
            self.fix_short(added, last.typed_in);
        }
        Some(Fix {
            erase: added.len() + 1,
            text: layout::text_for(added, target),
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
        Engine::new(
            Model::from_bytes(bytes).unwrap(),
            HashSet::new(),
            HashSet::new(),
        )
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
    fn fixes_words_of_3_letters() {
        for (typed, lang, word) in [
            ("rfr", Lang::En, "как"),
            ("ytn", Lang::En, "нет"),
            ("еру", Lang::Ru, "the"),
        ] {
            assert_eq!(fixed(typed, lang).as_deref(), Some(word), "{typed}");
        }
    }

    #[test]
    fn words_of_1_or_2_keys_stay_until_the_person_adds_them() {
        let cases = [
            ("z", Lang::En, "я"),
            ("Z", Lang::En, "Я"),
            ("yt", Lang::En, "не"),
            ("nj", Lang::En, "то"),
            ("z,", Lang::En, "я,"),
            ("ш", Lang::Ru, "i"),
            ("шт", Lang::Ru, "in"),
        ];
        for (typed, lang, _) in cases {
            assert_eq!(fixed(typed, lang), None, "{typed}");
        }
        let mut e = engine();
        e.fix_words
            .extend(["z", "yt", "nj", "ш", "шт"].map(String::from));
        for (typed, lang, word) in cases {
            type_word(&mut e, typed, lang).unwrap();
            let last = e.last.as_ref().unwrap();
            assert_eq!(last.fixed.as_deref(), Some(word), "{typed}");
        }
    }

    #[test]
    fn convert_adds_a_short_word_to_fix_words() {
        let mut e = engine();
        assert_eq!(type_word(&mut e, "yt", Lang::En), None);
        assert_eq!(e.convert(Some(Lang::En)).unwrap().text, "не");
        assert!(e.fix_words.contains("yt") && e.take_lists_changed());
        assert!(type_word(&mut e, "yt", Lang::En).is_some());

        // A longer word converted by hand joins no list.
        let mut e = engine();
        type_word(&mut e, "hello", Lang::En);
        e.convert(Some(Lang::En)).unwrap();
        assert!(e.fix_words.is_empty() && !e.take_lists_changed());
    }

    #[test]
    fn backspace_right_after_a_fix_keeps_the_word() {
        let mut e = engine();
        type_word(&mut e, "ytn", Lang::En).unwrap();
        e.backspace(true);
        assert!(e.keep_words.contains("ytn") && e.take_lists_changed());
        e.reset();
        assert_eq!(type_word(&mut e, "ytn", Lang::En), None);
    }

    #[test]
    fn ctrl_z_right_after_a_fix_keeps_the_word() {
        let mut e = engine();
        e.fix_words.insert("z".to_owned());
        type_word(&mut e, "z", Lang::En).unwrap();
        e.reject();
        e.reset();
        assert!(e.keep_words.contains("z") && !e.fix_words.contains("z"));
        assert_eq!(type_word(&mut e, "z", Lang::En), None);
    }

    #[test]
    fn backspace_later_keeps_nothing() {
        let mut e = engine();
        type_word(&mut e, "ytn", Lang::En).unwrap();
        e.end_word(Some(Lang::Ru));
        e.backspace(true);
        let mut e2 = engine();
        type_word(&mut e2, "ghbdtn", Lang::En).unwrap();
        e2.key(
            layout::strokes_for("а", Lang::Ru).unwrap()[0],
            Some(Lang::Ru),
        );
        e2.backspace(false);
        assert!(e.keep_words.is_empty() && e2.keep_words.is_empty());
    }

    #[test]
    fn backspace_right_after_a_switch_while_typed_keeps_the_whole_word() {
        let mut e = engine();
        let mut current = Lang::En;
        for s in layout::strokes_for("ghbd", Lang::En).unwrap() {
            if let Some(fix) = e.key(s, Some(current)) {
                current = fix.switch_to;
            }
        }
        e.backspace(false);
        // The keys of «д» and «т», then Space.
        for s in layout::strokes_for("дт", Lang::Ru).unwrap() {
            assert_eq!(e.key(s, Some(current)), None);
        }
        assert_eq!(e.end_word(Some(current)), None);
        assert!(e.keep_words.contains("ghbln"), "{:?}", e.keep_words);
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
    fn a_word_ended_in_a_third_layout_leaves_no_switch_behind() {
        let mut e = engine();
        for s in layout::strokes_for("ghbd", Lang::En).unwrap() {
            e.key(s, Some(Lang::En));
        }
        // Alt+Shift went on to a third layout, such as Ukrainian, before Space.
        assert_eq!(e.end_word(None), None);
        assert!(e.undo().is_none());
        assert!(type_word(&mut e, "ghbdtn", Lang::En).is_some());
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

    /// Types `added` in `lang` onto the word before the caret, then Space. Returns the fixes.
    fn go_on(e: &mut Engine, added: &str, lang: Lang) -> Vec<Fix> {
        let mut current = lang;
        let mut fixes = Vec::new();
        for s in layout::strokes_for(added, lang).unwrap() {
            if let Some(fix) = e.key(s, Some(current)) {
                current = fix.switch_to;
                fixes.push(fix);
            }
        }
        fixes.extend(e.end_word(Some(current)));
        fixes
    }

    #[test]
    fn keys_in_the_word_layout_stay() {
        let mut e = engine();
        assert_eq!(e.continue_word("Скажи при"), Some((3, Lang::Ru)));
        assert_eq!(go_on(&mut e, "ятно", Lang::Ru), []);
        assert_eq!(e.continue_word("test"), Some((4, Lang::En)));
        assert_eq!(go_on(&mut e, "ing", Lang::En), []);
    }

    #[test]
    fn keys_in_the_other_layout_stay_on_a_word_on_screen() {
        let mut e = engine();
        e.continue_word("привет");
        assert_eq!(go_on(&mut e, "cndbt", Lang::En), []);
        e.continue_word("hel");
        assert_eq!(go_on(&mut e, "дщ", Lang::Ru), []);
    }

    #[test]
    fn a_sign_that_may_end_the_word_waits_for_the_next_key() {
        // «трубка» typed in English: "nhe," may be an English word and its comma.
        let mut e = engine();
        let switched: Vec<bool> = layout::strokes_for("nhe,rf", Lang::En)
            .unwrap()
            .into_iter()
            .map(|s| e.key(s, Some(Lang::En)).is_some())
            .collect();
        assert_eq!(switched, [false, false, false, false, true, false]);
    }

    #[test]
    fn a_mixed_word_stays_as_typed() {
        let mut e = engine();
        e.continue_word("Windows");
        assert_eq!(go_on(&mut e, "ом", Lang::Ru), []);
    }

    #[test]
    fn backspace_after_space_goes_on_with_the_word() {
        let mut e = engine();
        assert_eq!(type_word(&mut e, "работ", Lang::Ru), None);
        e.backspace(true);
        assert_eq!(go_on(&mut e, "f", Lang::En), []);

        // Enter may have sent the word away.
        let mut e = engine();
        type_word(&mut e, "работ", Lang::Ru);
        e.backspace(false);
        assert!(e.buffer.is_empty() && e.word_lang.is_none());
    }

    #[test]
    fn backspace_after_a_second_space_starts_a_new_word() {
        let mut e = engine();
        assert!(type_word(&mut e, "ytn", Lang::En).is_some());
        // A second Space, then Backspace takes it back.
        assert_eq!(e.end_word(Some(Lang::Ru)), None);
        e.backspace(true);
        assert_eq!(go_on(&mut e, "hello", Lang::En), []);
    }

    #[test]
    fn backspace_into_the_old_word_lets_it_go() {
        let mut e = engine();
        e.continue_word("ab");
        for _ in 0..3 {
            e.backspace(true);
        }
        assert!(e.buffer.is_empty() && e.word_lang.is_none());
    }
}
