# syntax=docker/dockerfile:1.6

FROM rust:1.78-slim AS builder
WORKDIR /app

# Install build dependencies (placeholder - add camera libs when integrating hardware capture)
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN addgroup --system app && \
    adduser --system --ingroup app app

COPY --from=builder /app/target/release/picam-backend /usr/local/bin/picam-backend

ENV BACKEND_HOST=0.0.0.0 \
    BACKEND_PORT=8080 \
    FRAME_RATE=12 \
    FRAME_WIDTH=1280 \
    FRAME_HEIGHT=720

EXPOSE 8080

USER app

CMD ["picam-backend"]
