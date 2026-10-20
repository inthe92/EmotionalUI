#!/bin/sh
# Finder-friendly launcher; builds the current sources if needed.
set -eu
cd "$(dirname "$0")"
if command -v cargo >/dev/null 2>&1; then
    emotional_cargo=cargo
elif [ -x "$HOME/.cargo/bin/cargo" ]; then
    emotional_cargo="$HOME/.cargo/bin/cargo"
else
    printf '%s\n' 'Instala Rust desde https://rustup.rs para ejecutar EmotionalUI.'
    exit 1
fi
exec "$emotional_cargo" run --locked -p emotional-ui -- "$@"
