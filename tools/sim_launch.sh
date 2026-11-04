#!/usr/bin/env bash
# A take opened in the iOS Simulator (from terra-incognita's tools/toys/sim_launch.sh, NOTICE.md) with Metal's API validation on: the launch a Linux check can't
# make. The toy's shaders go through Apple's own Metal compiler and its every Metal call through
# Apple's validation layer (MTL_DEBUG_LAYER), which aborts on a misuse the phone's driver might crash
# on, naming it. Then it is left running and is asked whether it is still alive.
#
#     bash tools/sim_launch.sh <take> [out-dir]      # macOS with Xcode and rustup's aarch64-apple-ios-sim
#
# It builds the toy for the Simulator (SIM=1 ios/build.sh), boots an iPhone, installs and launches
# it, waits SECONDS_ALIVE (default 25) and fails if the toy is gone or logged a GPU error. <out-dir> keeps what it saw: the toy's
# own toy-log.txt (the kit's launch steps, its GPU errors), its stdout and stderr (Metal's validation
# messages), any crash report, and a screenshot. What it can't vouch for: the phone's memory limit
# (the Simulator has none of its own) and the phone's GPU (the Mac's draws for it): toy-log.txt from a
# phone stays the last word.
set -euo pipefail
TOY="${1:?usage: sim_launch.sh <toy> [out-dir]}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$(python3 -c "import os,sys; print(os.path.abspath(sys.argv[1]))" "${2:-$ROOT/build/$TOY/sim}")"
WAIT="${SECONDS_ALIVE:-25}"
mkdir -p "$OUT"
trap 'rm -rf "$OUT/build"' EXIT     # (the built app: only what was seen is kept)
SIM=1 bash "$ROOT/ios/build.sh" "$TOY" "$OUT/build" | tail -5
APP="$(ls -d "$OUT"/build/Payload/*.app)"
EXEC="$(basename "$APP" .app)"
BUNDLE="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['bundleIdentifier'])" "$ROOT/takes/$TOY/toy.json")"

# an iPhone (the newest runtime's first iPhone), booted once and reused by the next toy in this job
UDID="$(xcrun simctl list devices available -j | python3 -c '
import json, sys
d = json.load(sys.stdin)["devices"]
runtimes = sorted((r for r in d if "iOS" in r), key=lambda r: [int(x) for x in r.rsplit("iOS-", 1)[-1].split("-") if x.isdigit()])
for r in reversed(runtimes):
    phones = [x for x in d[r] if x["name"].startswith("iPhone")]
    booted = [x for x in phones if x["state"] == "Booted"]
    if booted or phones:
        print((booted or phones)[0]["udid"]); break
')"
[ -n "$UDID" ] || { echo "no iPhone simulator"; exit 1; }
xcrun simctl list devices | grep "$UDID" | sed 's/^ */simulator: /'
xcrun simctl boot "$UDID" 2> /dev/null || true
xcrun simctl bootstatus "$UDID" -b > /dev/null
xcrun simctl uninstall "$UDID" "$BUNDLE" 2> /dev/null || true
xcrun simctl install "$UDID" "$APP"

# Metal's validation layer and the shell's environment for the toy (SIMCTL_CHILD_ passes it through)
export SIMCTL_CHILD_MTL_DEBUG_LAYER=1
export SIMCTL_CHILD_MTL_DEBUG_LAYER_ERROR_MODE=assert
export SIMCTL_CHILD_RUST_BACKTRACE=1
before="$(ls ~/Library/Logs/DiagnosticReports 2> /dev/null | grep -c "$EXEC" || true)"
xcrun simctl launch --terminate-running-process --stdout="$OUT/stdout.txt" --stderr="$OUT/stderr.txt" "$UDID" "$BUNDLE"
alive=1
for i in $(seq 1 "$WAIT"); do
  sleep 1
  # (a Simulator app is a process of the Mac's own: its executable's path names it)
  if ! pgrep -f "/$EXEC.app/$EXEC" > /dev/null; then alive=0; echo "gone after ${i} s"; break; fi
done
xcrun simctl io "$UDID" screenshot "$OUT/screen.png" > /dev/null 2>&1 || true
DATA="$(xcrun simctl get_app_container "$UDID" "$BUNDLE" data 2> /dev/null || true)"
[ -n "$DATA" ] && cp "$DATA/Documents/toy-log.txt" "$OUT/toy-log.txt" 2> /dev/null || true
ls -t ~/Library/Logs/DiagnosticReports 2> /dev/null | grep "$EXEC" | head -n $(( $(ls ~/Library/Logs/DiagnosticReports 2> /dev/null | grep -c "$EXEC" || true) - before )) |
  while read -r f; do cp ~/Library/Logs/DiagnosticReports/"$f" "$OUT/"; echo "crash report: $f"; done || true
xcrun simctl terminate "$UDID" "$BUNDLE" 2> /dev/null || true
echo "== toy-log.txt"; cat "$OUT/toy-log.txt" 2> /dev/null || echo "(none)"
# a GPU error the toy only logged (a pipeline Apple's compiler refused draws nothing on the phone)
if grep -q "gpu: " "$OUT/toy-log.txt" 2> /dev/null && grep -q -E "gpu: .*(Error|error|lost)" "$OUT/toy-log.txt"; then
  echo "FAIL  $TOY logged a GPU error in the Simulator (toy-log.txt above)"; gpu_error=1
fi
echo "== stderr (last 40 lines)"; tail -40 "$OUT/stderr.txt" 2> /dev/null || true
if [ "${gpu_error:-0}" = 1 ]; then
  exit 1
elif [ "$alive" = 1 ]; then
  echo "ok    $TOY alive in the Simulator after ${WAIT} s, Metal validation on"
else
  echo "FAIL  $TOY closed in the Simulator (Metal validation on): toy-log.txt and stderr above say where"
  exit 1
fi
