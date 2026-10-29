//! Scores the model on `model/testset.tsv` at several thresholds.
//!
//! Each test word is typed twice: in its own layout, where a fix is a false fix, and in the
//! other layout, where anything but the original word is a miss.

use std::collections::HashSet;

use langflip_core::Model;
use langflip_core::decide::{THRESHOLD, decide_with};
use langflip_core::layout::{Lang, strokes_for};

fn main() {
    let model = Model::from_bytes(include_bytes!("../../../model/langflip.model")).unwrap();
    let tests = include_str!("../../../model/testset.tsv");
    let keep = HashSet::new();
    println!("threshold  lang kind  words  false fixes  misses");
    for threshold in [0.5, 0.75, 1.0, 1.5, 2.0] {
        let mark = if threshold == THRESHOLD {
            " (current)"
        } else {
            ""
        };
        for lang in [Lang::En, Lang::Ru] {
            for kind in ["dict", "rare"] {
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
                    if decide_with(&model, &strokes, lang, &keep, threshold).is_some() {
                        false_fixes += 1;
                    }
                    let fixed = decide_with(&model, &strokes, lang.other(), &keep, threshold);
                    if fixed.map(|c| c.text) != Some(word.to_owned()) {
                        misses += 1;
                    }
                }
                let pct = |n: i32| 100.0 * n as f64 / words.max(1) as f64;
                println!(
                    "{threshold:>9}  {tag:>4} {kind:>4}  {words:>5}  {:>10.2}%  {:>5.2}%{mark}",
                    pct(false_fixes),
                    pct(misses)
                );
            }
        }
    }
}
