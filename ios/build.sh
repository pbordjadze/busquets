#!/usr/bin/env bash
# Builds one take for the device without an Xcode project (from terra-incognita's toys/ios/build.sh,
# NOTICE.md): its Rust crate as a static archive for aarch64-apple-ios, the shared Swift shell
# (ios/Sources) linked against it, an unsigned .app in Payload/ zipped into <take>.ipa (SideStore signs it
# when installing). The name and bundle id come from takes/<take>/toy.json. macOS with Xcode's
# command-line tools and rustup (`rustup target add aarch64-apple-ios`).
#
#     BUILD=<n> bash ios/build.sh <take> [out-dir]     # <out>/<take>.ipa; version 1.0.<n>, build <n>
#     SIM=1 bash ios/build.sh <take> [out-dir]         # <out>/Payload/<Exec>.app for the iOS Simulator
#                                                      # (ad-hoc signed; tools/sim_launch.sh runs it)
set -euo pipefail
TAKE="${1:?usage: build.sh <take> [out-dir]}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$(python3 -c "import os,sys; print(os.path.abspath(sys.argv[1]))" "${2:-$ROOT/build/$TAKE/ios}")"
BUILD="${BUILD:-1}"
MIN_IOS=17.0
meta() { python3 -c "import json,sys; print(json.load(open('$ROOT/takes/$TAKE/toy.json'))[sys.argv[1]])" "$1"; }
NAME="$(meta name)"; BUNDLE="$(meta bundleIdentifier)"
EXEC="$(echo "$TAKE" | python3 -c "import sys; print(sys.stdin.read().strip().title().replace('-', ''))")"
SIM="${SIM:-0}"
if [ "$SIM" = 1 ]; then
  RTARGET=aarch64-apple-ios-sim; SDKNAME=iphonesimulator; STARGET="arm64-apple-ios$MIN_IOS-simulator"
else
  RTARGET=aarch64-apple-ios; SDKNAME=iphoneos; STARGET="arm64-apple-ios$MIN_IOS"
fi
SDK="$(xcrun --sdk $SDKNAME --show-sdk-path)"
export SDKROOT="$SDK"
mkdir -p "$OUT"
cd "$ROOT"
echo "== rust: $TAKE ($RTARGET, release)"
cargo build --release --target $RTARGET -p "$TAKE" --lib
LIB="target/$RTARGET/release/lib$(echo "$TAKE" | tr - _).a"
ls -l "$LIB"
APP="$OUT/Payload/$EXEC.app"
rm -rf "$OUT/Payload"
mkdir -p "$APP/data"
echo "== swift"
swiftc -O -target $STARGET -sdk "$SDK" \
  -I "$ROOT/ios/Sources" -import-objc-header "$ROOT/ios/Sources/toy.h" \
  -parse-as-library -module-name "$EXEC" \
  "$ROOT/ios/Sources/main.swift" "$LIB" \
  -framework UIKit -framework QuartzCore -framework Metal -framework CoreGraphics -framework Foundation \
  -framework AVFoundation \
  -lc++ -o "$APP/$EXEC"
echo "== bundle"
sed -e "s/TOY_VERSION/1.0.$BUILD/" -e "s/TOY_BUILD/$BUILD/" -e "s/TOY_BUNDLE/$BUNDLE/" \
    -e "s/TOY_EXEC/$EXEC/" \
    -e "s/TOY_NAME/$NAME/g" "$ROOT/ios/Info.plist" > "$APP/Info.plist"
plutil -convert binary1 "$APP/Info.plist"
ICON="$ROOT/takes/$TAKE/icon.png"     # 1024 px, drawn from code (tools/icon.py)
sips -z 120 120 "$ICON" --out "$APP/AppIcon60x60@2x.png" > /dev/null
sips -z 180 180 "$ICON" --out "$APP/AppIcon60x60@3x.png" > /dev/null
sips -z 152 152 "$ICON" --out "$APP/AppIcon76x76@2x.png" > /dev/null
echo "APPL????" > "$APP/PkgInfo"
if [ "$SIM" = 1 ]; then
  # the Simulator runs only signed apps; an ad-hoc signature does
  codesign --force --sign - "$APP" > /dev/null
  du -sh "$APP" | cut -f1 | xargs echo "app (simulator):"
  exit 0
fi
rm -f "$OUT/$TAKE.ipa"
(cd "$OUT" && zip -qry "$TAKE.ipa" Payload)
ls -l "$OUT/$TAKE.ipa"
du -sh "$APP" | cut -f1 | xargs echo "app:"
