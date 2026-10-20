#!/bin/sh
# Explicit context: does not replace the user's default Docker runtime.
set -eu
emotional_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
: "${EMOTIONAL_DOCKER_CONTEXT:=colima-emotional-ui}"
: "${EMOTIONAL_SIM_IMAGE:=emotional-ui-simulator:debian13-arm64}"
emotional_output=${1:-"$emotional_root/docs/simulator/run-$(date +%Y%m%d-%H%M%S)"}
mkdir -p "$emotional_output"
emotional_output=$(CDPATH= cd -- "$emotional_output" && pwd)
if ! docker --context "$EMOTIONAL_DOCKER_CONTEXT" image inspect "$EMOTIONAL_SIM_IMAGE" >/dev/null 2>&1; then
    docker --context "$EMOTIONAL_DOCKER_CONTEXT" build --platform linux/arm64 -t "$EMOTIONAL_SIM_IMAGE" "$emotional_root/platform/simulator"
fi
docker --context "$EMOTIONAL_DOCKER_CONTEXT" run --rm --platform linux/arm64 \
    --user "$(id -u):$(id -g)" -e XDG_CACHE_HOME=/tmp/cache \
    --network none --read-only --cap-drop ALL --security-opt no-new-privileges \
    --pids-limit 128 --memory 512m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=64m \
    --mount "type=bind,src=$emotional_root,dst=/workspace,readonly" \
    --mount "type=bind,src=$emotional_output,dst=/results" \
    "$EMOTIONAL_SIM_IMAGE"
