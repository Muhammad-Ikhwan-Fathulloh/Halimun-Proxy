# syntax=docker/dockerfile:1
# Stage 1: build statis (musl) dengan cache dependensi terpisah
FROM rust:alpine AS builder

WORKDIR /usr/src/halimun-proxy
RUN apk add --no-cache musl-dev gcc

# Layer dependensi (di-cache selama Cargo.toml/Cargo.lock tidak berubah)
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src

# Build kode sebenarnya
COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

# Stage 2: runtime minimal, non-root, tanpa shell pengguna
FROM alpine:3.20

RUN apk add --no-cache ca-certificates \
    && addgroup -S halimun \
    && adduser -S -G halimun -h /app halimun

WORKDIR /app
COPY --from=builder /usr/src/halimun-proxy/target/release/halimun-proxy ./halimun-proxy

USER halimun
# 8080 = proxy (internal Docker), 9090 = admin API (jangan dipublikasikan ke host)
EXPOSE 8080 9090

ENTRYPOINT ["./halimun-proxy"]
CMD ["--config", "/app/config.yaml"]
