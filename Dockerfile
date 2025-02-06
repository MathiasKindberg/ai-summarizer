FROM docker.io/rustlang/rust:nightly AS builder

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

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /ai-summarizer

COPY --from=builder /ai-summarizer/target/x86_64-unknown-linux-musl/release/ai-summarizer ./

USER ai-summarizer:ai-summarizer
CMD ["./ai-summarizer"]
