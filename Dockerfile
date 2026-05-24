FROM rust:1.78-alpine AS builder

WORKDIR /usr/src/halimun-proxy
RUN apk add --no-cache musl-dev gcc

# Create dummy src for dependency caching
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Build actual code
COPY src ./src
# Touch main to force rebuild
RUN touch src/main.rs
RUN cargo build --release

# Stage 2: Minimal runtime
FROM alpine:3.19

WORKDIR /app
RUN apk add --no-cache libgcc

COPY --from=builder /usr/src/halimun-proxy/target/release/halimun-proxy ./halimun-proxy

EXPOSE 80 9090

CMD ["./halimun-proxy", "--config", "config.yaml"]
