#!/usr/bin/env python3
"""Regenerate the shared warm material, offline. Pillow is a production tool only.

Rounded translucent gradients are prerendered because Slint 1.13's software
renderer produces corner artifacts with a native rounded gradient. PNG avoids
its SVG/nine-slice zero-sized texture path. Runtime does no blur or synthesis.
"""
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1] / 'assets/illustrations'
TOP = (59, 48, 37, 230)
BOTTOM = (22, 27, 23, 240)
# Insets leave a nonzero center in the smallest destination (software renderer).
SURFACES = {'room-material': (64,64,14), 'room-tile': (64,64,10), 'room-action': (184,32,16), 'room-dock': (340,48,24), 'room-status': (172,24,12)}
for name, (width,height,radius) in SURFACES.items():
    scale = 4
    w,h = width * scale,height * scale
    image = Image.new('RGBA', (w, h))
    pixels = image.load()
    mask = Image.new('L', (w, h))
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, w - 1, h - 1), radius=radius * scale, fill=255)
    for y in range(h):
        for x in range(w):
            t = (x/(w - 1) + y/(h - 1))/2
            color = tuple(round(a + (b - a) * t) for a, b in zip(TOP, BOTTOM))
            pixels[x, y] = (*color[:3], color[3] * mask.getpixel((x, y)) // 255)
    image.resize((width, height), Image.Resampling.LANCZOS).save(ROOT / (name + '.png'))
