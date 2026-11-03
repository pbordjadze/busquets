#!/usr/bin/env python3
"""Draws each take's placeholder icon from code (1024 px, takes/<take>/icon.png): the SideStore source and
the IPA use it (ios/build.sh resizes it). The names and final icons are Paul's.

    python3 tools/icon.py [<take> ...]      # all when none is named
"""
import math, random, sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
N = 1024
RING = [(-300, -260), (300, -260), (-400, 60), (400, 60), (-200, 300), (200, 300)]


def layer():
    return Image.new('RGBA', (N, N), (0, 0, 0, 0))


def blur_over(img, lay, r):
    img.alpha_composite(lay.filter(ImageFilter.GaussianBlur(r)))
    img.alpha_composite(lay)


def pivot():
    img = Image.new('RGBA', (N, N))
    d = ImageDraw.Draw(img)
    for y in range(0, N, 128):
        d.rectangle([0, y, N, y + 64], fill=(28, 74, 40, 255))
        d.rectangle([0, y + 64, N, y + 128], fill=(32, 82, 45, 255))
    c = (N // 2, N // 2)
    lines = layer()
    ld = ImageDraw.Draw(lines)
    for x, y in RING[2:5]:
        ld.line([c, (c[0] + x, c[1] + y)], fill=(255, 220, 130, 200), width=10)
    blur_over(img, lines, 12)
    d = ImageDraw.Draw(img)
    for x, y in RING:
        d.ellipse([c[0] + x - 44, c[1] + y - 44, c[0] + x + 44, c[1] + y + 44], fill=(200, 225, 248, 255))
    for x, y in [(-130, -140), (150, 130)]:
        d.ellipse([c[0] + x - 44, c[1] + y - 44, c[0] + x + 44, c[1] + y + 44], fill=(237, 107, 92, 255))
    d.ellipse([c[0] - 56, c[1] - 56, c[0] + 56, c[1] + 56], fill=(252, 246, 226, 255))
    return img


def pulse():
    img = Image.new('RGBA', (N, N), (8, 9, 22, 255))
    random.seed(3)
    c = (N // 2, N // 2)
    pts = [(c[0] + x, c[1] + y) for x, y in RING]
    th = layer()
    td = ImageDraw.Draw(th)
    for i, p in enumerate(pts):
        q = pts[(i + 1) % len(pts)] if i % 2 == 0 else c
        td.line([p, q], fill=(150, 160, 230, 90), width=4)
    td.line([c, pts[4]], fill=(255, 210, 140, 220), width=8)
    blur_over(img, th, 8)
    dark = layer()
    ImageDraw.Draw(dark).ellipse([620, 280, 820, 480], fill=(0, 0, 0, 230))
    img.alpha_composite(dark.filter(ImageFilter.GaussianBlur(40)))
    st = layer()
    sd = ImageDraw.Draw(st)
    hues = [(255, 200, 115), (190, 220, 255), (255, 160, 110), (230, 240, 190), (255, 215, 165), (180, 205, 255)]
    for p, h in zip(pts, hues):
        sd.ellipse([p[0] - 16, p[1] - 16, p[0] + 16, p[1] + 16], fill=h + (255,))
    sd.ellipse([c[0] - 30, c[1] - 30, c[0] + 30, c[1] + 30], fill=(255, 255, 255, 255))
    blur_over(img, st, 22)
    return img


def garden():
    img = Image.new('RGBA', (N, N), (234, 227, 212, 255))
    random.seed(7)
    c = (N // 2, N // 2)
    pts = [(c[0] + x * 0.9, c[1] + y * 0.9) for x, y in RING] + [c]
    inks = [(34, 46, 82), (115, 62, 36), (56, 92, 77)]
    for k in range(26):
        a, b = random.sample(pts, 2)
        ink = inks[k % 3] if k > 12 else inks[0]
        al = 150 if k % 4 == 0 else 50
        lay = layer()
        ImageDraw.Draw(lay).line([a, b], fill=ink + (al,), width=7 if al > 100 else 4)
        img.alpha_composite(lay.filter(ImageFilter.GaussianBlur(1.2)))
    d = ImageDraw.Draw(img)
    d.ellipse([c[0] - 40, c[1] - 40, c[0] + 40, c[1] + 40], fill=(40, 42, 50, 255))
    return img


def ghost():
    # a night pitch seen low, a figure solid and one a ghost ring
    img = Image.new('RGBA', (N, N), (6, 9, 14, 255))
    d = ImageDraw.Draw(img)
    d.rectangle([0, N * 0.55, N, N], fill=(10, 30, 18, 255))
    d.line([0, N * 0.72, N, N * 0.6], fill=(150, 170, 160, 255), width=6)
    st = layer()
    sd = ImageDraw.Draw(st)
    sd.rounded_rectangle([N * 0.36, N * 0.3, N * 0.5, N * 0.62], radius=40, fill=(250, 212, 140, 255))
    sd.ellipse([N * 0.39, N * 0.2, N * 0.47, N * 0.28], fill=(220, 185, 150, 255))
    sd.rounded_rectangle([N * 0.58, N * 0.36, N * 0.68, N * 0.6], radius=30, outline=(140, 175, 235, 200), width=8)
    blur_over(img, st, 3)
    return img


def light():
    img = Image.new('RGBA', (N, N), (4, 5, 10, 255))
    random.seed(11)
    c = (N * 0.5, N * 0.5)
    hues = [(255, 184, 97), (250, 230, 200), (128, 168, 255)]
    for k in range(70):
        a = (c[0] + random.gauss(0, 140), c[1] + random.gauss(0, 110))
        b = (a[0] + random.gauss(0, 220), a[1] + random.gauss(-60, 160))
        lay = layer()
        ImageDraw.Draw(lay).line([a, b], fill=hues[k % 3] + (90,), width=6)
        img.alpha_composite(lay.filter(ImageFilter.GaussianBlur(3)))
    st = layer()
    ImageDraw.Draw(st).ellipse([c[0] - 22, c[1] - 22, c[0] + 22, c[1] + 22], fill=(255, 250, 240, 255))
    blur_over(img, st, 10)
    return img


TAKES = {'pivot': pivot, 'pulse': pulse, 'garden': garden, 'ghost': ghost, 'light': light}

if __name__ == '__main__':
    for t in sys.argv[1:] or TAKES:
        TAKES[t]().convert('RGB').save(ROOT / 'takes' / t / 'icon.png')
        print('takes/%s/icon.png' % t)
