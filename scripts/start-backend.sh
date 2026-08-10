#!/bin/bash
# Start Le Coq backend

export LD_LIBRARY_PATH="/home/yaya/.local/share/uv/python/cpython-3.11.15-linux-x86_64-gnu/lib:$LD_LIBRARY_PATH"
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/le_coq"
export BIND_ADDRESS="0.0.0.0:8000"
export RUST_LOG="info"
export DISABLE_AUTH="true"

cd /home/yaya/le-coq
exec ./target/release/le-coq-backend
