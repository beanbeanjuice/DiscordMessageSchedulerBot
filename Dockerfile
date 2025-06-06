FROM rust:slim AS builder

WORKDIR /app/bot
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --target-dir ./
RUN ls -la ./release/

FROM debian:bookworm-slim

WORKDIR /app/bot
COPY --from=builder /app/bot/release/discord_message_scheduler_bot ./bot

RUN chmod +x ./bot

CMD ["./bot"]
