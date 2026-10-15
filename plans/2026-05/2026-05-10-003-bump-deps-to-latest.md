# Bump all `Cargo.toml` dependencies to latest

**Status**: Implemented

## Motivation

`Cargo.lock` is months behind. `schemars` is still on the 1.0 alpha line.
`reqwest` 0.13 is out, `rusqlite` has moved 6 minor versions, `html2text` 3.
Bring everything to current.

The sister production service at `~/ai-educational-videos` already runs the
target versions for the libraries we share — that's the cross-validation
that this combination compiles and works under load.

## Version inventory

| Crate                  | Locked now      | Target    |
|------------------------|-----------------|-----------|
| `anyhow`               | 1.0.95          | 1.0.102   |
| `backon`               | 1.6.0           | 1.6.0     |
| `clap`                 | 4.5.29          | 4.6.1     |
| `dotenvy`              | 0.15.7          | 0.15.7    |
| `html2text`            | 0.14.0          | 0.17.1    |
| `regex`                | 1.11.1          | 1.12.3    |
| `reqwest`              | 0.12.12         | 0.13.3    |
| `rusqlite`             | 0.33.0          | 0.39.0    |
| `schemars`             | 1.0.0-alpha.17  | 1.2.1     |
| `serde`                | 1.0.217         | 1.0.228   |
| `serde_json`           | 1.0.138         | 1.0.149   |
| `tokio`                | 1.43.0          | 1.52.3    |
| `tracing`              | 0.1.41          | 0.1.44    |
| `tracing-appender`     | 0.2.3           | 0.2.5     |
| `tracing-subscriber`   | 0.3.19          | 0.3.23    |

Most move via `cargo update`. Four need a `Cargo.toml` edit: `html2text`,
`reqwest`, `rusqlite`, `schemars`.

## Changes

### `Cargo.toml`

```diff
-html2text = "0.14"
+html2text = "0.17"

-reqwest = { version = "0.12", features = [
-    "rustls-tls",
-    "json",
-], default-features = false }
+reqwest = { version = "0.13", features = [
+    "rustls",
+    "json",
+], default-features = false }

-rusqlite = { version = "0.33", features = ["bundled"] }
+rusqlite = { version = "0.39", features = ["bundled"] }

-schemars = "1.0.0-alpha.17"
+schemars = "1"
```

The `reqwest` feature `rustls-tls` was renamed to `rustls` in 0.13. The new
`rustls` feature uses `aws-lc-rs` as the crypto provider, which needs `cmake`
at build time — added to the Dockerfile below. This matches what
`~/ai-educational-videos` runs.

### `Dockerfile` (builder stage)

Insert after `WORKDIR /ai-summarizer`, before `COPY . .`:

```dockerfile
RUN apt-get update \
 && apt-get install -y --no-install-recommends cmake \
 && rm -rf /var/lib/apt/lists/*
```

### Lockfile

```bash
cargo update
```

### Source code

No source changes. Every call site we use survives unchanged across these
bumps:

- `reqwest`: `Client::new`, `.post`/`.get`, `.header(reqwest::header::USER_AGENT, _)`,
  `.bearer_auth`, `.json`, `.send`, `.error_for_status_ref`, `.json::<T>()`.
- `schemars`: `#[derive(JsonSchema)]`, `#[schemars(required)]`,
  `#[schemars(description = ...)]`, `schemars::generate::SchemaSettings::default()`,
  `with_transform`, `transform::RecursiveTransform`, `Schema::remove`,
  `into_generator`, `into_root_schema_for::<T>()`. The `format`-stripping
  workaround in `src/openai.rs:189` is still required.
- `rusqlite`: `Connection::open`, `execute`, `prepare`, `query_map`, `Row::get`,
  `bundled`. Bundled SQLite jumps 3.48 → 3.51.3.
- `html2text`: `config::plain().raw_mode(true).string_from_read(_, 80)?`.
- `clap`, `backon`, `tokio`, `tracing*`, `serde*`, `regex`, `anyhow`, `dotenvy`:
  patch/minor bumps within existing semver range, no API change.

## Verification

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
docker build .
```

Then with a populated `.env`:

```bash
cargo run -- --log-to-console
```

Confirm the run reaches `Sent message to google chat` or
`No stories to send to google chat`. This exercises HTTP, SQLite, HTML
parsing, and OpenAI structured-output schema generation — the last is the
one surface that only fails at request time.

## Out of scope

- Models, endpoints, schemas, system prompts, env-var defaults — unchanged
  per `CLAUDE.md`.
- Reference-project features we don't use (`reqwest` `multipart`/`stream`,
  `tracing-subscriber` `env-filter`).
- Tightening the auto-resolving version pins (`tokio = "1"` etc.).
