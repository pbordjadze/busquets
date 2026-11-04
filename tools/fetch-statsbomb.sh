#!/usr/bin/env bash
# Fetches StatsBomb open data (events, and 360 frames where they exist) for the given match ids into
# data/statsbomb (outside git: StatsBomb's user agreement does not allow redistribution; NOTICE.md).
# The headless checks of the takes that read real matches (ghost, light) read it from there
# (BUSQUETS_DATA, default data/statsbomb). With no ids: the matches the takes and examples use.
#
#     bash tools/fetch-statsbomb.sh               # the takes' matches
#     bash tools/fetch-statsbomb.sh 3794686 ...   # those
#     bash tools/fetch-statsbomb.sh --busquets    # every open match Busquets played with 360 frames (45; ~550 MB)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${BUSQUETS_DATA:-$ROOT/data/statsbomb}"
BASE=https://raw.githubusercontent.com/statsbomb/open-data/master/data
mkdir -p "$OUT/events" "$OUT/three-sixty" "$OUT/matches"
ids=("$@")
if [ ${#ids[@]} -eq 0 ]; then ids=(3794686 3795220 3857291 3773585 3773597 3869220); fi
if [ "${ids[0]}" = "--busquets" ]; then
  ids=()
  for cs in 55/43 43/106 11/90; do
    curl -fsSL --retry 4 --retry-all-errors --retry-delay 5 "$BASE/matches/$cs.json" -o "$OUT/matches/${cs/\//_}.json"
  done
  mapfile -t ids < <(python3 -I - "$OUT/matches" <<'PY'
import json, glob, sys
for f in sorted(glob.glob(sys.argv[1] + '/*.json')):
    for m in json.load(open(f)):
        if {'Spain', 'Barcelona'} & {m['home_team']['home_team_name'], m['away_team']['away_team_name']}:
            print(m['match_id'])
PY
)
fi
for id in "${ids[@]}"; do
  [ -s "$OUT/events/$id.json" ] || curl -fsSL --retry 4 --retry-all-errors --retry-delay 5 "$BASE/events/$id.json" -o "$OUT/events/$id.json"
  [ -s "$OUT/three-sixty/$id.json" ] || curl -fsSL --retry 4 --retry-all-errors --retry-delay 5 "$BASE/three-sixty/$id.json" -o "$OUT/three-sixty/$id.json" || rm -f "$OUT/three-sixty/$id.json"
done
echo "statsbomb: ${#ids[@]} matches in $OUT"
