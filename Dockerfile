# Build stage with musl for Alpine compatibility
FROM rust:1.83-alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

# Copy everything
COPY . .

# Build with static linking for OpenSSL
ENV OPENSSL_STATIC=1
ENV OPENSSL_LIB_DIR=/usr/lib
ENV OPENSSL_INCLUDE_DIR=/usr/include

RUN cargo build --release

# Runtime stage - minimal Alpine
FROM alpine:3.19

WORKDIR /app

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Copy binary from builder
COPY --from=builder /app/target/release/rust-kv /app/rust-kv

EXPOSE 11223

CMD ["./rust-kv"]
