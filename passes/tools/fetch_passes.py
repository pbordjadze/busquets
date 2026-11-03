"""Fetch Sergio Busquets' passes from StatsBomb open data (https://github.com/statsbomb/open-data).

The data is StatsBomb's and its user agreement forbids redistributing it, so nothing this writes is
committed: it fetches at build time into a directory you name (default: build/passes) and writes
passes.json there, a compact list the page draws. Run: python3 passes/tools/fetch_passes.py [outdir]
"""
import json, os, sys, urllib.request
from concurrent.futures import ThreadPoolExecutor

BASE = "https://raw.githubusercontent.com/statsbomb/open-data/master/data/"
TEAMS = {"Barcelona", "Spain", "Inter Miami"}
COMPS = {"La Liga", "Champions League", "FIFA World Cup", "UEFA Euro", "Major League Soccer", "Copa del Rey"}

def get(path, cache):
    f = os.path.join(cache, path.replace("/", "_"))
    if os.path.exists(f):
        return json.load(open(f))
    with urllib.request.urlopen(BASE + path, timeout=120) as r:
        data = r.read()
    open(f, "wb").write(data)
    return json.loads(data)

def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "build/passes"
    cache = os.path.join(out, "cache"); os.makedirs(cache, exist_ok=True)
    seasons = [c for c in get("competitions.json", cache)
               if c["competition_name"] in COMPS and c["season_name"][:4] >= "2008" and c["competition_gender"] == "male"]
    matches = []
    for c in seasons:
        for m in get(f"matches/{c['competition_id']}/{c['season_id']}.json", cache):
            if m["home_team"]["home_team_name"] in TEAMS or m["away_team"]["away_team_name"] in TEAMS:
                matches.append((m["match_date"], m["match_id"], c["competition_name"]))
    matches.sort()
    print(len(matches), "candidate matches", file=sys.stderr)

    def one(m):
        date, mid, comp = m
        ev = get(f"events/{mid}.json", cache)
        ps = []
        for e in ev:
            if e["type"]["name"] != "Pass" or "Busquets" not in e.get("player", {}).get("name", ""):
                continue
            if "Sergio" not in e["player"]["name"]:
                continue
            p = e["pass"]
            ps.append([round(e["location"][0], 1), round(e["location"][1], 1),
                       round(p["end_location"][0], 1), round(p["end_location"][1], 1),
                       e["minute"] * 60 + e["second"] + (2700 if e["period"] == 2 else 0) * 0,
                       0 if "outcome" not in p else 1, e["period"]])
        os.remove(os.path.join(cache, f"events_{mid}.json"))
        return {"date": date, "id": mid, "comp": comp, "passes": ps}

    with ThreadPoolExecutor(12) as ex:
        res = [r for r in ex.map(one, matches) if r["passes"]]
    print(len(res), "matches with Busquets,", sum(len(r["passes"]) for r in res), "passes", file=sys.stderr)
    json.dump(res, open(os.path.join(out, "passes.json"), "w"), separators=(",", ":"))

main()
