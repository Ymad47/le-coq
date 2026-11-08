#!/usr/bin/env bash
# Run plugin tests with the SAME python the backend uses (plugins/.venv).
# Prevents false positives from testing against a system python with different deps.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VENV_PYTHON="${LE_COQ_PLUGINS_PYTHON:-$ROOT/plugins/.venv/bin/python}"

if [[ ! -x "$VENV_PYTHON" ]]; then
    echo "ERR: backend venv not found at $VENV_PYTHON" >&2
    echo "Create it: python3.11 -m venv $ROOT/plugins/.venv && $VENV_PYTHON -m pip install -r $ROOT/plugins/requirements-dev.txt" >&2
    exit 1
fi

echo "Using: $VENV_PYTHON"
cd "$ROOT"

# Quick sanity: all plugins importable
"$VENV_PYTHON" - "$ROOT" <<'PY'
import sys
sys.path.insert(0, sys.argv[1])
from plugins import list_plugins, plugin_failures
loaded = list_plugins()
failed = plugin_failures()
print(f"loaded: {loaded}")
if failed:
    print(f"FAILED to import: {failed}", file=sys.stderr)
    sys.exit(2)
PY

# Run pytest with the same interpreter
exec "$VENV_PYTHON" -m pytest plugins/tests/ -q "$@"
