# --- Build Stage ---
FROM rust:1.80-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libwebp-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source files to cache dependencies
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    touch src/lib.rs && \
    echo "fn main() {}" > src/bin/cli.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release

# Copy actual source code
COPY . .

# Build the actual binaries
# We need to touch the files to ensure cargo notices the changes
RUN touch src/main.rs src/lib.rs src/bin/cli.rs && \
    cargo build --release

# --- Runtime Stage ---
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libwebp7 \
    libssl3 \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/mangad /app/
COPY --from=builder /app/target/release/mangad-cli /app/

# Copy configuration template as default config
RUN mkdir -p /app/config
COPY --from=builder /app/config/template.toml /app/config/config.toml

# Copy initialization/migration scripts
COPY --from=builder /app/init /app/init

# Create storage directory
RUN mkdir -p /app/storage

# Set default environment variables
ENV MANGAD_SERVICE_HOST=0.0.0.0:6789
ENV MANGAD_CONFIG_PATH=/app/config/config.toml
ENV MANGAD_CRAWLER_STORAGE=/app/storage

# Expose the service port
EXPOSE 6789

# Run the server
CMD ["./mangad"]
