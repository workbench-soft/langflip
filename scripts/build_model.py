"""Builds model/langflip.model and model/testset.tsv from the wordfreq word lists.

Run it with `mise run model`. The format is in docs/architecture.md.
"""

import struct
from pathlib import Path

from wordfreq import top_n_list, zipf_frequency

ROOT = Path(__file__).resolve().parent.parent
ALPHABETS = {
    # The apostrophe keeps "don't" an English word, though its key types «э».
    "en": "abcdefghijklmnopqrstuvwxyz'",
    "ru": "абвгдеёжзийклмнопрстуфхцчшщъыьэюя",
}
TRAIN_WORDS = 200_000
DICT_WORDS = 100_000
TEST_WORDS = 10_000
MIN_DICT_LEN = 4
# Add-k smoothing keeps a triple the corpus lacks from scoring minus infinity.
SMOOTHING = 0.01


def words_for(lang: str, alphabet: str, count: int) -> list[str]:
    letters = set(alphabet)
    return [w for w in top_n_list(lang, count, wordlist="large") if w and set(w) <= letters]


def triple_log_probs(words: list[str], lang: str, alphabet: str) -> list[float]:
    import math

    size = len(alphabet) + 1  # index 0 marks the word start and end
    index = {c: i + 1 for i, c in enumerate(alphabet)}
    counts = [0.0] * size**3
    for word in words:
        # Zipf values run 1 to 8, so frequent words count more without drowning the rest.
        weight = max(zipf_frequency(word, lang, wordlist="large"), 1.0)
        codes = [0, 0] + [index[c] for c in word] + [0]
        for a, b, c in zip(codes, codes[1:], codes[2:]):
            counts[(a * size + b) * size + c] += weight
    probs = []
    for ab in range(size * size):
        row = counts[ab * size : (ab + 1) * size]
        total = sum(row) + SMOOTHING * size
        probs.extend(math.log((n + SMOOTHING) / total) for n in row)
    return probs


def main() -> None:
    out = bytearray(b"LFM1")
    tests = []
    for lang, alphabet in ALPHABETS.items():
        train = words_for(lang, alphabet, TRAIN_WORDS)
        dictionary = [w for w in train[:DICT_WORDS] if len(w) >= MIN_DICT_LEN]
        held_out = [w for w in train[DICT_WORDS:] if len(w) >= MIN_DICT_LEN]
        tests += [(lang, "dict", w) for w in dictionary[:TEST_WORDS]]
        tests += [(lang, "rare", w) for w in held_out[:TEST_WORDS]]

        encoded = alphabet.encode()
        out += struct.pack("<2sI", lang.encode(), len(encoded)) + encoded
        probs = triple_log_probs(train, lang, alphabet)
        out += struct.pack(f"<{len(probs)}f", *probs)
        blob = "\n".join(sorted(dictionary)).encode()
        out += struct.pack("<I", len(blob)) + blob
        print(f"{lang}: {len(train)} training words, {len(dictionary)} dictionary words")

    (ROOT / "model").mkdir(exist_ok=True)
    (ROOT / "model" / "langflip.model").write_bytes(out)
    lines = [f"{lang}\t{kind}\t{word}" for lang, kind, word in tests]
    (ROOT / "model" / "testset.tsv").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"model: {len(out)} bytes, testset: {len(lines)} words")


if __name__ == "__main__":
    main()
