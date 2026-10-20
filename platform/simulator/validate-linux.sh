#!/bin/sh
# Runs inside the existing ARM64 Docker VM. Project bind mount is read-only.
set -eu
mkdir -p /results/tests
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked --no-default-features --features emotional-ui/simulator -- -D warnings
cargo test --workspace --locked --no-default-features --features emotional-ui/simulator
cargo build --workspace --locked --no-default-features --features emotional-ui/simulator
cp /build/target/debug/emotional-ui /results/emotional-ui-simulator
cargo test --workspace --locked --no-default-features --features emotional-ui/simulator --no-run --message-format=json > /results/tests-build.jsonl
python3 /workspace/platform/simulator/stage_tests.py /results
cargo build -p emotional-ui --locked --no-default-features --features linuxkms-direct
cp /build/target/debug/emotional-ui /results/emotional-ui-kms
python3 -m unittest discover -s tests -p 'test_phase*_tools.py' -v
python3 tests/contrast.py --output /results/contrast-audit.json
cargo run -p emotional-ui --example catalog_bench --locked > /results/performance-phase7.json
cargo run -p emotional-services --example audio_probe --locked > /results/audio-lifecycle-linux.json
cp /build/target/debug/examples/audio_probe /results/audio-probe
python3 /workspace/platform/simulator/verify_elf.py /results/emotional-ui-simulator /results/emotional-ui-kms > /results/elf-format.txt
readelf -d /results/emotional-ui-kms > /results/elf-dynamic.txt
readelf --version-info /results/emotional-ui-kms > /results/elf-versions.txt
dpkg-query -W > /results/packages.txt
rustc --version > /results/rust-version.txt

cargo run -p emotional-ui --example phase10_bench --locked > /results/performance-phase10.json
cargo run -p emotional-ui --example phase10_evaluate --locked > /results/evaluation-phase10.json

cargo run -p emotional-ui --example phase11_evaluate --locked > /results/evaluation-phase11.json

# Build the artwork benchmark; execute it separately without competing builds.
cargo build -p emotional-ui --example artwork_bench --locked
cp /build/target/debug/examples/artwork_bench /results/artwork-bench
