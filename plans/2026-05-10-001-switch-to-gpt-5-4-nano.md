# Switch OpenAI Model from o4-mini to gpt-5.4-nano (medium reasoning)

**Status**: Planned

Per `CLAUDE.md` ("NEVER change model names ... without explicit user approval"),
this plan exists to get that approval before any code lands. No `.env.example`,
`Config`, or request-struct change happens until the user signs off here.

## Motivation

The daily HN AI digest is a summarization + categorical-scoring task. Switching
to gpt-5.4-nano with medium reasoning effort improves quality (Intelligence
Index 38 vs o4-mini's 33) and cuts cost roughly 4×. Going straight to medium
reasoning rather than starting from non-reasoning — raw non-reasoning models
in 2026 underperform their reasoning-effort siblings by enough that hedging
through a phased ramp isn't worth the operational overhead.

Current state:

- Production `.env` runs `o4-mini-2025-04-16` (confirmed via the model line in
  a recent digest sent to Google Chat).
- `.env.example:9` still pins `o3-mini-2025-01-31`. Drift; fix as part of this
  plan.
- The Chat Completions request body in `src/openai.rs:32-36`
  (`OpenAIChatCompletionQuery`) has fields `model`, `messages`,
  `response_format`. No `reasoning_effort` field today — needs to be added.
- Model is read once at startup via `LazyLock<Config>` in `src/config.rs:13`
  and required (`expect("OPENAI_MODEL not set")` at line 22).

## Grounded data (verified May 2026)

Source: Artificial Analysis Intelligence Index (composite reasoning + knowledge
benchmark). Numbers pulled directly from the per-model pages, not summarized
marketing material.

| Model                                | Intelligence Index | Input $/1M | Output $/1M | Blended (3:1) |
|--------------------------------------|-------------------:|-----------:|------------:|--------------:|
| `o4-mini` (high) — current prod      |                 33 |       1.10 |        4.40 |          1.93 |
| `gpt-5.4-nano` (non-reasoning)       |                 24 |       0.20 |        1.25 |          0.46 |
| **`gpt-5.4-nano` (medium) — target** |             **38** |   **0.20** |    **1.25** |      **0.46** |
| `gpt-5.4-nano` (xhigh reasoning)     |                 44 |       0.20 |        1.25 |          0.46 |
| `gpt-5-nano` (high)                  |                 27 |       0.05 |        0.40 |          0.14 |

Important caveats not visible in the table:

- Per-token price for gpt-5.4-nano is identical across reasoning effort levels,
  but **xhigh emits ~8× the output tokens of medium** during the Intelligence
  Index eval, so its real per-call cost can be 5–10× medium. Skip xhigh for
  this workload.
- `gpt-5.4-nano` non-reasoning at IQ 24 is a quality downgrade from o4-mini's
  33. Rejected.
- `gpt-5-nano` is also a downgrade (27 vs 33). Listed for context only.
- No public summarization-specific benchmark for these models was findable.
  Closest proxy: Label Studio's receipt extraction benchmark
  (`gpt-5-mini` 0.87, `gpt-5-nano` 0.77, `gpt-5` 0.89). Treat the IQ
  numbers above as a directional signal, not a guarantee for our task.
- gpt-5.4-nano (medium) on IQ alone beats o4-mini (high) by 5 points at
  roughly ¼ the blended cost. xhigh widens the IQ gap further but those gains
  show up on Humanity's Last Exam / GPQA / SciCode — not summarization.

## Approach

Single change: `OPENAI_MODEL=gpt-5.4-nano` with `reasoning_effort: "medium"`.
No phased ramp — non-reasoning IQ 24 is below current production and not
worth A/B testing as a hedge. xhigh skipped per the cost/verbosity caveat
above.

## Step-by-step changes

1. **Extend the request struct.** `src/openai.rs:32-36`:
   ```rust
   #[derive(Debug, serde::Serialize)]
   pub(crate) struct OpenAIChatCompletionQuery {
       model: String,
       messages: Vec<Message>,
       response_format: ResponseFormat,
       #[serde(skip_serializing_if = "Option::is_none")]
       reasoning_effort: Option<ReasoningEffort>,
   }
   ```
2. **Define the enum** (per CLAUDE.md "Enums Over Bools" and typed structs
   over `serde_json::json!`):
   ```rust
   #[derive(Debug, Clone, serde::Serialize)]
   #[serde(rename_all = "lowercase")]
   pub(crate) enum ReasoningEffort { Low, Medium, High }
   ```
3. **Update `OpenAIChatCompletionQuery::new`** (`src/openai.rs:39-48`) to
   accept `Option<ReasoningEffort>` and thread it into the new field.
4. **`src/config.rs`** — add `reasoning_effort: Option<ReasoningEffort>` to
   `Config` (currently lines 2-11), parsed from a new `OPENAI_REASONING_EFFORT`
   env var. Match the existing pattern: absent var → `None`; present →
   explicit parse with hard failure on a malformed value. Keep all
   `std::env::var` reads in this file per CLAUDE.md ("NEVER read env vars
   from other modules").
5. **`src/openai.rs:140-147`** — pass
   `crate::config::config().reasoning_effort.clone()` into the constructor.
6. **Add a `--no-post` CLI flag** to suppress the webhook send during local
   verification (and any other dry-run-style use). Extend the `Args` struct
   at `main.rs:16-28`:
   ```rust
   #[arg(short, long, default_value = "false")]
   #[arg(help = "Skip the Google Chat post (for local verification)")]
   no_post: bool,
   ```
   Gate the existing post at `main.rs:216-218`:
   ```rust
   if !args.no_post {
       let message = google_chat::create_message(stories.clone())?;
       google_chat::send_message(message, &config::config().google_chat_webhook_url).await?;
       tracing::info!("Sent message to google chat");
   } else {
       tracing::info!("Skipping google chat post (--no-post)");
   }
   ```
   Caveat: `GOOGLE_CHAT_WEBHOOK_URL` is still required at startup
   (`config.rs:29-30`), so a dummy value (e.g. `GOOGLE_CHAT_WEBHOOK_URL=x`)
   needs to be present in the env even when running with `--no-post`. Not
   worth promoting the field to `Option<String>` for this — keeping `Config`
   strict matches the rest of the file. The DB insert at `main.rs:220` is
   intentionally NOT gated by `--no-post`; pair with `--reset` for repeated
   verification runs (matches the existing flag-composition convention). 
   Update `README.md:17-23` to list the new flag.
7. **`.env.example`** updates:
   - Line 9: `OPENAI_MODEL="gpt-5.4-nano"`
   - Lines 11-13 (the `# gpt-4o-mini` comment block): refresh to list
     `gpt-5.4-nano`, `gpt-5.4-mini`, and keep `o4-mini-2025-04-16` as a
     fallback option.
   - Add: `OPENAI_REASONING_EFFORT="medium"` with a brief comment that valid
     values are `low` / `medium` / `high` and the var is only meaningful for
     reasoning-capable models.
8. **Update production `.env`** on the Droplet to match.
9. **Verify the parameter name at implementation time.** OpenAI's Chat
   Completions API has used `reasoning_effort` for the o-series; confirm
   gpt-5.4 still accepts it on `chat/completions` (vs requiring the
   `responses` API) before merging. If the parameter name has changed,
   stop and surface it — do not guess. Per CLAUDE.md, "model not found" /
   "unsupported parameter" errors must be reported, not silently worked
   around.

## Verification

- `cargo check`, `cargo clippy`, `cargo test`, `cargo fmt --check` all green
  per the Boy Scout rule.
- **Mandatory: actually run the binary against real articles before
  merging — type checks and unit tests do not catch model-style regressions.**
  A green `cargo test` is necessary but not sufficient.
- Real-run smoke test:
  `cargo run --release -- --export-text --reset --no-post --log-to-console`
  against today's HN top list. `--reset` wipes `db.sqlite` so the
  already-processed-IDs filter at `main.rs:174-178` doesn't drop everything;
  `--export-text` writes the final High-impact-only set to
  `export/exported_stories.json` (per `main.rs:209-214`) where the actual
  summary text and per-story `Usage` are visible; `--no-post` (added in
  step 6) skips the webhook entirely so nothing leaks to the production
  channel. `GOOGLE_CHAT_WEBHOOK_URL` still needs a (dummy) value because
  the config loader at `config.rs:29-30` requires it at startup. One run
  is enough; no need for a multi-day soak.
- **Verbosity gate (hard stop).** Read the resulting Google Chat message
  end-to-end. The output must remain a tight two-paragraph-per-story digest,
  not a verbose mess. Specifically check for: paragraphs ballooning past ~5
  sentences, bullet lists where the prompt asked for prose, preamble like
  "Here is the summary:", or model-introduced section headers. Compare
  `completion_tokens` per story to a recent o4-mini run — if gpt-5.4-nano
  (medium) is ≥1.5× the output token count for the same article, treat as a
  regression and tighten the system prompt before flipping production. The
  Google Chat formatter has no length cap, so a chatty model produces a
  visibly worse digest, not a truncated one.
- Are the `Category` (`High`/`Medium`/`Low`/`Zero`) assignments sensible and
  consistent with what o4-mini was producing?
- No malformed JSON / structured-output failures in `./log/`.
- Compare the `Usage` struct fields (`prompt_tokens`, `completion_tokens`,
  `total_tokens`) in the JSON logs across a few runs to confirm the predicted
  cost drop materializes — and to catch verbosity creep that isn't visually
  obvious (output token count is the canary).

## Risks / open questions

- **`reasoning_effort` parameter name on gpt-5.4 Chat Completions.** Has been
  stable for o-series; verify before implementation. If gpt-5.4 only exposes
  reasoning effort via the newer `responses` API, scope expands meaningfully
  and we should reopen this plan.
- **`Role::Developer` compatibility.** `src/openai.rs:69-73` sends the system
  message with `role: "developer"` (the o-series convention). Confirm
  gpt-5.4-nano accepts this on Chat Completions; if it requires `system`
  instead, we need a per-model role mapping. Not expected to be an issue —
  the `developer` role has been the recommended replacement since the o-series
  — but flag it during smoke test.
- **Structured-output quirks.** The `RecursiveTransform` that strips `format`
  keys (`src/openai.rs:187-191`) is a workaround for OpenAI's structured-output
  validator. Confirm it still fires correctly under gpt-5.4-nano; the model
  family may have tightened or relaxed the validator. CLAUDE.md explicitly
  warns: don't remove the workaround.
- **Account/tier availability.** No public confirmation that gpt-5.4-nano is
  available on the user's API tier. If the first request returns a "model not
  found" or "unsupported" error, surface it and ask — don't silently fall
  back per CLAUDE.md guidance.
- **Verbosity regression.** The gpt-5 family is anecdotally chattier than the
  o-series at equivalent prompt budgets, and medium reasoning may amplify
  this. The system prompt in `.env.example:16` is short and complementary to
  the JSON schema (per CLAUDE.md "LLM Prompting" guidance) — if verbosity
  creeps in, the fix is to tighten that prompt with an explicit length
  constraint ("≤ 4 sentences per paragraph", etc.), not to bolt on
  post-processing.

## Rollout

1. Land code + config changes per the step-by-step.
2. Run once locally with `--export-text --reset --no-post`; inspect
   `export/exported_stories.json`. Verbosity gate must pass
   (see Verification).
3. Push to the `release` branch (triggers `.github/workflows/deploy.yml`) and
   update the production `.env` on the Droplet.
4. Watch the next 2-3 cron runs in `./log/`. If structured-output, role, or
   verbosity issues surface, revert by setting `OPENAI_MODEL` back to
   `o4-mini-2025-04-16` and unsetting `OPENAI_REASONING_EFFORT` (the new
   field is `Option`, so absence is safe).

## References

- Artificial Analysis — `gpt-5.4-nano` (medium):
  <https://artificialanalysis.ai/models/gpt-5-4-nano-medium>
- Artificial Analysis — `gpt-5.4-nano` (all variants):
  <https://artificialanalysis.ai/models/gpt-5-4-nano>
- Artificial Analysis — `o4-mini`:
  <https://artificialanalysis.ai/models/o4-mini>
- Artificial Analysis — `gpt-5-nano`:
  <https://artificialanalysis.ai/models/gpt-5-nano>
- DataCamp — GPT-5.4 mini and nano benchmarks:
  <https://www.datacamp.com/blog/gpt-5-4-mini-nano>
- Simon Willison — GPT-5.4 mini and nano:
  <https://simonwillison.net/2026/Mar/17/mini-and-nano/>
- Label Studio — Evaluating the GPT-5 series on custom benchmarks:
  <https://labelstud.io/blog/evaluating-the-gpt-5-series-on-custom-benchmarks/>
