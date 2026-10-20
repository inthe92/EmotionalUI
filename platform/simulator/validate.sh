#!/bin/sh
# Reuse the existing ARM64 VM. No console access and no privileged containers.
set -eu
emotional_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
: "${EMOTIONAL_DOCKER_CONTEXT:=colima-emotional-ui}"
: "${EMOTIONAL_BUILD_IMAGE:=emotional-ui-builder:rust1.99.1-arm64}"
mkdir -p "$emotional_root/dist/simulator"
if ! docker --context "$EMOTIONAL_DOCKER_CONTEXT" image inspect "$EMOTIONAL_BUILD_IMAGE" >/dev/null 2>&1; then
    docker --context "$EMOTIONAL_DOCKER_CONTEXT" build --platform linux/arm64 -f "$emotional_root/platform/simulator/Dockerfile.build" -t "$EMOTIONAL_BUILD_IMAGE" "$emotional_root/platform/simulator"
fi
docker --context "$EMOTIONAL_DOCKER_CONTEXT" run --rm --platform linux/arm64 \
    --cpus 2 --memory 1700m --cap-drop ALL --security-opt no-new-privileges \
    --mount "type=bind,src=$emotional_root,dst=/workspace,readonly" \
    --mount "type=bind,src=$emotional_root/dist/simulator,dst=/results" \
    --mount type=volume,src=emotional-ui-cargo-registry,dst=/usr/local/cargo/registry \
    --mount type=volume,src=emotional-ui-build,dst=/build \
    "$EMOTIONAL_BUILD_IMAGE"
