#!/usr/bin/env python3
"""The device shell's text on the Linux check's screenshots: a stand-in for ios/Sources/main.swift (from terra-incognita's tools/kit/shell_text.py).

The info mark's line and the rules (toy-kit's info.rs) are set by the iOS shell in the system's text,
so a check's frame lacks them; the check writes what was showing beside each shot as
`<shot>.shell.json` (size, ppp, line, rules). This draws them as the shell lays them out, into
`<shot>.shell.png`, so a screenshot shows what the reader sees:

    python3 tools/shell_text.py <check out dir> [...]     # every shot with a .shell.json under it

Stand-ins, not the device: Inter for SF (close in width), the system's default text size (callout
16 pt, footnote 13 pt), an iPhone's 34-point bottom safe area, and a blur with a dark tint for
UIKit's ultra-thin dark material. The layout numbers are main.swift's: change both together.
"""
import json, sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter, ImageFont

FONT = '/usr/share/fonts/opentype/inter/Inter-Regular.otf'
SAFE_BOTTOM, SAFE_TOP = 34.0, 47.0


def wrap(draw, text, font, width):
    out = []
    for para in text.split('\n'):
        line = ''
        for word in para.split(' '):
            t = (line + ' ' + word).strip()
            if draw.textlength(t, font=font) <= width or not line:
                line = t
            else:
                out.append(line)
                line = word
        out.append(line)
    return out


def rules(img, text, k):
    W, H = img.size[0] / k, img.size[1] / k
    w = min(W - 32, 480)
    pad = (14, 16)
    body = ImageFont.truetype(FONT, round(16 * k))
    note = ImageFont.truetype(FONT, round(13 * k))
    d = ImageDraw.Draw(img)
    parts = text.split('\n\n')
    rows = []  # (text, font, alpha, space before in points, line height)
    for i, p in enumerate(parts[0].split('\n')):
        for j, l in enumerate(wrap(d, p, body, (w - 2 * pad[1]) * k)):
            rows.append((l, body, 0.95, 7 if i and not j else 0, 21))
    for i, l in enumerate(wrap(d, '\n'.join(parts[1:]), note, (w - 2 * pad[1]) * k)):
        rows.append((l, note, 0.64, 13 if not i else 0, 18))
    th = sum(r[3] + r[4] for r in rows)
    foot = H - 60
    h = min(th + 2 * pad[0], foot - SAFE_TOP - 8)
    x0, y0 = (W - w) / 2, foot - h
    box = tuple(round(v * k) for v in (x0, y0, x0 + w, y0 + h))
    glass = img.crop(box).filter(ImageFilter.GaussianBlur(14 * k))
    glass = Image.blend(glass, Image.new("RGB", glass.size, (15, 15, 17)), 0.75)  # (material + the 0.62 tint)
    mask = Image.new('L', glass.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, glass.size[0] - 1, glass.size[1] - 1), radius=round(16 * k), fill=255)
    img.paste(glass, box[:2], mask)
    d = ImageDraw.Draw(img, 'RGBA')
    y = y0 + pad[0]
    for t, f, a, before, lh in rows:
        y += before
        if y + lh > y0 + h:
            break
        d.text(((x0 + pad[1]) * k, (y + lh * 0.12) * k), t, font=f, fill=(255, 255, 255, round(255 * a)))
        y += lh


def caption(img, text, k):
    W, H = img.size[0] / k, img.size[1] / k
    w = min(W - 48, 560)
    f = ImageFont.truetype(FONT, round(16 * k))
    d = ImageDraw.Draw(img, 'RGBA')
    tw = d.textlength(text, font=f)
    if tw > w * k:  # (adjustsFontSizeToFitWidth, down to 0.7)
        f = ImageFont.truetype(FONT, round(16 * k * max(0.7, w * k / tw)))
        tw = d.textlength(text, font=f)
    x, y = (W * k - tw) / 2, (H - SAFE_BOTTOM - 96 + 22 - 10) * k
    shadow = Image.new('RGBA', img.size, (0, 0, 0, 0))
    ImageDraw.Draw(shadow).text((x, y), text, font=f, fill=(0, 0, 0, 230))
    img.paste(Image.new('RGB', img.size, 0), (0, 0), shadow.filter(ImageFilter.GaussianBlur(4 * k)).split()[3])
    ImageDraw.Draw(img, 'RGBA').text((x, y), text, font=f, fill=(255, 255, 255, 235))


def main():
    for root in sys.argv[1:] or ['.']:
        for j in sorted(Path(root).rglob('*.shell.json')):
            s = json.loads(j.read_text())
            shot = j.with_name(j.name[: -len('.shell.json')] + '.png')
            img = Image.open(shot).convert('RGB')
            if s.get('rules'):
                rules(img, s['rules'], s['ppp'])
            if s.get('line'):
                caption(img, s['line'], s['ppp'])
            out = j.with_name(j.name[: -len('.shell.json')] + '.shell.png')
            img.save(out)
            print(out)


if __name__ == '__main__':
    main()
