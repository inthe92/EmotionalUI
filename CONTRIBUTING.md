# Contributing

EmotionalUI is an experimental native Rust/Slint frontend, GPL-3.0-only. Contributions must preserve 640×480 readability, button navigation, offline use and existing user data. Keep changes focused and include a reproducible check appropriate to the behavior changed.

Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets --locked -- -D warnings, cargo test --workspace --locked and cargo build --workspace --locked. Python tool tests use unittest discovery: tests/test_phase*_tools.py. Linux ARM64 simulation is documented in SIMULATOR.md; it does not certify a handheld.

Atlas contributions: preserve stable IDs, cite primary gameplay sources, separate objective facts from interpretations, keep unknown values unknown and label generated proposals. Schema validation cannot approve editorial quality. Do not invent human reviewers. See ATLAS_EDITORIAL_REVIEW.md.

Do not contribute ROMs, BIOS, commercial artwork, credentials, private logs or untraceable samples. Explain asset licenses separately from code licenses. Physical tests require a compatible device, current supervision and independently verified recovery. Never change boot as part of a routine test.

No public repository or maintainer contact is announced yet. These instructions prepare collaboration; they do not imply an active contribution service.
