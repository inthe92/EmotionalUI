#!/bin/sh
# Reproducible cross-build; host libraries are excluded from pkg-config.
set -eu
emotional_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
: "${EMOTIONAL_ZIG:?Set absolute path to Zig 0.14.1 for this host}"
: "${EMOTIONAL_ARM64_SYSROOT:?Set the reference or verified device SDK root}"
if [ "$("$EMOTIONAL_ZIG" version)" != 0.14.1 ]; then
    echo 'This build wrapper requires Zig 0.14.1' >&2
    exit 1
fi
: "${EMOTIONAL_GLIBC_FLOOR:=2.36}"
: "${EMOTIONAL_KMS_FEATURE:=linuxkms}"
case "$EMOTIONAL_KMS_FEATURE" in
    linuxkms|linuxkms-direct|simulator) ;;
    *) echo 'Unsupported KMS feature' >&2; exit 1 ;;
esac
export EMOTIONAL_GLIBC_FLOOR
emotional_target=aarch64-unknown-linux-gnu
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$emotional_root/tools/arm64/cc.py"
export CC_aarch64_unknown_linux_gnu="$emotional_root/tools/arm64/cc.py"
export AR_aarch64_unknown_linux_gnu="$emotional_root/tools/arm64/ar.py"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR="$EMOTIONAL_ARM64_SYSROOT"
export PKG_CONFIG_LIBDIR="$EMOTIONAL_ARM64_SYSROOT/usr/lib/aarch64-linux-gnu/pkgconfig:$EMOTIONAL_ARM64_SYSROOT/usr/share/pkgconfig"
export PKG_CONFIG_PATH=
: "${CARGO_BUILD_JOBS:=4}"
export CARGO_BUILD_JOBS
cd "$emotional_root"
exec "${EMOTIONAL_CARGO:-$HOME/.cargo/bin/cargo}" build --locked --release -p emotional-ui --target "$emotional_target" --no-default-features --features "$EMOTIONAL_KMS_FEATURE"
