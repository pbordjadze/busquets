#!/usr/bin/env bash
# Regenerates the SideStore source from this repository's releases (ci/sidestore_source.py,
# ci/sidestore_apps.json) and force-pushes it, with the apps' icons, to the `sidestore` branch, which
# SideStore reads at https://raw.githubusercontent.com/<repo>/sidestore/source.json. Run by ci.yml's
# publish job after each release (from terra-incognita's ci/, NOTICE.md); needs GH_TOKEN and GITHUB_REPOSITORY.
#
#     bash ci/sidestore_publish.sh <build dir> "<commit message>"
set -euo pipefail
DIR="$1"; MSG="$2"
SOURCE="https://raw.githubusercontent.com/${GITHUB_REPOSITORY}/sidestore"
rm -rf "$DIR" && mkdir -p "$DIR"
python3 ci/sidestore_source.py "$SOURCE" > "$DIR/source.json"
python3 - "$DIR" <<'PY'
import json, shutil, sys, os
d = sys.argv[1]
for a in json.load(open('ci/sidestore_apps.json'))['apps']:
    shutil.copyfile(a['iconSource'], os.path.join(d, a['icon']))
PY
printf '# SideStore source\n\nAdd %s/source.json in SideStore (Sources → +). It lists every app built here.\n' "$SOURCE" > "$DIR/README.md"
cd "$DIR"
git init -q && git checkout -q -b sidestore && git add -A
git -c user.name="ci" -c user.email="ci@users.noreply.github.com" commit -qm "$MSG"
git push -qf "https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git" "sidestore:refs/heads/sidestore"
