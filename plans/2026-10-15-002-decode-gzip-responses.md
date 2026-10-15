# Decode gzip-compressed article responses in the scraper

**Status**: Implemented

## Symptoms

In the 2026-10-15 full-article runs (plan `2026-10-14-002`), the "Gemini 4
Argon" story (`50073879`, `https://deepmind.google/models/gemini/`) produced a
33,168-token prompt whose summary was just the title restated:

> P1: The story is titled "Gemini 4 Argon."

`tmp/50073879.txt` is 93 KB of bytes like `���m{�ƒ��~?F�s��>�X��'Ή�8q`. Of
the 58 scraped pages that day, 2 were like this (over 1% U+FFFD replacement
characters after decoding): `50073879` and `50086363`
(`https://dfarq.homeip.net/nvidias-first-gpu-geforce-256/`, 138 KB).

Both servers send `content-encoding: gzip` even when the request carries no
`Accept-Encoding` header:

```
$ curl -sI https://deepmind.google/models/gemini/ | grep -i content-encoding
content-encoding: gzip
$ curl -sI https://dfarq.homeip.net/nvidias-first-gpu-geforce-256/ | grep -i content-encoding
Content-Encoding: gzip
```

## Root cause

`Cargo.toml` builds `reqwest` 0.13 with `default-features = false` and only
`rustls` + `json`. Without the `gzip` feature reqwest neither advertises
`Accept-Encoding: gzip` nor decodes a gzip body, so `response.text()` hands
the raw deflate stream to `html2text`. The model then sees tens of thousands
of tokens of noise, costs ~$0.003 per such story, and (correctly, under the
new prompt) falls back to the title.

## Fix

Add `gzip` to the reqwest features in `Cargo.toml`:

```toml
reqwest = { version = "0.13", features = [
    "rustls",
    "json",
    "gzip",
], default-features = false }
```

reqwest then sends `Accept-Encoding: gzip` and transparently decodes gzip
bodies in `.text()`. Only gzip is enabled: it is the encoding both offending
servers force, and servers that honour `Accept-Encoding` will now pick gzip
too. `brotli` / `zstd` are not added because nothing observed needs them.

## Verification

`cargo build --release` then one `--reset --no-post --export-text` run on the
same day's stories. Pass: `tmp/50073879.txt` and `tmp/50086363.txt` are plain
text, the replacement-character scan finds 0 garbage pages, and the Gemini
story's prompt drops from 33k tokens to the size of its real article.

## Results (2026-10-15, 10:16 UTC)

One `--reset --no-post --export-text` run on the same 58 stories, zero errors
or retries.

| | Before (10:11 run) | After |
|---|---:|---:|
| Garbage pages (>1% U+FFFD) | 2 | 0 |
| `tmp/50073879.txt` (Gemini) | 93 KB binary | 31 KB text, "Explore our next generation AI systems…" |
| `tmp/50086363.txt` (dfarq) | 138 KB binary | 9.6 KB text |
| Gemini prompt tokens | 33,168 | 7,102 |
| Run prompt tokens | 309,283 | 238,091 |
| Run cost | $0.0373 | $0.0302 |

The Gemini story went from a title-only summary to a grounded one: every
specific it names (Chromium refactor, quantum chemistry, enzymes, 1-million-token
output window, CWE-Bench, AI Pro/Ultra, prompt injection) appears in the
decoded page. It was rated High and exported.

Decoding the two pages also cut the run's prompt tokens by 23%, since the
compressed bytes tokenised far worse than the text they encoded.
