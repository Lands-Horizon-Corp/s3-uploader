# Stage 1: Build Rust binary on Debian Bullseye
FROM rust:bullseye AS builder
WORKDIR /usr/src/app

COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bullseye-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/s3-uploader .
COPY --from=builder /usr/src/app/files ./files

EXPOSE 8080
ENTRYPOINT ["./s3-uploader"]
