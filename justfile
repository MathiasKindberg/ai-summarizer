set dotenv-load

build-image:
    podman build -t ai-summarizer \
        --build-arg=CRON_SCHEDULE="${CRON_SCHEDULE:-}" \
        --build-arg=OPENAI_API_KEY="$OPENAI_API_KEY" \
        --build-arg=OPENAI_MODEL="$OPENAI_MODEL" \
        --build-arg=GOOGLE_CHAT_WEBHOOK_URL="$GOOGLE_CHAT_WEBHOOK_URL" \
        --build-arg=SYSTEM_PROMPT="$SYSTEM_PROMPT" \
        .

run-image:
    podman run -it --rm --name ai-summarizer ai-summarizer

deploy:
    cargo build --release
    scp target/release/ai-summarizer "$DEPLOY_HOST":~/ai-summarizer/ai-summarizer
    scp .env.prod "$DEPLOY_HOST":~/ai-summarizer/.env
