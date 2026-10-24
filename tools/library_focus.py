#!/usr/bin/env python3
"""Prepare the Library's transparent focus ring using the approved card colors.

Offline asset generation only; Pillow is not a runtime dependency.
"""
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw

# Same four-corner palette as the approved card focus, without its glass fill.
COLORS = [(88, 244, 221, 250), (124, 170, 255, 250),
          (113, 151, 255, 250), (224, 135, 255, 250)]
SIZE, RADIUS, STROKE, SCALE = 64, 10, 2, 8


def render():
    size, radius, inset = SIZE * SCALE, RADIUS * SCALE, STROKE * SCALE
    ring = Image.new('L', (size, size))
    draw = ImageDraw.Draw(ring)
    draw.rounded_rectangle((0, 0, size - 1, size - 1), radius, fill=255)
    draw.rounded_rectangle((inset, inset, size - inset - 1, size - inset - 1),
                           radius - inset, fill=0)
    colors = Image.new('RGBA', (2, 2))
    colors.putdata(COLORS)
    colors = colors.resize((size, size), Image.Resampling.BILINEAR)
    colors.putalpha(ImageChops.multiply(colors.getchannel('A'), ring))
    return colors.resize((SIZE, SIZE), Image.Resampling.LANCZOS)


if __name__ == '__main__':
    render().save(Path(__file__).resolve().parents[1] /
                  'assets/illustrations/library-focus.png')
