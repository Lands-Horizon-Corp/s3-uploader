# Stage 1: Build the Rust binary
FROM rust:1.77 AS builder
WORKDIR /usr/src/app

# Copy Cargo files first for caching
COPY Cargo.toml Cargo.lock ./

# Fetch dependencies only
RUN cargo fetch

# Copy source code
COPY src ./src
COPY files ./files
COPY README.md LICENSE ./

# Build release binary
RUN cargo build --release

# Stage 2: Minimal runtime
FROM debian:bullseye-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary and files
COPY --from=builder /usr/src/app/target/release/s3_uploader .
COPY --from=builder /usr/src/app/files ./files

# Expose port for server command
EXPOSE 8080

# Entrypoint
ENTRYPOINT ["./s3_uploader"]
