#!/usr/bin/env python3
"""Audit actual prerendered pixels for ALL daily palettes, not the LCD."""
import json
import sys
from pathlib import Path
from PIL import Image, ImageDraw
from room_materials import SURFACES
from daycycle_assets import catalog, FOLDER, rgba


def luminance(c):
    return sum((x/12.92 if x <= .04045 else ((x+.055)/1.055)**2.4)*w
               for x, w in zip(c, (.2126, .7152, .0722)))


def contrast(a, b):
    lo, hi = sorted((luminance(a), luminance(b)))
    return (hi+.05)/(lo+.05)


def audit():
    report = {}
    rows=catalog()
    all_text={r["id"]+k:[c/255 for c in rgba(r["colors"][k])[:3]] for r in rows for k in ("primary_text","secondary_text","focus_accent")}
    for row in rows:
        colors = {k: [c/255 for c in rgba(row['colors'][k])[:3]]
                  for k in ('primary_text', 'secondary_text', 'focus_accent')}
        for name, (width, height, radius) in SURFACES.items():
            for focused in ((False,) if name == 'room-dock' else (False, True)):
                path = FOLDER/'materials'/row['id']/(name+('-focus' if focused else '')+'.png')
                # Exclude only the perimeter and its antialias coverage from text zones.
                interior = Image.new('L', (width, height))
                ImageDraw.Draw(interior).rounded_rectangle((4, 4, width-5, height-5),
                                                          radius=max(0, radius-4), fill=255)
                with Image.open(path) as image:
                    image.load()
                    assert image.mode == 'RGBA' and image.size == (width, height)
                    assert image.getpixel((0, 0))[3] == 0, ('Square corner', path)
                    samples = {image.getpixel((x, y)) for y in range(height) for x in range(width)
                               if interior.getpixel((x, y)) == 255 and image.getpixel((x, y))[3] >= 225}
                assert samples
                minimum = 100
                for r, g, b, a in samples:
                    for scene in (0., 1.):
                        bg = [x/255*a/255+scene*(1-a/255) for x in (r, g, b)]
                        for strength in (0., .14):
                            lit = [x*(1-strength)+y*strength for x, y in zip(bg, colors['focus_accent'])]
                            minimum = min(minimum, *(contrast(color, lit) for color in all_text.values()))
                assert minimum >= 4.5, (path, minimum)
                report[row['id']+'/'+path.name] = dict(size=[width, height], minimum_text_contrast=round(minimum, 3))
    return dict(passed=True, palettes=8, scope='All endpoint text colors over every material palette and white/black including active tint; no LCD certification', materials=report)


if __name__ == '__main__':
    Path(sys.argv[1]).write_text(json.dumps(audit(), indent=2)+'\n')
