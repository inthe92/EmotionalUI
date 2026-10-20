#!/bin/sh
# Offline exports only; never installs encoders or plays sound.
set -eu
emotional_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if [ -n "${EMOTIONAL_FFMPEG:-}" ]; then
    emotional_encoder=$EMOTIONAL_FFMPEG
elif [ -x /opt/homebrew/opt/ffmpeg-full/bin/ffmpeg ]; then
    emotional_encoder=/opt/homebrew/opt/ffmpeg-full/bin/ffmpeg
else
    emotional_encoder=ffmpeg
fi
emotional_output=${1:-"$emotional_root/assets/audio/previews"}
mkdir -p "$emotional_output"
"$emotional_encoder" -nostdin -n -hide_banner -i "$emotional_root/assets/audio/previews/la-habitacion-permanece.wav" -c:a libvorbis -q:a 4 "$emotional_output/la-habitacion-permanece.ogg"
# AAC is an additional convenient preview for ordinary iPhone/Mac players.
"$emotional_encoder" -nostdin -n -hide_banner -i "$emotional_root/assets/audio/previews/la-habitacion-permanece.wav" -c:a aac -b:a 96k "$emotional_output/la-habitacion-permanece.m4a"
