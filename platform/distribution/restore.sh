#!/bin/sh
# Does not edit boot or system services.
set -eu
emotional_package=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 "$emotional_package/manage.py" restore --package "$emotional_package" "$@"
