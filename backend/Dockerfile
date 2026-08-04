FROM rust:1.85-slim AS builder

WORKDIR /app

# Install system deps
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest
COPY Cargo.toml ./

# Create dummy source for caching
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs && echo "pub fn lib_dummy() {}" > backend/src/lib.rs

# Cache dependencies
RUN cargo build --release && rm -rf backend

# Copy actual source code
COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations

# Build
RUN cargo build --release

# Runtime stage with Python 3.11 (matching build environment)
FROM python:3.11-slim

WORKDIR /app

# Copy built binary from builder
COPY --from=builder /app/target/release/le-coq-backend ./

# Install runtime deps for PyO3
RUN apt-get update && apt-get install -y \
    libpython3.11 \
    && rm -rf /var/lib/apt/lists/*

# Copy Python plugins
COPY plugins ./plugins
COPY Cargo.toml ./

CMD ["./le-coq-backend"]
