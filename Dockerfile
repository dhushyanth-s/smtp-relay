# Build stage
FROM rust:slim AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo.toml first for dependency caching
COPY Cargo.toml ./

# Create dummy main.rs to build dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer is cached unless Cargo.toml changes)
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src ./src

# Build the application (only recompiles changed files)
RUN cargo build --release

# Runtime stage
FROM debian:trixie-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/smtp-relay /app/smtp-relay

# Create /etc/smtp-relay directory for config (Unix standard location)
RUN mkdir -p /etc/smtp-relay && \
    useradd -m -u 1000 smtp && \
    chown -R smtp:smtp /app && \
    chown smtp:smtp /etc/smtp-relay

USER smtp

# Expose SMTP port
EXPOSE 2525

# Run the server
CMD ["./smtp-relay"]
