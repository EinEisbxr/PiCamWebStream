# syntax=docker/dockerfile:1.6

FROM rust:slim AS builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libv4l-dev \
    clang \
    libclang-dev && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM debian:trixie-slim AS runtime
WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        gnupg \
        libv4l-0 && \
    curl -fsSL https://archive.raspberrypi.org/debian/raspberrypi.gpg.key | gpg --dearmor -o /etc/apt/trusted.gpg.d/raspberrypi.gpg && \
    echo "deb http://archive.raspberrypi.org/debian/ trixie main" > /etc/apt/sources.list.d/raspberrypi.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        rpicam-apps-lite && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/picam-backend /usr/local/bin/picam-backend

ENV BACKEND_HOST=0.0.0.0 \
    BACKEND_PORT=8080 \
    FRAME_RATE=30 \
    FRAME_WIDTH=3840 \
    FRAME_HEIGHT=2160

EXPOSE 8080

CMD ["picam-backend"]
