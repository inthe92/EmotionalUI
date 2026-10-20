#!/bin/sh
# Reuse built binaries/images. Serialize graphics/performance checks; no devices.
set -eu
[ "$#" -eq 2 ] || { echo 'Usage: artwork_validate.sh NEW_OUTPUT EXISTING_FIXTURES' >&2; exit 2; }
emotional_repo=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
mkdir -p "$1"
emotional_output=$(CDPATH= cd -- "$1" && pwd)
emotional_fixtures=$(CDPATH= cd -- "$2" && pwd)
: "${EMOTIONAL_DOCKER_CONTEXT:=colima-emotional-ui}"
: "${EMOTIONAL_SIM_IMAGE:=emotional-ui-simulator:debian13-arm64}"
[ ! -e "$emotional_output/performance.json" ] || { echo 'Preserve previous evidence: use a new output folder.' >&2; exit 1; }
run_tool() {
 docker --context "$EMOTIONAL_DOCKER_CONTEXT" run --rm --platform linux/arm64 \
  --user "$(id -u):$(id -g)" -e XDG_CACHE_HOME=/tmp/cache --network none --read-only \
  --cap-drop ALL --security-opt no-new-privileges --pids-limit 128 --cpus 2 --memory 512m \
  --tmpfs /tmp:rw,nosuid,nodev,size=64m \
  --mount "type=bind,src=$emotional_repo,dst=/workspace,readonly" \
  --mount "type=bind,src=$emotional_output,dst=/results" \
  --mount "type=bind,src=$emotional_fixtures,dst=/fixtures,readonly" \
  --entrypoint python3 "$EMOTIONAL_SIM_IMAGE" "$@"
}
run_tool /workspace/platform/simulator/artwork_measure.py /workspace/dist/simulator/artwork-bench /results/performance.json
"$emotional_repo/platform/simulator/run.sh" "$emotional_output/regression"
run_tool /workspace/platform/simulator/navigation_ux.py /workspace/dist/simulator/emotional-ui-simulator /results/navigation
run_tool /workspace/platform/simulator/artwork_visual.py /workspace/dist/simulator/emotional-ui-simulator /fixtures /results/captures

run_tool /workspace/platform/simulator/artwork_primary.py /workspace/dist/simulator/emotional-ui-simulator /results/captures
