#!/bin/sh
# Cross-build using the same pinned ARM64 SDK as the console build.
set -eu
emotional_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
emotional_cargo=${EMOTIONAL_CARGO:-"$HOME/.cargo/bin/cargo"}
emotional_dest="$emotional_root/dist/simulator"
mkdir -p "$emotional_dest/tests"
EMOTIONAL_KMS_FEATURE=linuxkms-direct "$emotional_root/tools/arm64/build.sh"
cp "$emotional_root/target/aarch64-unknown-linux-gnu/release/emotional-ui" "$emotional_dest/emotional-ui-kms"
EMOTIONAL_KMS_FEATURE=simulator "$emotional_root/tools/arm64/build.sh"
cp "$emotional_root/target/aarch64-unknown-linux-gnu/release/emotional-ui" "$emotional_dest/emotional-ui-simulator"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$emotional_root/tools/arm64/cc.py"
export CC_aarch64_unknown_linux_gnu="$emotional_root/tools/arm64/cc.py"
export AR_aarch64_unknown_linux_gnu="$emotional_root/tools/arm64/ar.py"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR="$EMOTIONAL_ARM64_SYSROOT"
export PKG_CONFIG_LIBDIR="$EMOTIONAL_ARM64_SYSROOT/usr/lib/aarch64-linux-gnu/pkgconfig:$EMOTIONAL_ARM64_SYSROOT/usr/share/pkgconfig"
export PKG_CONFIG_PATH=
cd "$emotional_root"
"$emotional_cargo" test --workspace --locked --target aarch64-unknown-linux-gnu \
    --no-default-features --features emotional-ui/simulator --no-run --message-format=json > "$emotional_dest/tests-build.jsonl"
python3 "$emotional_root/platform/simulator/stage_tests.py" "$emotional_dest"
