# Make the justfile committable by sourcing everything from `.env`

**Status**: Implemented

## Motivation

`justfile` is in `.gitignore` for one reason: the `deploy` recipe hard-codes the
production host (`root@188.166.55.34`). Everything else in it is already generic.
The `build-image` recipe also hand-parses `.env` with a `grep | cut | tr` pipeline per
variable, which is fragile (the `GOOGLE_CHAT_WEBHOOK_URL` line needs `cut -f 2,3,4`
because the value contains `=`) and duplicates what `just` can do natively.

## Approach

`just` has `set dotenv-load`, which exports every key in `.env` into the environment of
each recipe. With that, recipes reference `$OPENAI_API_KEY` etc. directly and the
deploy host becomes one more `.env` key, `DEPLOY_HOST`. The justfile then contains no
secrets and no machine-specific values, so it can be committed.

## Changes

1. `justfile`
   - Add `set dotenv-load` at the top.
   - `build-image`: pass `--build-arg=NAME="$NAME"` for each Dockerfile ARG.
     `CRON_SCHEDULE` is not in `.env` today (and the Dockerfile only echoes it), so use
     `${CRON_SCHEDULE:-}` to keep it optional under just's default `sh -cu` shell.
   - `deploy`: replace the literal host with `$DEPLOY_HOST`.
2. `.env.example`: add `DEPLOY_HOST="root@<YOUR_DROPLET_IP>"` under a `# Deploy` heading.
3. `.env` (local, untracked): add `DEPLOY_HOST=root@188.166.55.34`.
4. `.gitignore`: remove the `justfile` line.

## Verification

- `just --dry-run build-image` / `just --dry-run deploy` print the expanded recipes.
- `just --evaluate` is not applicable (no just variables); instead run a throwaway
  recipe via `just --justfile` that echoes `$DEPLOY_HOST` and `$OPENAI_MODEL` to confirm
  `dotenv-load` picks up `.env`.
- `git check-ignore justfile` exits non-zero after the `.gitignore` edit.
