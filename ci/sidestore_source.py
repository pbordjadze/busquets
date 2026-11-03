#!/usr/bin/env python3
"""Writes the SideStore/AltStore source (JSON) listing this repository's apps and their latest builds.

    ci/sidestore_source.py <icon-url-base> > source.json

The apps are ci/sidestore_apps.json's; each one's versions come from the releases tagged
`<tag>-<n>` that carry an .ipa asset (newest first, KEEP of them). Needs `gh` with GH_TOKEN and
GITHUB_REPOSITORY. ci.yml's publish job runs it after a release and pushes the result to the `sidestore`
branch with the icons beside it. (From terra-incognita's ci/, NOTICE.md.)
"""
import json
import os
import subprocess
import sys

# Builds listed per app; the ipa jobs keep this many releases of each (their `tail -n +6`).
KEEP = 5

icon_base = sys.argv[1].rstrip("/")
repo = os.environ["GITHUB_REPOSITORY"]
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
spec = json.load(open(os.path.join(root, "ci", "sidestore_apps.json")))

releases = json.loads(subprocess.run(
    ["gh", "api", f"repos/{repo}/releases?per_page=100"], check=True, capture_output=True, text=True).stdout)

apps = []
for a in spec["apps"]:
    prefix = a["tag"] + "-"
    versions = []
    for r in releases:
        tag = r["tag_name"]
        ipa = next((x for x in r["assets"] if x["name"].endswith(".ipa")), None)
        if not tag.startswith(prefix) or not tag[len(prefix):].isdigit() or ipa is None:
            continue
        build = tag[len(prefix):]
        versions.append({
            "version": f"1.0.{build}",
            "buildVersion": build,
            "date": r["published_at"] or r["created_at"],
            "localizedDescription": (r["body"] or "").strip() or f"Build {build}",
            "downloadURL": ipa["browser_download_url"],
            "size": ipa["size"],
            "minOSVersion": a["minOSVersion"],
        })
    versions.sort(key=lambda v: int(v["buildVersion"]), reverse=True)
    versions = versions[:KEEP]
    if not versions:
        print(f"no {prefix}* release with an .ipa asset: {a['name']} left out", file=sys.stderr)
        continue
    latest = versions[0]
    apps.append({
        "name": a["name"],
        "bundleIdentifier": a["bundleIdentifier"],
        "developerName": repo.split("/")[0],
        "subtitle": a["subtitle"],
        "localizedDescription": a["localizedDescription"],
        "iconURL": f"{icon_base}/{a['icon']}",
        "tintColor": a["tintColor"],
        "category": "games",
        "versions": versions,
        # Legacy fields for clients that predate `versions`.
        "version": latest["version"],
        "versionDate": latest["date"],
        "versionDescription": latest["localizedDescription"],
        "downloadURL": latest["downloadURL"],
        "size": latest["size"],
        "appPermissions": {"entitlements": [], "privacy": {}},
    })
if not apps:
    sys.exit("no app has a release with an .ipa asset")

source = {
    "name": "Busquets sketches (CI builds)",
    "identifier": "com.pbordjadze.busquets.source",
    "subtitle": "Sketches of the pivot's loop: receive, scan, decide, release",
    "iconURL": f"{icon_base}/{spec['apps'][0]['icon']}",
    "website": f"https://github.com/{repo}",
    "tintColor": "#E8C877",
    "apps": apps,
    "news": [],
}
json.dump(source, sys.stdout, indent=2)
print()
