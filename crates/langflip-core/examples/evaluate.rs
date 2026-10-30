//! Scores the model on `model/testset.tsv` at several thresholds.
//!
//! Each test word is typed twice: in its own layout, where a fix is a false fix, and in the
//! other layout, where anything but the original word is a miss. Keys go in one at a time, so
//! a switch while the word is typed counts too. Gain `inf` turns that switch off.

use std::collections::HashSet;

use langflip_core::Model;
use langflip_core::decide::{PREFIX_GAIN, THRESHOLD, decide_prefix_with, decide_with};
use langflip_core::layout::{Lang, Stroke, strokes_for, text_for};

/// The word on screen once Space ends it, or None when it stays as typed.
fn result(
    model: &Model,
    strokes: &[Stroke],
    lang: Lang,
    threshold: f32,
    gain: f32,
) -> Option<String> {
    let keep = HashSet::new();
    for len in 1..=strokes.len() {
        if decide_prefix_with(model, &strokes[..len], lang, &keep, gain).is_some() {
            return Some(text_for(strokes, lang.other()));
        }
    }
    decide_with(model, strokes, lang, &keep, threshold).map(|c| c.text)
}

fn main() {
    let model = Model::from_bytes(include_bytes!("../../../model/langflip.model")).unwrap();
    let tests = include_str!("../../../model/testset.tsv");
    println!("threshold   gain  lang kind  words  false fixes  misses");
    let runs = [0.5, 0.75, 1.0, 1.5, 2.0]
        .map(|t| (t, PREFIX_GAIN))
        .into_iter()
        .chain([f32::INFINITY, 0.5, 1.0].map(|g| (THRESHOLD, g)));
    for (threshold, gain) in runs {
        let mark = if threshold == THRESHOLD && gain == PREFIX_GAIN {
            " (current)"
        } else {
            ""
        };
        for lang in [Lang::En, Lang::Ru] {
            // "short" words have 1 to 3 letters, "srare" are rarer short words the dictionary lacks.
            for kind in ["dict", "rare", "short", "srare"] {
                let (mut words, mut false_fixes, mut misses) = (0, 0, 0);
                let tag = if lang == Lang::En { "en" } else { "ru" };
                for line in tests.lines() {
                    let mut cols = line.split('\t');
                    if cols.next() != Some(tag) || cols.next() != Some(kind) {
                        continue;
                    }
                    let word = cols.next().unwrap();
                    let Some(strokes) = strokes_for(word, lang) else {
                        continue;
                    };
                    words += 1;
                    if result(&model, &strokes, lang, threshold, gain).is_some() {
                        false_fixes += 1;
                    }
                    let fixed = result(&model, &strokes, lang.other(), threshold, gain);
                    if fixed.as_deref() != Some(word) {
                        misses += 1;
                    }
                }
                let pct = |n: i32| 100.0 * n as f64 / words.max(1) as f64;
                println!(
                    "{threshold:>9}  {gain:>5}  {tag:>4} {kind:>4}  {words:>5}  {:>10.2}%  {:>5.2}%{mark}",
                    pct(false_fixes),
                    pct(misses)
                );
            }
        }
    }
}
