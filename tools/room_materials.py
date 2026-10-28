#!/usr/bin/env python3
"""Offline frames with supersampled circular corners and panel-specific strokes.

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
            'room-status': (176, 28, 14), 'room-tab': (100,30,15), 'room-hint': (18,18,9)}


def render(width, height, radius, *, focused=False, selection=False, active=False, palette=None, stroke_width=STROKE, fantasy_focus=False):
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
        inset = round(stroke_width * SCALE)
        ImageDraw.Draw(ring).rounded_rectangle((inset, inset, w - inset - 1, h - inset - 1), radius=r - inset, fill=0)
        if focused and fantasy_focus:
            # One baked contour: no extra runtime image, glow or animation.
            # Bilinear corner colors stay continuous through the nine-slice seams.
            stroke = Image.new('RGBA', (2, 2))
            stroke.putdata([(88, 244, 221, 250), (124, 170, 255, 250),
                            (113, 151, 255, 250), (224, 135, 255, 250)])
            stroke = stroke.resize((w, h), Image.Resampling.BILINEAR)
        else:
            # Neutral, non-uniform light on the unfocused glass edge.
            alpha = (focus if focused else border)[3]
            stroke = Image.new('RGBA', (w, h))
            edge = ImageDraw.Draw(stroke)
            for yy in range(h):
                light = .90 - .45 * yy / max(1,h-1)
                edge.line((0,yy,w-1,yy),fill=(242,246,250,round(alpha*light)))
        stroke.putalpha(ImageChops.multiply(stroke.getchannel('A'), ring))
        image = Image.alpha_composite(image, stroke)
    return image.resize((width, height), Image.Resampling.LANCZOS)


def panel_palette(palette, profile):
    """More scene visibility in large panels, compensated by darker pigments.

    Panel-specific glass fill and restrained borders; other controls stay unchanged.
    """
    top,bottom,border,focus=palette
    def adjust(color,scale,reduction):
        return (*[round(c*scale) for c in color[:3]],max(0,color[3]-reduction))
    return (adjust(top,profile['top_rgb_scale'],profile['top_alpha_reduction']),
            adjust(bottom,profile['bottom_rgb_scale'],profile['bottom_alpha_reduction']),
            (*border[:3],round(border[3]*profile.get('border_alpha_scale',1))),
            (*focus[:3],round(focus[3]*profile.get('focus_alpha_scale',1))))


def generate(output=ROOT, palette=None, capsule_top=None, panel_profile=None, capsule_alpha_reduction=0):
    output.mkdir(parents=True, exist_ok=True)
    for name, (width, height, radius) in SURFACES.items():
        frame_palette = (capsule_top, *palette[1:]) if name == 'room-status' and palette is not None and capsule_top is not None else palette
        if name == 'room-status' and frame_palette is not None:
            top,bottom,border,focus=frame_palette
            frame_palette=((*top[:3],max(0,top[3]-capsule_alpha_reduction)),(*bottom[:3],max(0,bottom[3]-capsule_alpha_reduction)),border,focus)
        if name in ('room-material','room-tile') and palette is not None and panel_profile is not None:
            frame_palette=panel_palette(palette,panel_profile)
        if name in ('room-tab','room-hint'):
            frame_palette=((255,255,255,44),(255,255,255,12),(242,246,250,88),(242,246,250,160))
        for focused in ((False,) if name == 'room-dock' else (False, True)):
            panel = name in ('room-material','room-tile') and panel_profile is not None
            stroke_width = panel_profile.get('focus_stroke_width' if focused else 'stroke_width', STROKE) if panel else (1.0 if focused else 0.75)
            card_focus = focused and name in ('room-material', 'room-tile')
            render(width, height, radius, focused=focused, palette=frame_palette,
                   stroke_width=STROKE if card_focus else stroke_width,
                   fantasy_focus=card_focus).save(output / (name + ('-focus' if focused else '') + '.png'))
    for name, focused, active in [('active', False, True), ('focus', True, False), ('active-focus', True, True)]:
        render(80, 44, 22, focused=focused, selection=True, active=active, palette=palette, stroke_width=1.0).save(output / ('room-dock-selection-' + name + '.png'))


if __name__ == '__main__':
    generate()
