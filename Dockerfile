# Stage 1: Build Rust binary on Debian Bullseye
FROM rust:bullseye AS builder
WORKDIR /usr/src/app

# Copy source code
COPY . .

# Build release binary
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:bullseye-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy binary and files
COPY --from=builder /usr/src/app/target/release/s3-uploader .
COPY --from=builder /usr/src/app/files ./files

# Expose port for server
EXPOSE 8080

# Entrypoint: use shell so $PORT and S3 env vars are expanded
ENTRYPOINT ["/bin/sh", "-c", "./s3-uploader server --port ${PORT:-8080}"]
