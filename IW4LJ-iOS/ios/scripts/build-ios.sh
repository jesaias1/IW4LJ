#!/usr/bin/env bash
# Build IW4LJ for iOS. Runs on macOS with Xcode installed (a Mac, or the GitHub
# Actions workflow in .github/workflows/ios.yml). It cannot run on Windows.
#
#   ios/scripts/build-ios.sh                 # device build -> target/ios/IW4LJ.app + IW4LJ.ipa (unsigned)
#   ios/scripts/build-ios.sh sim             # simulator build -> target/ios-sim/IW4LJ.app
#   ios/scripts/build-ios.sh device --debug  # unoptimized (slow in game, quick to build)
#
# Signing: the device build is left unsigned, because the usual route from a
# Windows PC is Sideloadly or AltStore, which sign with your Apple ID when they
# install it. To sign here instead:
#   CODESIGN_IDENTITY="Apple Development: you (TEAMID)" \
#   PROVISIONING_PROFILE=/path/to/profile.mobileprovision \
#   ENTITLEMENTS=ios/app/Entitlements-paid.plist ios/scripts/build-ios.sh
set -euo pipefail

kind="${1:-device}"
shift || true
profile="play"
profile_dir="play"
for arg in "$@"; do
    case "$arg" in
        --debug) profile="dev"; profile_dir="debug" ;;
        --release) profile="release"; profile_dir="release" ;;
        *) echo "unknown option: $arg" >&2; exit 2 ;;
    esac
done

case "$kind" in
    device) triple="aarch64-apple-ios"; out_name="ios" ;;
    sim) triple="aarch64-apple-ios-sim"; out_name="ios-sim" ;;
    sim-x86) triple="x86_64-apple-ios"; out_name="ios-sim" ;;
    *) echo "usage: $0 [device|sim|sim-x86] [--debug|--release]" >&2; exit 2 ;;
esac

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
app_src="$root/ios/app"
out="$root/target/$out_name"
app="$out/IW4LJ.app"

export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-16.0}"

rustup target add "$triple"

if [ "$profile" = "dev" ]; then
    cargo build -p iw4l-ios --bin iw4lj --target "$triple"
else
    cargo build -p iw4l-ios --bin iw4lj --target "$triple" --profile "$profile"
fi
binary="$root/target/$triple/$profile_dir/iw4lj"

rm -rf "$app"
mkdir -p "$app"
cp "$binary" "$app/iw4lj"
cp "$app_src/Info.plist" "$app/Info.plist"

# The icon: an asset catalog compiled to Assets.car, as Xcode would.
xcrun actool "$app_src/Assets.xcassets" \
    --compile "$app" \
    --platform "$([ "$kind" = device ] && echo iphoneos || echo iphonesimulator)" \
    --minimum-deployment-target "$IPHONEOS_DEPLOYMENT_TARGET" \
    --app-icon AppIcon \
    --output-partial-info-plist "$out/assetcatalog.plist" \
    --target-device iphone --target-device ipad >/dev/null
/usr/libexec/PlistBuddy -c "Merge $out/assetcatalog.plist" "$app/Info.plist" || true

entitlements="${ENTITLEMENTS:-$app_src/Entitlements.plist}"

if [ -n "${CODESIGN_IDENTITY:-}" ]; then
    [ -n "${PROVISIONING_PROFILE:-}" ] && cp "$PROVISIONING_PROFILE" "$app/embedded.mobileprovision"
    codesign --force --sign "$CODESIGN_IDENTITY" --entitlements "$entitlements" "$app"
elif [ "$kind" != device ]; then
    # The simulator wants a signature, and an ad-hoc one is enough.
    codesign --force --sign - --entitlements "$entitlements" "$app"
fi

if [ "$kind" = device ]; then
    rm -rf "$out/Payload" "$out/IW4LJ.ipa"
    mkdir -p "$out/Payload"
    cp -R "$app" "$out/Payload/"
    (cd "$out" && zip -qry IW4LJ.ipa Payload)
    rm -rf "$out/Payload"
    echo "built: $out/IW4LJ.ipa"
    if [ -z "${CODESIGN_IDENTITY:-}" ]; then
        echo "unsigned: install it with Sideloadly or AltStore (see ios/docs/IOS-DEVICE.md)"
    fi
else
    echo "built: $app"
    echo "run it with ios/scripts/run-simulator.sh"
fi
