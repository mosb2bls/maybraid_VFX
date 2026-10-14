#!/usr/bin/env python3
"""Convert flat-background Tanz captures into 128px white silhouette PNGs.

The playground capture uses a magenta clear color. Any pixel that is not the
keyed background becomes an opaque white silhouette, cropped to a square.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from PIL import Image, ImageFilter

DEFAULT_NAMES = (
    "healing_salute.png",
    "left_strafe.png",
    "right_strafe.png",
    "forward_dash.png",
)
# Magenta clear color from characters-playground capture.
KEY = (255, 0, 255)
KEY_DISTANCE = 48
OUTPUT_SIZE = 128
PAD_RATIO = 0.10


def channel_distance(pixel: tuple[int, ...], key: tuple[int, int, int]) -> int:
    return abs(pixel[0] - key[0]) + abs(pixel[1] - key[1]) + abs(pixel[2] - key[2])


def to_silhouette(src: Path, dst: Path, size: int = OUTPUT_SIZE) -> None:
    image = Image.open(src).convert("RGBA")
    width, height = image.size
    pixels = image.load()
    mask = Image.new("L", (width, height), 0)
    mask_px = mask.load()
    min_x, min_y, max_x, max_y = width, height, 0, 0
    for y in range(height):
        for x in range(width):
            if channel_distance(pixels[x, y], KEY) <= KEY_DISTANCE:
                continue
            mask_px[x, y] = 255
            min_x = min(min_x, x)
            min_y = min(min_y, y)
            max_x = max(max_x, x)
            max_y = max(max_y, y)
    if max_x <= min_x or max_y <= min_y:
        raise SystemExit(f"no foreground pixels in {src}")
    mask = mask.filter(ImageFilter.MaxFilter(3)).filter(ImageFilter.MinFilter(3))
    box_w = max_x - min_x + 1
    box_h = max_y - min_y + 1
    pad = int(max(box_w, box_h) * PAD_RATIO)
    crop = (
        max(0, min_x - pad),
        max(0, min_y - pad),
        min(width, max_x + 1 + pad),
        min(height, max_y + 1 + pad),
    )
    cropped = mask.crop(crop)
    side = max(cropped.size)
    square = Image.new("L", (side, side), 0)
    square.paste(cropped, ((side - cropped.size[0]) // 2, (side - cropped.size[1]) // 2))
    square = square.resize((size, size), Image.Resampling.LANCZOS)
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.paste(Image.new("RGBA", (size, size), (255, 255, 255, 255)), mask=square)
    dst.parent.mkdir(parents=True, exist_ok=True)
    out.save(dst)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_dir", type=Path, help="directory of playground screenshots")
    parser.add_argument("out_dir", type=Path, help="directory for 128px silhouette PNGs")
    parser.add_argument("--size", type=int, default=OUTPUT_SIZE)
    args = parser.parse_args()
    missing = [name for name in DEFAULT_NAMES if not (args.raw_dir / name).is_file()]
    if missing:
        raise SystemExit(f"missing captures in {args.raw_dir}: {', '.join(missing)}")
    for name in DEFAULT_NAMES:
        src = args.raw_dir / name
        dst = args.out_dir / name
        to_silhouette(src, dst, size=args.size)
        print(f"wrote {dst}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
