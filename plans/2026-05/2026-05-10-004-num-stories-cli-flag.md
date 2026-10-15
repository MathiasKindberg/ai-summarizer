# Add `--num-stories` CLI flag

**Status**: Implemented

## Motivation

For local runs (`--no-post`, `--export-text`) you often want to process fewer
than the production default of 60 top stories — both to shorten the loop and
to keep OpenAI cost down. Today the only knob is the `NUM_TITLES_TO_REQUEST`
env var, which means editing `.env` and reverting it. A CLI flag is the
natural override.

## Approach

Add `num_stories: Option<usize>` to `Args`. When `Some(n)`, use `n`;
otherwise fall back to `config().num_titles_to_request`. Plumb the resolved
value into `hn_api::get_hackernews_top_stories` as a parameter so the
function no longer reads `config()` directly — keeps the env-vs-flag
precedence in one place (`get_summary`).

The limit is applied at the top-stories fetch (the upstream of the
pipeline). Truncating there cascades naturally: fewer top stories → fewer
HN item fetches → fewer scrapes → fewer OpenAI calls.

`-n` is already taken by `--no-post` and `-s` reads as "stories" but is
also a common short for "silent"; skip the short flag to avoid future
collisions.

## Changes

### `src/main.rs::Args`

Add a fourth field after `no_post`:

```rust
#[arg(long)]
#[arg(help = "Number of HN top stories to fetch and process (default: NUM_TITLES_TO_REQUEST)")]
num_stories: Option<usize>,
```

### `src/main.rs::get_summary`

Replace:

```rust
let stories = hn_api::get_hackernews_top_stories().await?;
```

with:

```rust
let num_stories = args
    .num_stories
    .unwrap_or(config::config().num_titles_to_request);
let stories = hn_api::get_hackernews_top_stories(num_stories).await?;
```

### `src/hn_api.rs::get_hackernews_top_stories`

Take `num_titles: usize` as a parameter; drop the `config()` read at line 7.

```rust
pub(crate) async fn get_hackernews_top_stories(
    num_titles: usize,
) -> anyhow::Result<Vec<crate::Story>> {
    let response = crate::CLIENT
        .get("https://hacker-news.firebaseio.com/v0/topstories.json")
        .send()
        .await?;
    let stories: Vec<i64> = response.json::<Vec<i64>>().await?[..num_titles].to_vec();
    ...
}
```

## Verification

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- --num-stories 3 --no-post --log-to-console
```

The last command should log `num_stories = 3` in the "Got top stories" line
and complete the pipeline against three articles.
