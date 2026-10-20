#!/bin/sh
# Real Slint RGB renders, no console/device access. Never resize a UI screenshot.
set -eu
emotional_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$emotional_root"
emotional_output=${1:-"$emotional_root/docs/visual-correction/review-$(date +%Y%m%d-%H%M%S)"}
if [ -e "$emotional_output" ]; then
    echo "Output already exists; choose a new folder." >&2
    exit 1
fi
emotional_cargo=$(command -v cargo || true)
if [ -z "$emotional_cargo" ]; then emotional_cargo="$HOME/.cargo/bin/cargo"; fi
"$emotional_cargo" build -p emotional-ui --locked --no-default-features --features simulator
mkdir -p "$emotional_root/dist/local"
cp "$emotional_root/target/debug/emotional-ui" "$emotional_root/dist/local/emotional-ui-simulator"
mkdir -p "$emotional_output"
"$emotional_root/dist/local/emotional-ui-simulator" --db :memory: --view resume --demo-history --at 2026-10-18T17:42 --weather clear --snapshot "$emotional_output/continuar.png"
"$emotional_root/dist/local/emotional-ui-simulator" --db :memory: --view emotional --at 2026-10-18T17:42 --weather clear --snapshot "$emotional_output/emotional-ui.png"
