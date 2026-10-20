#!/bin/sh
# Compare the cached cross-compiled release simulator without graphical/audio devices.
set -eu
emotional_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
: "${EMOTIONAL_DOCKER_CONTEXT:=colima-emotional-ui}"
: "${EMOTIONAL_SIM_IMAGE:=emotional-ui-simulator:debian13-arm64}"
emotional_output=${1:?Provide a new output directory}
mkdir -p "$emotional_output"
emotional_output=$(CDPATH= cd -- "$emotional_output" && pwd)
if [ -e "$emotional_output/phase8" ]; then echo 'Preserve previous evidence: use a new directory' >&2; exit 1; fi
docker --context "$EMOTIONAL_DOCKER_CONTEXT" run --rm --platform linux/arm64 \
    --user "$(id -u):$(id -g)" -e XDG_CACHE_HOME=/tmp/cache \
    --network none --read-only --cap-drop ALL --security-opt no-new-privileges \
    --pids-limit 128 --memory 512m --cpus 2 --tmpfs /tmp:rw,nosuid,nodev,size=64m \
    --mount "type=bind,src=$emotional_root,dst=/workspace,readonly" \
    --mount "type=bind,src=$emotional_output,dst=/results" \
    --entrypoint python3 "$EMOTIONAL_SIM_IMAGE" \
    /workspace/platform/simulator/phase8.py /workspace/dist/phase8-cross/emotional-ui-simulator /results/phase8
