#!/bin/sh
# Run from any directory. Graphical tests are opt-in for CI/headless hosts.
set -eu
cd "$(dirname "$0")/.."
emotional_cargo=${EMOTIONAL_UI_CARGO:-cargo}
if ! command -v "$emotional_cargo" >/dev/null 2>&1; then emotional_cargo="$HOME/.cargo/bin/cargo"; fi
"$emotional_cargo" fmt --all -- --check
"$emotional_cargo" clippy --workspace --all-targets --locked -- -D warnings
"$emotional_cargo" test --workspace --locked
"$emotional_cargo" build --workspace --locked
python3 tests/contrast.py
python3 -m unittest discover -s tests -p 'test_phase3_tools.py' -v
python3 -m unittest discover -s tests -p 'test_phase5_tools.py' -v
if [ "${EMOTIONAL_UI_GRAPHICAL_TESTS:-0}" = 1 ]; then
    python3 tests/graphical.py
    python3 tests/graphical_smoke.py
fi
