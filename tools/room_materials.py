#!/usr/bin/env python3
"""Offline pixel-aligned 2px frames with supersampled circular corners.

Pillow is a production tool only. No runtime blur or second native border.
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageChops

ROOT = Path(__file__).resolve().parents[1] / 'assets/illustrations'
TOP = (59, 48, 37, 230)
BOTTOM = (22, 27, 23, 240)
BORDER = (233, 214, 180, 150)
FOCUS = (234, 209, 164, 255)
STROKE = 2
SCALE = 8
# Fixed-corner nine-slices for panels; capsules at exact logical dimensions.
SURFACES = {'room-material': (64, 64, 16), 'room-tile': (64, 64, 12),
            'room-action': (188, 36, 18), 'room-dock': (344, 52, 26),
            'room-status': (176, 28, 14)}


def render(width, height, radius, *, focused=False, selection=False, active=False, palette=None):
    top, bottom, border, focus = (palette if palette is not None else (TOP, BOTTOM, BORDER, FOCUS))
    w, h, r = width * SCALE, height * SCALE, radius * SCALE
    mask = Image.new('L', (w, h))
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, w - 1, h - 1), radius=r, fill=255)
    image = Image.new('RGBA', (w, h))
    draw = ImageDraw.Draw(image)
    for y in range(h):
        t = y / (h - 1)
        color = tuple(round(a + (b - a) * t) for a, b in zip(top, bottom))
        if selection:
            color = (*focus[:3], 33 if active else 0)
        draw.line((0, y, w - 1, y), fill=color)
    image.putalpha(ImageChops.multiply(image.getchannel('A'), mask))
    if not selection or focused:
        ring = mask.copy()
        inset = STROKE * SCALE
        ImageDraw.Draw(ring).rounded_rectangle((inset, inset, w - inset - 1, h - inset - 1), radius=r - inset, fill=0)
        stroke = Image.new('RGBA', (w, h), focus if focused else border)
        stroke.putalpha(ImageChops.multiply(stroke.getchannel('A'), ring))
        image = Image.alpha_composite(image, stroke)
    return image.resize((width, height), Image.Resampling.LANCZOS)


def panel_palette(palette, profile):
    """More scene visibility in large panels, compensated by darker pigments.

    Only the fill changes. Preserve the original 2px outline and AA coverage.
    """
    top,bottom,border,focus=palette
    def adjust(color,scale,reduction):
        return (*[round(c*scale) for c in color[:3]],max(0,color[3]-reduction))
    return (adjust(top,profile['top_rgb_scale'],profile['top_alpha_reduction']),
            adjust(bottom,profile['bottom_rgb_scale'],profile['bottom_alpha_reduction']),border,focus)


def generate(output=ROOT, palette=None, capsule_top=None, panel_profile=None):
    output.mkdir(parents=True, exist_ok=True)
    for name, (width, height, radius) in SURFACES.items():
        frame_palette = (capsule_top, *palette[1:]) if name == 'room-status' and palette is not None and capsule_top is not None else palette
        if name=='room-material' and palette is not None and panel_profile is not None:
            frame_palette=panel_palette(palette,panel_profile)
        for focused in ((False,) if name == 'room-dock' else (False, True)):
            render(width, height, radius, focused=focused, palette=frame_palette).save(output / (name + ('-focus' if focused else '') + '.png'))
    for name, focused, active in [('active', False, True), ('focus', True, False), ('active-focus', True, True)]:
        render(80, 44, 22, focused=focused, selection=True, active=active, palette=palette).save(output / ('room-dock-selection-' + name + '.png'))


if __name__ == '__main__':
    generate()
