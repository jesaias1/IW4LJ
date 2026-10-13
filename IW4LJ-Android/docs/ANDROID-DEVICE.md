# Testing IW4L on an Android device

The Android build is a `cdylib` the platform loads (`crates/android`). It reads
game data from the first of these that holds a `zone/` directory, and writes its
caches under the app's private files directory:

```
/sdcard/Android/media/dev.iw4j.iw4l/mw2     game install, drag-in over USB (preferred)
/sdcard/Android/data/dev.iw4j.iw4l/files/mw2  game install, `adb push`
/sdcard/Android/data/dev.iw4j.iw4l/files/iw4l-artifacts  caches, logs, demos
```

`Android/media` is used first because on Android 11+ a file manager or an MTP
connection cannot write to `Android/data`, while `Android/media/<pkg>` is shared
and writable. The app creates that directory on first run.

## Build

The Android toolchain and paths used on the build machine are described in
`docs/ANDROID.md`. From the repository root, with the SDK/NDK environment set:

```sh
cargo apk build -p iw4l-android --target aarch64-linux-android
# release (smaller, faster): add --release
```

The APK lands at `target/<profile>/apk/iw4l.apk`.

## Install

```sh
adb install -r target/debug/apk/iw4l.apk
```

The `min_sdk` is 31 (Android 12+); the ABI is `arm64-v8a` only.

## Provide the game data

IW4L ships no game assets. Copy a **Modern Warfare 2 multiplayer** install to the
device so a `mw2/` directory holds a `zone/` with `common_mp.ff` and the
`mp_*.ff` maps, and the IWDs the menu reads under `main/`.

**Drag-in (no adb).** Run the app once so it creates
`/sdcard/Android/media/dev.iw4j.iw4l/`, then copy the `mw2` folder into it over
USB/MTP or a file manager:

```
/sdcard/Android/media/dev.iw4j.iw4l/mw2/zone/...
/sdcard/Android/media/dev.iw4j.iw4l/mw2/main/...
```

**Over USB debugging.** Both locations work; the app prefers whichever already
has `zone/`:

```sh
adb shell mkdir -p /sdcard/Android/data/dev.iw4j.iw4l/files/mw2
adb push "C:/Games/iw4x/zone" /sdcard/Android/data/dev.iw4j.iw4l/files/mw2/zone
adb push "C:/Games/iw4x/main" /sdcard/Android/data/dev.iw4j.iw4l/files/mw2/main
```

This is multi-GB either way. The menu refuses with a clear message naming the
folder it searched if the data is missing or incomplete.

## Run and read the log

```sh
adb shell am start -n dev.iw4j.iw4l/android.app.NativeActivity
adb logcat -s iw4l:* RustStdoutStderr:* AndroidRuntime:E
```

`diag` writes a log beside the artifacts as well:

```sh
adb shell cat /sdcard/Android/data/dev.iw4j.iw4l/files/iw4l-artifacts/logs/*.log
```

## What to look for

- **Black screen, no window**: the wgpu device failed to create. The usual cause
  is requesting a feature the GPU lacks — `TEXTURE_COMPRESSION_BC` is already
  dropped on Android; if the log names another requested feature, the request in
  `crates/bootstrap/src/plugins.rs` must shrink to the adapter's set.
- **Window, then an error naming a folder**: the game data is not where the app
  looks; see the push step.
- **Menu appears**: expect the touch pad overlay. The stick is low-left; the
  right-hand buttons are fire/ADS/jump/crouch/etc.; the left d-pad and Enter
  drive menus.
- **Textures are wrong-coloured or tiled noise**: a BC decode or format mapping
  problem in `asset_material`.

## Input

Touch synthesizes the keyboard and mouse the runtime already reads
(`crates/touch`): W/A/S/D for the stick, the default-bound keys for the buttons
(`console/assets/default_controls.cfg`). A Bluetooth/USB gamepad also works
through the existing gamepad path.

The `MODS` button opens an overlay that runs console lines without a keyboard:
unlock all, reset level, add bots, start match, spawn. Bots are deliberately
**not** automatic — a 1v1 LAN is the main use.

## LAN

IW4L networking is **master-mediated**: `iw4l-master` is a relay and server
browser, and a client will not attempt a network match without a community
`.iw4l-server` descriptor. There is no peer-to-peer discovery, so "two phones on
the same Wi-Fi, nothing else" does not work as-is.

To play on a LAN today:

1. Run `iw4l-master` on a machine on the network (`cargo run -p iw4l-master`,
   or `cargo xtask master install` for a host). It needs a certificate and a
   `community.iw4l-server` descriptor pointing at the machine's LAN address.
2. Drop that `.iw4l-server` in the app's files directory
   (`/sdcard/Android/data/dev.iw4j.iw4l/files/`). The Android entry selects the
   first descriptor it finds and sets `IW4L_COMMUNITY` to it.
3. One device hosts from the menu (or `IW4L_MASTER_HOST_NAME`), the other joins
   from the browser.

A direct-connect path (host a listen server, join by IP with no master) is not
implemented; it would be an engine change in `net`.

