#!/usr/bin/env python3
"""Render assets/l-gpui-256.png to match assets/l-gpui.svg (needs Pillow)."""
from PIL import Image, ImageDraw

S = 4  # supersampling factor
W = 256 * S


def p(v):
    return int(v * S)


img = Image.new("RGBA", (W, W), (0, 0, 0, 0))
# Diagonal gradient tile.
grad = Image.new("RGBA", (W, W))
a, b = (0x4D, 0x85, 0xFF), (0x24, 0x5B, 0xDB)
gp = grad.load()
for y in range(W):
    for x in range(W):
        t = (x + y) / (2 * (W - 1))
        gp[x, y] = tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3)) + (255,)
mask = Image.new("L", (W, W), 0)
ImageDraw.Draw(mask).rounded_rectangle([p(8), p(8), p(248), p(248)], radius=p(56), fill=255)
img.paste(grad, (0, 0), mask)

d = ImageDraw.Draw(img)
# Speech bubble.
d.rounded_rectangle([p(40), p(72), p(216), p(184)], radius=p(24), fill="white")
d.polygon([(p(84), p(180)), (p(120), p(184)), (p(84), p(212))], fill="white")
# "L" stroke.
blue = (0x33, 0x70, 0xFF, 255)
w = p(18)
d.line([(p(100), p(100)), (p(100), p(160)), (p(152), p(160))], fill=blue, width=w, joint="curve")
for cx, cy in [(100, 100), (152, 160), (100, 160)]:
    d.ellipse([p(cx) - w // 2, p(cy) - w // 2, p(cx) + w // 2, p(cy) + w // 2], fill=blue)

img.resize((256, 256), Image.LANCZOS).save("assets/l-gpui-256.png")
print("wrote assets/l-gpui-256.png")
