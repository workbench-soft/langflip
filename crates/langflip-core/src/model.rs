//! Letter-triple tables and dictionaries for both languages, read from `model/langflip.model`.

use std::collections::{HashMap, HashSet};

use crate::layout::Lang;

/// The log probability one character outside the alphabet adds. It sits below any real triple.
const UNKNOWN_CHAR: f32 = -9.0;

pub struct LangModel {
    index: HashMap<char, usize>,
    size: usize,
    log_probs: Vec<f32>,
    dictionary: HashSet<String>,
}

impl LangModel {
    /// Mean log probability per letter of a lowercase word, with start and end marks.
    pub fn score(&self, word: &str) -> f32 {
        let mut prev = [0usize, 0usize];
        let mut total = 0.0;
        let mut steps = 0;
        for c in word.chars().map(Some).chain([None]) {
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
            let dictionary = r.text()?.split('\n').map(str::to_owned).collect();
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
