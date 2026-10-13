#!/usr/bin/env bash
# Build the simulator app, install it into a booted iOS simulator and start it.
# macOS + Xcode only. The simulator is the one place the game can be tried on a
# Mac without an iPhone; its GPU is the Mac's, so frame rates mean nothing.
#
#   ios/scripts/run-simulator.sh                 # uses/boots "iPhone 15"
#   SIMULATOR="iPhone 15 Pro" ios/scripts/run-simulator.sh
#   GAMES=/path/to/mw2 ios/scripts/run-simulator.sh   # symlink a game install in
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
device="${SIMULATOR:-iPhone 15}"
arch_kind="sim"
[ "$(uname -m)" = "x86_64" ] && arch_kind="sim-x86"

"$root/ios/scripts/build-ios.sh" "$arch_kind"

xcrun simctl boot "$device" 2>/dev/null || true
open -a Simulator
xcrun simctl bootstatus "$device" -b
xcrun simctl install "$device" "$root/target/ios-sim/IW4LJ.app"

if [ -n "${GAMES:-}" ]; then
    data="$(xcrun simctl get_app_container "$device" dev.iw4j.iw4l data)"
    mkdir -p "$data/Documents"
    rm -rf "$data/Documents/mw2"
    ln -s "$GAMES" "$data/Documents/mw2"
    echo "linked $GAMES into the simulator's Documents/mw2"
fi

xcrun simctl launch --console-pty "$device" dev.iw4j.iw4l
