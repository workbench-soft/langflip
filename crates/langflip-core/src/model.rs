//! Letter-triple tables and dictionaries for both languages, read from `model/langflip.model`.

use std::collections::HashMap;

use crate::layout::Lang;

/// The log probability one character outside the alphabet adds. It sits below any real triple.
const UNKNOWN_CHAR: f32 = -9.0;

pub struct LangModel {
    index: HashMap<char, usize>,
    size: usize,
    log_probs: Vec<f32>,
    dictionary: Dictionary,
}

/// The words in one string, with the start and end of each. A String per word took 191k heap
/// blocks and about 9 MB, most of the memory Langflip holds.
struct Dictionary {
    text: String,
    /// Sorted by word, so a binary search finds a word or the words that start with some letters.
    spans: Vec<(u32, u32)>,
}

impl Dictionary {
    fn new(text: &str) -> Dictionary {
        let mut words: Vec<&str> = text.split('\n').collect();
        words.sort_unstable();
        let mut joined = String::with_capacity(text.len());
        let spans = words
            .iter()
            .map(|w| {
                let start = joined.len() as u32;
                joined.push_str(w);
                (start, joined.len() as u32)
            })
            .collect();
        Dictionary {
            text: joined,
            spans,
        }
    }

    fn word(&self, &(start, end): &(u32, u32)) -> &str {
        &self.text[start as usize..end as usize]
    }

    fn contains(&self, word: &str) -> bool {
        self.spans
            .binary_search_by(|s| self.word(s).cmp(word))
            .is_ok()
    }

    fn starts_word(&self, start: &str) -> bool {
        let i = self.spans.partition_point(|s| self.word(s) < start);
        self.spans
            .get(i)
            .is_some_and(|s| self.word(s).starts_with(start))
    }
}

impl LangModel {
    /// Mean log probability per letter of a lowercase word, with start and end marks.
    pub fn score(&self, word: &str) -> f32 {
        self.score_with_end(word, true)
    }

    /// [`LangModel::score`] for the start of a word: no end mark.
    pub fn score_start(&self, start: &str) -> f32 {
        self.score_with_end(start, false)
    }

    fn score_with_end(&self, word: &str, end: bool) -> f32 {
        let mut prev = [0usize, 0usize];
        let mut total = 0.0;
        let mut steps = 0;
        let end = end.then_some(None);
        for c in word.chars().map(Some).chain(end) {
            let code = match c {
                None => 0,
                Some(c) => match self.index.get(&c) {
                    Some(&i) => i,
                    None => {
                        // A punctuation mark breaks the word, so the next letter starts afresh.
                        total += UNKNOWN_CHAR;
                        steps += 1;
                        prev = [0, 0];
                        continue;
                    }
                },
            };
            total += self.log_probs[(prev[0] * self.size + prev[1]) * self.size + code];
            steps += 1;
            prev = [prev[1], code];
        }
        total / steps as f32
    }

    pub fn contains(&self, word: &str) -> bool {
        self.dictionary.contains(word)
    }

    /// True when some dictionary word starts with these lowercase letters.
    pub fn starts_word(&self, start: &str) -> bool {
        self.dictionary.starts_word(start)
    }
}

pub struct Model {
    en: LangModel,
    ru: LangModel,
}

#[derive(Debug)]
pub struct ModelError(&'static str);

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bad model file: {}", self.0)
    }
}

impl std::error::Error for ModelError {}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ModelError> {
        if self.bytes.len() < n {
            return Err(ModelError("file ends early"));
        }
        let (head, rest) = self.bytes.split_at(n);
        self.bytes = rest;
        Ok(head)
    }

    fn u32(&mut self) -> Result<usize, ModelError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    }

    fn text(&mut self) -> Result<&'a str, ModelError> {
        let len = self.u32()?;
        std::str::from_utf8(self.take(len)?).map_err(|_| ModelError("text is not UTF-8"))
    }
}

impl Model {
    pub fn from_bytes(bytes: &[u8]) -> Result<Model, ModelError> {
        let mut r = Reader { bytes };
        if r.take(4)? != b"LFM1" {
            return Err(ModelError("wrong magic"));
        }
        let mut langs = HashMap::new();
        for _ in 0..2 {
            let tag = r.take(2)?;
            let alphabet: Vec<char> = r.text()?.chars().collect();
            let size = alphabet.len() + 1;
            let log_probs = r
                .take(size * size * size * 4)?
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&b| f32::from_le_bytes(b))
                .collect();
            let dictionary = Dictionary::new(r.text()?);
            let index = alphabet
                .iter()
                .enumerate()
                .map(|(i, &c)| (c, i + 1))
                .collect();
            let model = LangModel {
                index,
                size,
                log_probs,
                dictionary,
            };
            langs.insert(tag.to_vec(), model);
        }
        let en = langs
            .remove(&b"en"[..])
            .ok_or(ModelError("no English part"))?;
        let ru = langs
            .remove(&b"ru"[..])
            .ok_or(ModelError("no Russian part"))?;
        Ok(Model { en, ru })
    }

    pub fn lang(&self, lang: Lang) -> &LangModel {
        match lang {
            Lang::En => &self.en,
            Lang::Ru => &self.ru,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Dictionary;

    #[test]
    fn the_dictionary_finds_words_and_starts_in_any_order() {
        let d = Dictionary::new("мир\nhello\nhelp\nмирный");
        assert!(d.contains("help") && d.contains("мир") && d.contains("мирный"));
        assert!(!d.contains("hel") && !d.contains("helps"));
        assert!(d.starts_word("hel") && d.starts_word("мирн"));
        assert!(!d.starts_word("hex") && !d.starts_word("мирр"));
    }
}
