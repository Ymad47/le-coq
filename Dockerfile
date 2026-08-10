FROM rust:latest as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    python3-dev \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*

# Set Python version for PyO3 (use ABI3 compatibility for Python 3.13)
ENV PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1

# Copy source
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend

# Build
RUN cargo build --release --package le-coq-backend

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (Python 3.11 for PyO3 compatibility)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    python3.11 \
    libpython3.11 \
    python3.11-venv \
    && rm -rf /var/lib/apt/lists/*

# Copy built binary
COPY --from=builder /app/target/release/le-coq-backend .

# Copy Python plugins
COPY plugins ./plugins

# Set environment
ENV PYTHONPATH=/app
ENV LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu
ENV PYO3_PYTHON=python3.11

EXPOSE 8000

CMD ["./le-coq-backend"]
