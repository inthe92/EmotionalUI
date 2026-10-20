#!/usr/bin/env python3
"""Original deterministic technical placeholder. No input images; GPL-3.0-only."""
import argparse
import math
from pathlib import Path
import struct
import zlib


def render():
    width, height = 640, 480
    pixels = bytearray(bytes((31, 37, 36)) * width * height)

    def rect(x, y, w, h, color):
        for row in range(max(0, y), min(height, y + h)):
            start = (row * width + max(0, x)) * 3
            pixels[start:start + min(w, width - max(0, x)) * 3] = bytes(color) * min(w, width - max(0, x))

    rect(0, 340, 640, 140, (46, 39, 32))
    rect(322, 92, 202, 232, (60, 48, 37))
    for y in range(104, 312):
        t = (y - 104) / 208
        rect(334, y, 178, 1, (round(130 - t * 55), round(126 - t * 42), round(108 - t * 30)))
    for x in range(334, 512):
        ridge = round(230 + 18 * math.sin((x - 334) / 34) - 16 * math.cos((x - 334) / 19))
        rect(x, ridge, 1, 312 - ridge, (53, 70, 67))
    rect(420, 104, 6, 208, (56, 47, 38))
    rect(334, 208, 178, 5, (56, 47, 38))
    rect(396, 370, 230, 15, (79, 57, 38))
    rect(412, 385, 10, 80, (50, 40, 31))
    rect(604, 385, 10, 80, (50, 40, 31))
    rect(552, 312, 36, 58, (102, 76, 46))
    rect(558, 320, 24, 42, (198, 154, 90))
    rect(0, 422, 310, 58, (44, 43, 39))
    scanlines = b''.join(b'\0' + pixels[y * width * 3:(y + 1) * width * 3] for y in range(height))

    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff)

    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(scanlines, 9)) + chunk(b'IEND', b''))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    output = parser.parse_args().output
    data = render()
    if output.exists():
        if output.read_bytes() != data:
            raise SystemExit('Refusing to overwrite a different resource')
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(data)
