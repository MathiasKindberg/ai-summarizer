FROM docker.io/rustlang/rust:nightly AS builder
ARG CRON_SCHEDULE
ARG OPENAI_API_KEY
ARG OPENAI_MODEL
ARG GOOGLE_CHAT_WEBHOOK_URL
ARG SYSTEM_PROMPT

RUN echo "CRON_SCHEDULE=${CRON_SCHEDULE}"
RUN echo "OPENAI_API_KEY=${OPENAI_API_KEY}"
RUN echo "OPENAI_MODEL=${OPENAI_MODEL}"
RUN echo "GOOGLE_CHAT_WEBHOOK_URL=${GOOGLE_CHAT_WEBHOOK_URL}"
RUN echo "SYSTEM_PROMPT=${SYSTEM_PROMPT}"

WORKDIR /ai-summarizer

# RUN cargo install --path .
# RUN apt-get -y install podman
# RUN apt-get update 
# RUN apt-get -y install podman-docker

# ENV CROSS_CONTAINER_IN_CONTAINER=true
# RUN cargo install cross --git https://github.com/cross-rs/cross
# RUN rustup target add x86_64-unknown-linux-musl
# RUN apk add --no-cache musl-dev gcc

COPY . .

RUN cargo build  --release
# RUN cross build --target=x86_64-unknown-linux-musl --release


FROM docker.io/library/debian:bookworm-slim
ARG CRON_SCHEDULE
ARG OPENAI_API_KEY
ARG OPENAI_MODEL
ARG GOOGLE_CHAT_WEBHOOK_URL
ARG SYSTEM_PROMPT

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /ai-summarizer

COPY --from=builder /ai-summarizer/target/release/ai-summarizer ./

RUN echo "CRON_SCHEDULE=${CRON_SCHEDULE}"
RUN echo "OPENAI_API_KEY=${OPENAI_API_KEY}"
RUN echo "OPENAI_MODEL=${OPENAI_MODEL}"
RUN echo "GOOGLE_CHAT_WEBHOOK_URL=${GOOGLE_CHAT_WEBHOOK_URL}"
RUN echo "SYSTEM_PROMPT=${SYSTEM_PROMPT}"

ENV NUM_TITLES_TO_REQUEST=5
ENV MAX_NUMBER_OF_STORIES_TO_PRESENT=5
ENV CRON_SCHEDULE=${CRON_SCHEDULE}
ENV OPENAI_API_KEY=${OPENAI_API_KEY}
ENV OPENAI_MODEL=${OPENAI_MODEL}
ENV GOOGLE_CHAT_WEBHOOK_URL=${GOOGLE_CHAT_WEBHOOK_URL}
ENV SYSTEM_PROMPT=${SYSTEM_PROMPT}  


CMD ["./ai-summarizer"]
