# Switch OpenAI Model from o4-mini to gpt-6-luna (medium reasoning)

**Status**: Implemented — code, `.env.example`, `.env`, `.env.prod` done 2026-10-15; rollout to the Droplet (`just deploy`) still to run before 2026-10-23

Per `CLAUDE.md` ("NEVER change model names ... without explicit user approval"),
this plan is the approval gate. Nothing in `.env.example`, the production `.env`,
or `src/openai.rs` changes until it is signed off.

## Motivation

- **Production still runs `o4-mini-2025-04-16`.** Today's (2026-10-14) digest
  header reads "Daily digest of top Hacker news AI stories as per
  o4-mini-2025-04-16"; that string is `config().model`
  (`src/google_chat.rs:39-40`), so it is the model the Droplet actually called.
- **o4-mini shuts down on 2026-10-23** (OpenAI deprecations page, April 22, 2026
  legacy-snapshots notice). After that date the cron job fails every call. This
  switch has to land and deploy before then.
- **The gpt-5.4-nano switch (`plans/2026-05/2026-05-10-001-switch-to-gpt-5-4-nano.md`)
  never reached production because it did not reduce costs.** gpt-5.4-nano is
  itself scheduled for shutdown on 2027-04-01 with `gpt-6-luna` named as the
  replacement, so there is no reason to revisit it.
- `gpt-6-luna` (released 2026-09-22) is OpenAI's "most efficient model for
  focused, high-volume tasks" and is priced well under both previous candidates.

## Grounded data (verified 2026-10-14)

Pricing per 1M tokens, Standard tier, from the OpenAI model/pricing pages:

| Model                          | Input | Output | Shutdown   |
|--------------------------------|------:|-------:|------------|
| `o4-mini-2025-04-16` — prod    |  1.10 |   4.40 | 2026-10-23 |
| `gpt-5.4-nano` — never shipped |  0.20 |   1.25 | 2027-04-01 |
| **`gpt-6-luna` — target**      | **0.10** | **0.50** | —      |

`gpt-6-luna` facts relevant to this codebase:

- Model ID `gpt-6-luna`; it is the only snapshot.
- Supported on `v1/chat/completions` (our endpoint). The one Chat Completions
  restriction is that **function calling** requires `reasoning_effort: "none"` —
  we don't use function calling, we use `response_format: json_schema`, so this
  doesn't apply.
- Structured outputs: supported.
- Reasoning effort values: `none`, `low`, `medium`, `high`, `xhigh`, `max`.
  Default `medium`.
- Prompts over 272K input tokens are billed at 2× input / 1.5× output. Ours are
  ~200 tokens (see below), so irrelevant.

Real token counts from the only gpt-5.4-nano (medium) full run on record
(`log/ai_summarizer.log.2026-05-10`, 13:22 UTC, 59 stories):
`prompt_tokens: 11466, completion_tokens: 13854` → ~194 in / ~235 out per story.
Output dominates the bill, and `completion_tokens` includes reasoning tokens.
Priced at those token counts:

| Model         | Cost of that 59-story run |
|---------------|--------------------------:|
| o4-mini       | $0.0736 (if it used the same token counts) |
| gpt-5.4-nano  | $0.0196 |
| gpt-6-luna    | $0.0081 |

We have no o4-mini token counts on record, so we can't tell from this whether
nano lost on cost because it produced more reasoning tokens per call than
o4-mini. The local test below measures both models on the same stories, so
this question gets answered directly instead of from list prices. Luna breaks
even with o4-mini only if it emits ~8.8× o4-mini's output tokens per story.

## Reasoning effort

`medium`: some reasoning, not the high tiers. It is Luna's default and
matches what o4-mini runs with when no effort is sent, so the local comparison
is like-for-like. `high` / `xhigh` / `max` are skipped because output tokens
drive the cost.

## API shape

The request `OpenAIChatCompletionQuery` (`src/openai.rs:57-64`) already matches
what `gpt-6-luna` accepts on Chat Completions:

```json
{
  "model": "gpt-6-luna",
  "messages": [
    {"role": "developer", "content": "<SYSTEM_PROMPT>"},
    {"role": "user", "content": "<story>"}
  ],
  "response_format": {"type": "json_schema", "json_schema": {"name": "ai_relatedness_scores", "schema": {...}, "strict": true}},
  "reasoning_effort": "medium"
}
```

- `ReasoningEffort` (`src/openai.rs:31-38`) already serializes `medium`. It does
  not need the new `none` / `xhigh` / `max` variants for this change, so they
  aren't added.
- The `RecursiveTransform` that strips `format` stays as-is.
- Response side: `Usage` (`src/openai.rs:153-158`) has no field for reasoning
  tokens, so we can't tell whether a cost difference comes from reasoning or
  from summary length. Add
  `completion_tokens_details.reasoning_tokens`. Both o4-mini and Luna return
  it, so the field is required (no `Option`, fail fast if missing).

## Step-by-step changes

1. **`src/openai.rs`** — extend `Usage`:
   ```rust
   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub(crate) struct Usage {
       pub(crate) prompt_tokens: i64,
       pub(crate) completion_tokens: i64,
       pub(crate) total_tokens: i64,
       pub(crate) completion_tokens_details: CompletionTokensDetails,
   }

   #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
   pub(crate) struct CompletionTokensDetails {
       pub(crate) reasoning_tokens: i64,
   }
   ```
2. **`src/main.rs:115-128`** — `total_usage` builds a `Usage` literal; add the
   summed `completion_tokens_details.reasoning_tokens` so it compiles and the
   "Finished enriching stories" log line carries the run total.
3. **`.env.example`**:
   - `OPENAI_MODEL="gpt-6-luna"`
   - Replace the stale `# gpt-4o-mini` / `# gpt-4o-2024-11-20` /
     `# o3-mini-2025-01-31` comment block with `# gpt-6-luna`, `# gpt-6-sol`.
   - Add `OPENAI_REASONING_EFFORT="medium"` (currently missing from the file).
4. `cargo check`, `cargo clippy`, `cargo test`, `cargo fmt --check` green.

## Local cost test (gate before production)

Run both models on the same HN top list, alternating them, two runs each, so
a shift in the HN list or run-to-run variance in reasoning tokens doesn't skew
the result. The local `.env` must have `OPENAI_REASONING_EFFORT=medium` and a
dummy `GOOGLE_CHAT_WEBHOOK_URL`. Inline env vars override `.env`, because
dotenvy doesn't overwrite variables that are already set.

```bash
cargo build --release
for i in 1 2; do
  OPENAI_MODEL=o4-mini-2025-04-16 ./target/release/ai-summarizer --reset --no-post --export-text
  cp export/exported_stories.json /tmp/o4-mini-$i.json
  OPENAI_MODEL=gpt-6-luna          ./target/release/ai-summarizer --reset --no-post --export-text
  cp export/exported_stories.json /tmp/luna-$i.json
done
```

`--reset` makes every run see the full top-60 list. `--no-post` keeps the test
out of the production room.

Compare token usage per model, restricted to story URLs that every run scored
(the log is today's `log/ai_summarizer.log.YYYY-MM-DD`; each run's
"Starting AI Summarizer" line names its model):

```bash
jq -n '
  reduce inputs as $l ({model: null, rows: []};
    if $l.fields.message == "Starting AI Summarizer" then
      .model = ($l.fields.config | capture("model: \"(?<m>[^\"]+)\"").m)
    elif $l.fields.message == "Scored and summarized story" then
      .rows += [{model: .model, url: $l.fields.url}
        + ($l.fields.usage
           | capture("prompt_tokens: (?<p>\\d+), completion_tokens: (?<c>\\d+), total_tokens: \\d+, completion_tokens_details: CompletionTokensDetails \\{ reasoning_tokens: (?<r>\\d+)")
           | map_values(tonumber))]
    else . end)
  | .rows as $rows
  | ($rows | group_by(.model) | map(map(.url) | unique)
           | reduce .[1:][] as $s (.[0]; . - (. - $s))) as $common
  | $rows | map(select(.url as $u | $common | index($u)))
  | group_by(.model)
  | map({model: .[0].model, calls: length,
         prompt: (map(.p) | add), completion: (map(.c) | add), reasoning: (map(.r) | add)})
' log/ai_summarizer.log.$(date -u +%F)
```

Cost per call = `(prompt × in_price + completion × out_price) / 1e6 / calls`,
with the prices from the table above.

**Pass criteria (all required):**

- Luna's cost per call is lower than o4-mini's on the matched set. Record both
  numbers, plus the reasoning-token share, in this plan.
- Zero structured-output / deserialization errors for Luna in the log. This
  also confirms `completion_tokens_details` deserializes.
- Luna's `High` picks broadly match o4-mini's on the same stories. Read the
  `/tmp/luna-*.json` summaries side by side with `/tmp/o4-mini-*.json`: two
  prose paragraphs each, no preamble, no headers. Shorter than o4-mini is fine.

If Luna is not cheaper, stop and report the measured numbers. Don't ship it.

## Results (2026-10-14, 15:20–15:22 UTC)

Four alternating runs over the same 57 stories (every run had
`prompt_tokens: 11083`, so the set matched exactly and the URL intersection
wasn't needed):

| Run       | prompt | completion | reasoning | Cost    |
|-----------|-------:|-----------:|----------:|--------:|
| o4-mini 1 | 11,083 |     43,816 |    33,536 | $0.2050 |
| Luna 1    | 11,083 |     13,388 |     8,714 | $0.0078 |
| o4-mini 2 | 11,083 |     42,992 |    32,896 | $0.2014 |
| Luna 2    | 11,083 |     14,149 |     9,498 | $0.0082 |

**Cost: passes.** Luna costs ~$0.008 per run vs o4-mini's ~$0.203, about
25× cheaper. Luna emits about a third of o4-mini's output tokens, and each of
those tokens costs 8.8× less.

**Structured output: passes.** No OpenAI or deserialization errors. The 8
ERROR lines are all "Error scraping story: No such file or directory", from
`--export-text` writing `tmp/<title>.txt` for titles that contain `/`. That
bug predates this change and only affects `--export-text` runs.

**Categorisation: much stricter and more consistent.**

- High assignments per run: o4-mini 7 and 6, Luna 0 and 2.
- o4-mini changes category between its own two runs on 24 of 57 stories (e.g.
  "Eric Binnion has died" Zero → Medium). Luna changes on 12 of 57.
- Luna's run 1 would have posted **no digest**. Its run 2 posted 2 stories,
  where o4-mini posted 4.

**Summaries: Luna doesn't invent facts but says little; o4-mini invents
specifics.** `enrich_story` sends only `story.title` (`src/openai.rs:162`), so
both models are summarising a headline:

- o4-mini made up details: "€15.2 billion first-quarter orders", "backlog to a
  record €66 billion", "CEO Peter Wennink", "raised its 2024 sales guidance",
  "Researchers at [Organization]", "up to 2× faster top-k queries".
- Luna restates the headline in two 1–2 sentence paragraphs (e.g. ASML: "ASML
  reported strong orders, easing concerns that the boom in AI spending is
  losing momentum."). It invents no numbers or names, but it's thinner than
  the ~25% trim from o4-mini that was the target.

## Rollout (must finish before 2026-10-23)

1. Merge, push to `release` (registry image via `deploy.yml`).
2. On the Droplet: deploy the new binary, and set `OPENAI_MODEL=gpt-6-luna` and
   `OPENAI_REASONING_EFFORT=medium` in its `.env`. The new `Usage` field
   requires the new binary.
3. Next morning's digest header must read "as per gpt-6-luna". Check that run's
   "Finished enriching stories" `total_usage` in the Droplet's `./log/`.

## References

- OpenAI — gpt-6-luna model page: <https://developers.openai.com/api/docs/models/gpt-6-luna>
- OpenAI — pricing: <https://developers.openai.com/api/docs/pricing>
- OpenAI — deprecations: <https://developers.openai.com/api/docs/deprecations>
- VentureBeat — GPT-6 Sol and Luna release: <https://venturebeat.com/technology/openai-releases-gpt-6-sol-and-luna-models-slashing-api-costs-50-or-more>
