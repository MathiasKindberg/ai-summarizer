# Migrate from `backoff` crate to `backon`

**Status**: Planned

## Motivation

The `backoff` crate (`ihrwein/backoff`, v0.4.0) has been effectively unmaintained
since April 2022. `backon` (`Xuanwo/backon`) is actively maintained, has reached
a stable v1 API with a no-breakage policy, and exposes a cleaner trait-based
ergonomics — `your_fn.retry(builder).await` — that fits the rest of the
codebase better.

Our usage is small and contained:

- one dependency line in `Cargo.toml`
- one helper module (`src/backoff.rs`) with two preset functions, only one of
  which is actually called
- one call site in `src/scraper.rs`

So this is a low-risk swap that removes an unmaintained dep.

## Current state (verified)

`Cargo.toml:8`

```toml
backoff = { version = "0.4", features = ["tokio"] }
```

`src/backoff.rs` (entire file):

```rust
const MAX_INTERVAL_BETWEEN_TRIES: std::time::Duration = std::time::Duration::from_secs(5);

#[allow(unused)]
pub(crate) fn backoff_infinite() -> backoff::ExponentialBackoff {
    backoff::ExponentialBackoffBuilder::new()
        .with_max_interval(MAX_INTERVAL_BETWEEN_TRIES)
        .with_max_elapsed_time(None)
        .build()
}

pub(crate) fn backoff_default() -> backoff::ExponentialBackoff {
    backoff::ExponentialBackoffBuilder::new()
        .with_max_interval(MAX_INTERVAL_BETWEEN_TRIES)
        .with_max_elapsed_time(Some(std::time::Duration::from_secs(30)))
        .build()
}
```

`src/scraper.rs:40-60`:

```rust
let trimmed_text = backoff::future::retry_notify(
    crate::backoff::backoff_default(),
    || async {
        Ok(tokio::select! {
            res = scrape_and_trim_text(&story, export_text) => res,
            _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                Err(anyhow::anyhow!("Timeout when scraping story"))},
        }?)
    },
    |e, duration: std::time::Duration| {
        tracing::warn!(
            error =? e,
            error_at =? duration.as_secs(),
            title = title,
            id = id,
            url = url,
            "Error when scraping story, retrying"
        )
    },
)
.await?;
```

`src/main.rs:3` declares `mod backoff;`. The local module name happens to
collide with the crate name — `backon` removes that collision incidentally.

## API mapping

| `backoff` 0.4                              | `backon` 1.x                                                                   |
|--------------------------------------------|--------------------------------------------------------------------------------|
| `ExponentialBackoffBuilder::new().build()` | `ExponentialBuilder::default()`                                                |
| `.with_max_interval(d)`                    | `.with_max_delay(d)`                                                           |
| `.with_max_elapsed_time(Some(d))`          | no direct equivalent — use `.with_max_times(n)` (attempt count) instead        |
| `.with_max_elapsed_time(None)`             | `.without_max_times()` (or just don't call `with_max_times`)                   |
| `backoff::future::retry_notify(b, op, n)`  | `op.retry(b).sleep(tokio::time::sleep).notify(n).await` (`Retryable` trait)    |
| op closure returns `Result<T, backoff::Error<E>>` | op closure returns plain `Result<T, E>`                                 |

Notes:

- `backon` 1.x default features include `tokio-sleep`, so `backon = "1"` (no
  feature list) is enough. The `.sleep(tokio::time::sleep)` call is still
  required because `Retryable` is sleeper-agnostic at the type level.
- `backon` does not expose `with_max_elapsed_time`. The existing
  `backoff_default()` was a soft 30 s wall-clock cap; we approximate with a
  bounded attempt count. With defaults (factor 2, min 1 s) and our
  `with_max_delay(5 s)`, the sleep schedule is roughly 1 s → 2 s → 4 s → 5 s
  → 5 s. Picking `with_max_times(4)` gives 4 retries after the initial
  attempt, totalling ≈ 12 s of inter-attempt sleep, well under the previous
  30 s cap. The per-attempt 30 s timeout in the closure is unchanged.
- Behaviour change is acceptable: in practice `backoff_default()` rarely got
  near 30 s elapsed because the per-attempt timeout is 30 s itself, so the
  retry budget was essentially "one extra try". `with_max_times(4)` is no less
  generous than that.
- `op.retry(...)` requires the `Retryable` trait in scope. Per the project
  convention (`crate::` paths, no top-level `use`), import it locally in the
  function — same exception as `clap::Parser` and `tracing_subscriber` traits
  in `main()`.

## Step-by-step changes

### 1. `Cargo.toml`

- Remove: `backoff = { version = "0.4", features = ["tokio"] }`
- Add: `backon = "1"`

(Default features pull in `tokio-sleep`; no extra feature list needed.)

### 2. `src/backoff.rs`

Replace the file contents with:

```rust
const MAX_DELAY_BETWEEN_TRIES: std::time::Duration = std::time::Duration::from_secs(5);

pub(crate) fn backoff_default() -> backon::ExponentialBuilder {
    backon::ExponentialBuilder::default()
        .with_max_delay(MAX_DELAY_BETWEEN_TRIES)
        .with_max_times(4)
}
```

Specifically:

- Delete `backoff_infinite()` — currently `#[allow(unused)]`, so it's dead
  code by the project's "delete code that doesn't directly serve the task"
  rule.
- Keep the function name `backoff_default()` and the module name
  `src/backoff.rs`. The module is named after the *concept* (a backoff
  preset), not the *crate*; renaming would be churn. The crate name
  collision that previously forced `crate::backoff::backoff_default()`
  disambiguation no longer exists, but the existing call site already uses
  the qualified path so nothing changes.
- Rename the constant `MAX_INTERVAL_BETWEEN_TRIES` → `MAX_DELAY_BETWEEN_TRIES`
  to match `backon`'s vocabulary (`with_max_delay`).

### 3. `src/scraper.rs`

Rewrite the retry block. Drop the `Ok(...)?` wrapping that `backoff` required.

```rust
use backon::Retryable;

let trimmed_text = (|| async {
    tokio::select! {
        res = scrape_and_trim_text(&story, export_text) => res,
        _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
            Err(anyhow::anyhow!("Timeout when scraping story"))
        }
    }
})
.retry(crate::backoff::backoff_default())
.sleep(tokio::time::sleep)
.notify(|e: &anyhow::Error, duration: std::time::Duration| {
    tracing::warn!(
        error =? e,
        error_at =? duration.as_secs(),
        title = title,
        id = id,
        url = url,
        "Error when scraping story, retrying"
    )
})
.await?;
```

The `use backon::Retryable;` is the trait-method-resolution exception
permitted by `CLAUDE.md`.

### 4. Verification

- `cargo fmt`
- `cargo clippy --all-targets -- -D warnings` (matches CI lint posture)
- `cargo test`
- `cargo build --release` to confirm release compiles too
- Eyeball-check the diff in `src/scraper.rs` — confirm the closure body
  matches the original (minus the `Ok(...)?` wrap) per the
  "Verify Copies with Diff" rule in `CLAUDE.md`.

## Out of scope

- Renaming `backoff_default` to `default()` or moving it out of a dedicated
  module. The current shape works and matches existing call sites.
- Adding `with_jitter`. The original config didn't use it; we're matching
  behaviour, not improving it.
- Touching the per-attempt 30 s timeout in the scraper closure. It's
  orthogonal to the crate swap.

## Risk / rollback

- Single dep, one call site, no public API. Rollback = revert the commit.
- The behaviour delta (fixed attempt count vs. soft elapsed-time cap) is the
  only semantic difference and is documented above. If it ever bites, switch
  to `with_max_times(N)` with a different N or wrap the whole `.retry().await`
  in `tokio::time::timeout(Duration::from_secs(30), …)`.
