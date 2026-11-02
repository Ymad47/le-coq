FROM python:3.11-slim AS python-runtime

FROM rust:1.96-slim AS builder

WORKDIR /app

# Install system deps
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Link PyO3 against same Python shared library used at runtime
COPY --from=python-runtime /usr/local/lib/libpython3.11.so* /usr/local/lib/

# PyO3 uses abi3-py311; build without a host interpreter so runtime stays Python 3.11.
ENV PYO3_NO_PYTHON=1
ENV RUSTFLAGS="-L native=/usr/local/lib"

# Copy manifest and lockfile
COPY Cargo.toml Cargo.lock ./

# Create dummy source for caching
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs && echo "pub fn lib_dummy() {}" > backend/src/lib.rs

# Cache dependencies
RUN cargo build --locked --release && rm -rf backend target/release

# Copy actual source code
COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations

# Build
RUN cargo build --locked --release

# Runtime stage with Python 3.11 (matching build environment)
FROM python:3.11-slim

WORKDIR /app

# Copy built binary from builder
COPY --from=builder /app/target/release/le-coq-backend ./

# Python 3.11 runtime (includes libpython3.11 under /usr/local/lib)

# Copy Python plugins
COPY plugins ./plugins
COPY Cargo.toml ./

CMD ["./le-coq-backend"]
