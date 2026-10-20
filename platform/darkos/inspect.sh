#!/bin/sh
# JSON diagnostic only. Does not modify the device or consume input events.
set -eu
emotional_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 "$emotional_dir/diagnostic.py"
