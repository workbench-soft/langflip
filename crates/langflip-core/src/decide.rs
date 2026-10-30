//! Picks between the word as typed and the same keys in the other layout.

use std::collections::HashSet;

use crate::layout::{Lang, Stroke, in_word, text_for};
use crate::model::Model;

/// Words shorter than this stay as typed.
pub const MIN_LETTERS: usize = 4;

/// How much higher the other layout must score when its word is outside the dictionary.
/// `mise run eval` measures the effect of a change.
pub const THRESHOLD: f32 = 1.0;

/// The part of a word to convert: `strokes[..convert]` change layout, the rest stays as typed.
#[derive(Debug, PartialEq, Eq)]
pub struct Choice {
    pub convert: usize,
    pub text: String,
}

/// Returns the replacement text when the word was typed in the wrong layout.
pub fn decide(
    model: &Model,
    strokes: &[Stroke],
    current: Lang,
    keep_words: &HashSet<String>,
) -> Option<Choice> {
    decide_with(model, strokes, current, keep_words, THRESHOLD)
}

/// [`decide`] with a chosen threshold, for tuning.
pub fn decide_with(
    model: &Model,
    strokes: &[Stroke],
    current: Lang,
    keep_words: &HashSet<String>,
    threshold: f32,
) -> Option<Choice> {
    let other = current.other();
    let mut variants = vec![strokes.len()];
    // "hello," ends in a comma in English, though the same key types «б» in Russian.
    if let Some(last) = strokes.last()
        && !in_word(last.key, current)
    {
        variants.push(strokes.len() - 1);
    }

    let mut scored = Vec::new();
    for &len in &variants {
        let part = &strokes[..len];
        if len < MIN_LETTERS {
            continue;
        }
        let typed = text_for(part, current).to_lowercase();
        if keep_words.contains(&typed) || model.lang(current).contains(&typed) {
            return None;
        }
        let swapped = text_for(part, other).to_lowercase();
        if !part.iter().all(|s| in_word(s.key, other)) {
            continue;
        }
        let typed_score = model.lang(current).score(&typed);
        let swapped_score = model.lang(other).score(&swapped);
        let known = model.lang(other).contains(&swapped);
        scored.push((len, known, swapped_score - typed_score));
    }

    let pick = scored
        .iter()
        .find(|&&(_, known, gain)| known && gain > 0.0)
        .or_else(|| scored.iter().find(|&&(_, _, gain)| gain > threshold))?;
    let len = pick.0;
    let mut text = text_for(&strokes[..len], other);
    text.push_str(&text_for(&strokes[len..], current));
    Some(Choice { convert: len, text })
}

/// How much higher the other layout must score on the first letters of a word.
/// `mise run eval` measures the effect of a change.
pub const PREFIX_GAIN: f32 = 1.0;

/// Returns the text in the other layout when a word still being typed is in the wrong layout.
pub fn decide_prefix(
    model: &Model,
    strokes: &[Stroke],
    current: Lang,
    keep_words: &HashSet<String>,
) -> Option<String> {
    decide_prefix_with(model, strokes, current, keep_words, PREFIX_GAIN)
}

/// [`decide_prefix`] with a chosen gain, for tuning.
pub fn decide_prefix_with(
    model: &Model,
    strokes: &[Stroke],
    current: Lang,
    keep_words: &HashSet<String>,
    gain: f32,
) -> Option<String> {
    let other = current.other();
    // "hello," may end here, and Space decides it.
    let last = strokes.last()?;
    if strokes.len() < MIN_LETTERS
        || !in_word(last.key, current)
        || !strokes.iter().all(|s| in_word(s.key, other))
    {
        return None;
    }
    let typed = text_for(strokes, current).to_lowercase();
    if model.lang(current).starts_word(&typed) || keep_words.iter().any(|w| w.starts_with(&typed)) {
        return None;
    }
    let swapped = text_for(strokes, other).to_lowercase();
    if !model.lang(other).starts_word(&swapped) {
        return None;
    }
    let typed_score = model.lang(current).score_start(&typed);
    let swapped_score = model.lang(other).score_start(&swapped);
    (swapped_score - typed_score > gain).then(|| text_for(strokes, other))
}
