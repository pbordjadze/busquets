#!/usr/bin/env bash
# Makes a played plan's film watchable: an MP4 of its frames (with its audio.wav if there is one) and a
# contact sheet of every Nth frame, for a critic or a session to look at without a device.
#
#     bash tools/toys/film.sh <out-dir> [fps=10] [every=5]    # <out>/film.mp4, <out>/sheet.png
set -euo pipefail
OUT="$1"; FPS="${2:-10}"; EVERY="${3:-5}"
[ -d "$OUT/film" ] || { echo "no $OUT/film (the plan needs \"film\")"; exit 1; }
AUDIO=()
[ -f "$OUT/audio.wav" ] && AUDIO=(-i "$OUT/audio.wav" -c:a aac -shortest)
ffmpeg -loglevel error -y -framerate "$FPS" -i "$OUT/film/%04d.png" "${AUDIO[@]}" \
  -vf "scale='min(1080,iw)':-2" -c:v libx264 -pix_fmt yuv420p "$OUT/film.mp4"
ffmpeg -loglevel error -y -i "$OUT/film/%04d.png" -vf "select='not(mod(n\,$EVERY))',scale=360:-2,tile=6x3" -frames:v 1 "$OUT/sheet.png"
ls -l "$OUT/film.mp4" "$OUT/sheet.png"
