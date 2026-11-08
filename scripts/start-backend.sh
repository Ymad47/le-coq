#!/usr/bin/env bash
# Start the Le Coq backend (release build) from the repository root.
# Required: DATABASE_URL and LE_COQ_API_KEY, from the environment or .env.backend.
# If libpython3.11 is not on the default library path, set LD_LIBRARY_PATH.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export BIND_ADDRESS="${BIND_ADDRESS:-127.0.0.1:8000}"
export RUST_LOG="${RUST_LOG:-info}"
export LE_COQ_ROOT="$ROOT"

exec ./target/release/le-coq-backend
