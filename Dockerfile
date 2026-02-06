FROM rust:1.77 AS builder
WORKDIR /usr/src/app

# Copy everything at once
COPY . .

# Now fetch dependencies
RUN cargo fetch

# Build release binary
RUN cargo build --release
