FROM docker.io/rustlang/rust:nightly

WORKDIR /usr/src/ai-summarizer
COPY . .

RUN cargo install --path .

CMD ["ai-summarizer"]
