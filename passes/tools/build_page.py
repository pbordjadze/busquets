"""Build the passes page: passes/page.html with StatsBomb's passes and logo filled in.

Usage: python3 passes/tools/build_page.py <dir with passes.json> <out.html>
The output carries StatsBomb data, so it is written outside the repository's tracked files.
"""
import base64, io, json, struct, sys, os, urllib.request
from PIL import Image

src, out = sys.argv[1], sys.argv[2]
here = os.path.dirname(os.path.abspath(__file__))
matches = json.load(open(os.path.join(src, "passes.json")))
buf = bytearray()
q = lambda v, s: max(0, min(255, round(v * s)))
for m in matches:
    for i, (x0, y0, x1, y1, t, bad, _per) in enumerate(m["passes"]):
        flags = (1 if bad else 0) | (2 if i == 0 else 0)
        buf += bytes([q(x0, 2), q(y0, 3), q(x1, 2), q(y1, 3)]) + struct.pack("<H", min(int(t), 65535)) + bytes([flags])
logo_url = "https://raw.githubusercontent.com/statsbomb/open-data/master/img/SB%20-%20Icon%20Lockup%20-%20Colour%20positive.png"
im = Image.open(io.BytesIO(urllib.request.urlopen(logo_url, timeout=60).read())).convert("RGBA")
im = im.resize((480, round(480 * im.height / im.width)), Image.LANCZOS)
png = io.BytesIO(); im.save(png, "PNG", optimize=True)
html = open(os.path.join(here, "..", "page.html")).read()
html = html.replace("__PASSES__", base64.b64encode(bytes(buf)).decode())
html = html.replace("__LOGO__", "data:image/png;base64," + base64.b64encode(png.getvalue()).decode())
open(out, "w").write(html)
print(f"{len(buf)//7} passes, {len(html)//1024} KB -> {out}")
