# Redact secrets in startup `Config` debug print

**Status**: Implemented

## Motivation

`main()` logs the full `Config` at startup:

```rust
tracing::info!(
    config =? config::config(),
    args =? args,
    "Starting AI Summarizer"
);
```

`Config` derives `Debug` (`src/config.rs:1`), so the rendered line includes
the full `OPENAI_API_KEY` and `GOOGLE_CHAT_WEBHOOK_URL` — both contain
secrets. The webhook URL alone is enough for anyone with read access to
`./log/ai_summarizer.log*` (or the console, when `LOG_TO_CONSOLE=true`) to
post into the production Google Chat room. Logs are also shipped through
the daily-rolling JSON appender, so the secret persists on disk.

We still want the startup line — it's the single source of truth for
"what config is this run using" and is invaluable for debugging. We just
need enough of each secret to identify *which* key/webhook is in play
without exposing a usable credential.

## Approach

Replace `#[derive(Debug)]` on `Config` with a manual `impl std::fmt::Debug`
that prints every field as-is except for the two secret-bearing ones:

- `api_key` → `sk-proj-…wXyZ` style: first 8 chars + `…` + last 4 chars.
  Eight prefix chars covers the full `sk-proj-` / `sk-svca-` / `sk-admi-`
  disambiguator (including the trailing dash); four suffix chars uniquely
  identifies the key across the small handful we rotate between without
  being usable on its own.
- `google_chat_webhook_url` → strip the query string entirely. The path
  (`…/v1/spaces/<SPACE_ID>/messages`) identifies the room; the `key=` and
  `token=` query params are the credential and get replaced with
  `?<redacted>`.

Both rules are short enough to inline in the `Debug` impl — no helper
function, no new module. Strings shorter than the prefix+suffix budget
fall back to `***` so we never accidentally print the whole secret when
the input is unexpectedly short.

## Changes

### `src/config.rs`

Drop `#[derive(Debug)]` from `Config` and add:

```rust
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let api_key = if self.api_key.len() > 11 {
            format!("{}…{}", &self.api_key[..7], &self.api_key[self.api_key.len() - 4..])
        } else {
            "***".to_string()
        };
        let webhook = match self.google_chat_webhook_url.split_once('?') {
            Some((path, _)) => format!("{path}?<redacted>"),
            None => "***".to_string(),
        };

        f.debug_struct("Config")
            .field("api_key", &api_key)
            .field("model", &self.model)
            .field("reasoning_effort", &self.reasoning_effort)
            .field("system_prompt", &self.system_prompt)
            .field("num_titles_to_request", &self.num_titles_to_request)
            .field("google_chat_webhook_url", &webhook)
            .field("max_number_of_stories_to_present", &self.max_number_of_stories_to_present)
            .field("log_to_console", &self.log_to_console)
            .finish()
    }
}
```

No other call sites need to change — `tracing::info!(config =? …)` in
`src/main.rs:278` picks up the new impl automatically.

### Tests

Add a `#[cfg(test)] mod tests` to `src/config.rs` with two assertions:

1. `format!("{:?}", cfg)` for a `Config` populated with a realistic
   `sk-proj-AAAABBBBCCCCDDDD` key contains `sk-proj-…DDDD` and does **not**
   contain the middle (`AAAABBBBCCCC`).
2. The same call for a webhook
   `https://chat.googleapis.com/v1/spaces/XYZ/messages?key=SECRET&token=ALSO`
   contains `…/spaces/XYZ/messages?<redacted>` and does **not** contain
   `SECRET` or `ALSO`.

Construct the `Config` directly (it's `pub(crate)` with `pub(crate)`
fields) — no env-var setup needed.

## Verification

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- --no-post --log-to-console --num-stories 1
```

The last command's `Starting AI Summarizer` line should show
`api_key: "sk-proj-…XXXX"` and
`google_chat_webhook_url: ".../messages?<redacted>"`, with the rest of the
config fields rendered unchanged.
