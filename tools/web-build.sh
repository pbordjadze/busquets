#!/usr/bin/env bash
# The web build: every take in one wasm (crates/web), its bindings, a page a take (web/take.html) and
# the index (web/index.html), into one directory any static server serves (GitHub Pages does, from CI).
#
#     bash tools/web-build.sh [out-dir]              # default build/web
#     python3 -m http.server -d build/web 8000       # then http://localhost:8000/pivot/
#
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli 0.2.130 (the crate pins the same).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$(python3 -c "import os,sys; print(os.path.abspath(sys.argv[1]))" "${1:-$ROOT/build/web}")"
cd "$ROOT"
cargo build --release --target wasm32-unknown-unknown -p busquets-web
mkdir -p "$OUT"
wasm-bindgen --target web --no-typescript --out-dir "$OUT" target/wasm32-unknown-unknown/release/busquets_web.wasm
if command -v wasm-opt > /dev/null 2>&1; then wasm-opt -O3 -o "$OUT/busquets_web_bg.wasm" "$OUT/busquets_web_bg.wasm"; fi
python3 - "$OUT" <<'PY'
import json, sys, pathlib
out = pathlib.Path(sys.argv[1])
tpl = open('web/take.html').read()
for take in json.load(open('web/takes.json')):
    d = out / take['id']
    d.mkdir(parents=True, exist_ok=True)
    (d / 'index.html').write_text(tpl.replace('TAKE_ID', take['id']).replace('TAKE_TITLE', take['title']).replace('TAKE_BG', take['bg']))
PY
cp web/index.html "$OUT/index.html"
touch "$OUT/.nojekyll"
du -sh "$OUT" | cut -f1 | xargs echo "web build in $OUT:"
