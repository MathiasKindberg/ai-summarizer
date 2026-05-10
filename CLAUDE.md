# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CLI tool that fetches the top stories from Hacker News, scores each one for AI relevance and summarizes it via the OpenAI Chat Completions API, then posts the high-impact summaries to a Google Chat room. State (already-processed story IDs) lives in a local SQLite file (`db.sqlite`). Designed to be run from `cron` once per day; see `README.md` for the example crontab.

Single Rust binary. No workspace, no submodules, no frontend.

**Stack:** Rust 2024 edition, nightly toolchain (`rust-toolchain.toml`), Tokio, `reqwest` (rustls), `rusqlite` (bundled), `clap`, `dotenvy`, `serde`/`serde_json`, `schemars` (alpha) for JSON-Schema generation, `tracing` + `tracing-appender`, `backoff`, `html2text`, `regex`, `anyhow`.

## Git Commits

**NEVER commit automatically.** Only commit when explicitly asked. This applies to all skills and subagents.

## Tool Usage

Never use `find`, `grep`, `rg`, `cat`, `head`, or `tail` via Bash. Always use the dedicated Glob, Grep, and Read tools instead.

## Repository Layout

```
src/main.rs         Entry point, CLI parsing, top-level pipeline (get_summary), tracing setup
src/config.rs       LazyLock<Config> reading env vars (.env via dotenvy). The ONLY place env vars are read.
src/hn_api.rs       Fetches top story IDs and per-story metadata from the HN Firebase API
src/scraper.rs      Downloads each article URL, strips HTML to plain text, retries with backoff
src/openai.rs       OpenAI request/response types + structured-output schema (schemars) + enrich_story
src/google_chat.rs  Formats stories into a markdown message and POSTs to the configured webhook
src/db.rs           SQLite open / get_processed_stories / insert_stories
src/backoff.rs      ExponentialBackoff presets shared by retry sites
src/lints.rs        Crate-wide clippy/lint configuration

Cargo.toml          Pinned deps; Rust 2024 edition
rust-toolchain.toml Nightly channel
Dockerfile          Two-stage build (rustlang/rust:nightly → debian:bookworm-slim runtime)
.env.example        Template for required env vars (OPENAI_API_KEY, OPENAI_MODEL, SYSTEM_PROMPT, ...)
.github/workflows/  build-and-test.yml (cargo test + rustfmt) and deploy.yml (push-to-release → ghcr.io)
```

There is one global `static CLIENT: LazyLock<reqwest::Client>` declared in `main.rs` — share it from every module via `crate::CLIENT`. Don't construct ad-hoc `reqwest::Client` instances.

## Build, Run, Test

```bash
cargo build --release    # Production build
cargo run                # Run locally; expects .env in cwd
cargo test               # Run unit tests
cargo clippy             # Lints (see src/lints.rs for the configured set)
cargo fmt                # Format
```

Provide configuration either via a `.env` in the working directory (loaded by `dotenvy` at startup) or by injecting env vars directly. See `.env.example` for the full list.

## Boy Scout Rule

**Never leave the codebase in a broken state.** If `cargo check` / `cargo clippy` / `cargo test` fails before your change, fix the pre-existing errors first (or at minimum confirm they're unrelated and flag them to the user). If it fails after your change, fix it before moving on. Don't declare work done while the build is red.

## Coding Philosophy

**Write succinct, high-quality code that fails fast.**

**NEVER change model names, API versions, endpoints, or similar configuration defaults without explicit user approval.** These values are intentional choices — the OpenAI model in `.env.example` (`o3-mini-2025-01-31`), the OpenAI endpoint URL, the HN API URLs, and similar pinned values must not be "fixed" by changing them. If something fails with a "not found" or "unsupported" error, report the issue and ask how to proceed.

- **No boilerplate** — no unnecessary abstractions, wrappers, or helper functions
- **No defensive fallbacks** — trust the APIs and let errors bubble up via `anyhow::Result`. Don't catch errors just to log and re-raise
- **No extra features** — implement exactly what's needed. No "nice-to-have" additions
- **Fail fast** — let the code crash immediately on errors rather than masking with default values or silent fallbacks
- **High quality** — every line should have a clear purpose; delete code that doesn't directly serve the task
- **Default to no comments** — only add one when the WHY is non-obvious (e.g. the `format` workaround in `schema_for_summarizer_response`). Don't restate WHAT the code does

## Rust Conventions

This project already follows these patterns. Match them in new code.

### Fully Qualified Paths Over `use`

Use fully qualified paths at call sites instead of top-of-file `use` imports. This makes dependencies explicit at the point of use and matches the rest of the codebase.

```rust
// BAD
use reqwest::header::USER_AGENT;
use serde::Serialize;

#[derive(Serialize)]
struct Foo { ... }

let req = client.post(url).header(USER_AGENT, "x");

// GOOD
#[derive(serde::Serialize)]
struct Foo { ... }

let req = client.post(url).header(reqwest::header::USER_AGENT, "x");
```

Exception: trait imports needed for method resolution can use a local `use` inside a function (see `main()` for `tracing_subscriber::layer::{Layer, SubscriberExt}` and `clap::Parser`).

### `crate::` Over `super::`

Always use `crate::`-rooted paths for intra-crate references. Never `super::`. `crate::` is absolute, always correct, and doesn't break when modules move.

### `pub(crate)` By Default

Module items are `pub(crate)` unless they truly need to be public — this is a binary, nothing is consumed externally.

### Typed Structs Over `serde_json::json!`

**NEVER use `serde_json::json!` or `serde_json::Value` to construct outgoing data.** Define a typed struct with `#[derive(serde::Serialize)]` (see `OpenAIChatCompletionQuery`, `google_chat::Message`, etc.) and use it. Untyped JSON construction defeats the type system — typos and missing fields surface only at runtime.

### Enums Over Bools

When a `bool` parameter or return value's meaning isn't obvious at the call site, define a two-variant enum instead. Bools force readers to check the function signature; enums are self-documenting. Exceptions: well-known boolean predicates like `is_empty()` / `contains()`, and CLI flags like `--reset` / `--export-text`.

### Integer Types

Use `i64` / `u64` instead of `i32` / `u32` unless an external library forces a specific width. The `Story` struct already uses `i64` throughout.

### No Custom Macros

**NEVER write `macro_rules!` or proc macros.** Use generics, traits, functions, and derive macros from existing crates. Custom macros are hard to read and rarely worth the abstraction.

### Errors

Use `anyhow::Result<T>` for fallible functions. Propagate with `?`. Don't introduce thiserror enums or custom error types unless a caller actually needs to branch on the variant.

### Tracing

Use `tracing::info!` / `warn!` / `error!` with structured fields, not `format!`-style messages. Match the existing style:

```rust
tracing::info!(
    title = story.title,
    url = url,
    ai_score =? story.ai_impact_score,
    "Scored and summarized story"
);
```

`field =? value` for `Debug`, `field = value` for `Display`. Logs are JSON-serialized to a daily-rolling file in `./log/`; the optional pretty console layer is enabled by `LOG_TO_CONSOLE=true` or `--log-to-console`.

## Configuration

`src/config.rs::Config` is the single source of truth for runtime settings. All `std::env::var` reads happen there, exposed via `crate::config::config()` returning a `&'static Config`.

**NEVER read env vars from other modules.** Add a field to `Config` instead.

`OPENAI_API_KEY`, `OPENAI_MODEL`, `SYSTEM_PROMPT`, `GOOGLE_CHAT_WEBHOOK_URL`, and `LOG_TO_CONSOLE` are required (panic at startup if missing). `NUM_TITLES_TO_REQUEST` and `MAX_NUMBER_OF_STORIES_TO_PRESENT` have defaults.

## LLM Prompting

Structured output is requested via `response_format: { type: "json_schema", json_schema: ... }`, with the schema generated from `SummaryResponse` by `schemars`. The schema's field names and `#[schemars(description = "...")]` attributes already tell the model what to produce — keep `SYSTEM_PROMPT` short and complementary. Don't restate field semantics in the system prompt.

The `RecursiveTransform` that strips `format` keys in `schema_for_summarizer_response` is a workaround for OpenAI's structured-output validator rejecting unknown formats; don't remove it.

## Documentation Lookup

**NEVER use `cargo doc`.** It builds documentation locally for all dependencies, which is slow and wasteful. Search the web (docs.rs, official crate docs) for Rust API documentation.

## Testing

Never mock. Never use `#[ignore]`. Tests live next to the code they exercise (`#[cfg(test)] mod tests`) — see `main.rs::tests::test_sort_stories` for the pattern.

## Verify Copies with Diff

When splitting, moving, or copying code between files, always diff the result against the original to verify it's a faithful copy. Save the original to `/tmp/` before deletion, concatenate the new files, and run `diff` to confirm only expected mechanical changes (visibility, path adjustments). Never trust that a copy is clean without verifying.

## Plans

**ALL implementation work MUST have a corresponding plan document in `plans/` BEFORE any code is written.** No feature, refactor, or non-trivial change should be implemented without a plan.

**CRITICAL: Always write plans to the project `plans/` directory.** The `.claude/plans/` file used by plan mode is ephemeral — it is NOT persisted across sessions and is NOT sufficient. Every plan MUST be written to `plans/` using the Write or Edit tool, no exceptions. If you are in plan mode, write the plan to `plans/` first, then use ExitPlanMode. Never rely on the ephemeral plan-mode file as the only copy of a plan.

**In plan mode, you are allowed to create and edit files in `plans/`.** The user will approve file writes through the normal tool permission prompts.

**Every plan document MUST include a status field.** Add a line like `**Status**: Planned` near the top, update it as work progresses, and set it to `**Status**: Implemented` when the plan is done.

Plan documents MUST have filenames in the format `YYYY-MM-DD-NNN-description.md`, where `NNN` is a zero-padded sequence number (001–999). To determine the next number, find the highest `NNN` for the current date among existing plans and increment by one. If no plans exist for that date, start at `001`.

```
plans/2026-05-10-001-cache-hn-firebase-responses.md   # First plan on 2026-05-10
plans/2026-05-10-002-google-chat-card-formatting.md   # Second plan on 2026-05-10
plans/2026-05-11-001-new-feature.md                   # First plan on a new day
```

**Directory organization:** Current month's plans live at the `plans/` root level. At the start of each new month, move the previous month's plans into a subdirectory named `YYYY-MM` (e.g., `plans/2026-04/`). Only past months get subdirectories.

```
plans/
  2026-03/                                            # Past month — subdirectory
  2026-04/                                            # Past month — subdirectory
  2026-05-10-001-cache-hn-firebase-responses.md       # Current month — root level
```

**Plan body:** lead with the title, then `**Status**: ...` near the top, then the substance. For bug-fix plans, walk through Symptoms → Root cause → Fix with concrete evidence (real story IDs, real log lines, real numbers, real code snippets). For feature plans, walk through Motivation → Approach → Step-by-step changes, naming the specific files and functions you'll touch. Plans are working documents — be specific enough that another reader (or future-you) can verify the reasoning, not just the conclusion.

If asked to implement something and no plan exists yet, create one in `plans/` first and get approval before proceeding.

## Deployment

- `.github/workflows/build-and-test.yml` runs `cargo test --all-features` and `cargo fmt --check` on every push and PR.
- `.github/workflows/deploy.yml` builds the Docker image and pushes it to `ghcr.io/<repo>` on push to the `release` branch (with build-provenance attestation).
- Production runs the binary directly under `cron` on a Droplet (see `README.md`). The Dockerfile exists for the registry image but is not the only deploy target — be aware that env defaults baked into the Dockerfile (`NUM_TITLES_TO_REQUEST`, `MAX_NUMBER_OF_STORIES_TO_PRESENT`) only apply to the container path.
