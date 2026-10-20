#!/bin/sh
# Python harness supervises only the EmotionalUI child, never a system service.
set -eu
emotional_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 "$emotional_dir/manual_smoke.py" "$@"
