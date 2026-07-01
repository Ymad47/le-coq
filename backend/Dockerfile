FROM rust:1.75-slim

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml ./

# Create dummy source for caching
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Cache dependencies
RUN cargo build --release && rm -rf src

# Copy actual source
COPY src ./src
COPY migrations ./migrations

# Build
RUN cargo build --release

# Run
CMD ["./target/release/le-coq-backend"]
