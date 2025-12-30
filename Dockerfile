# Build stage
FROM rust:1.83-alpine AS builder

WORKDIR /app

# Install build dependencies including protoc for gRPC
RUN apk add --no-cache musl-dev protobuf-dev pkgconfig

# Copy everything
COPY . .

# Build release
RUN cargo build --release

# Runtime stage
FROM alpine:3.19

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/raft-kv /app/raft-kv

# REST port
EXPOSE 8080
# gRPC port
EXPOSE 50051

CMD ["./raft-kv"]
