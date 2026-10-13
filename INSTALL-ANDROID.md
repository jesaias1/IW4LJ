# Install IW4LJ on Android

You need: an Android 12+ phone (arm64, 6 GB RAM or more recommended), a copy of
**Modern Warfare 2 multiplayer** you own (about 12 GB: the `zone` and `main`
folders), and the IW4LJ APK.

## 1. Get the APK

Download **`IW4LJ-Android.apk`** (about 125 MB) from the latest release:
**https://github.com/jesaias1/IW4LJ/releases/latest** — on the phone's browser or
on the PC, then copy it across.

*Building it yourself instead:* see
[`IW4LJ-Android/docs/ANDROID-HANDOFF.md`](IW4LJ-Android/docs/ANDROID-HANDOFF.md)
§2–3 (`IW4LJ-Android/crates/android/package.ps1`; needs Rust with the
`aarch64-linux-android` target, JDK 17, the Android SDK and NDK r28 — the paths at
the top of the script are the author's, change them to yours).

## 2. Install it

Open `IW4LJ-Android.apk` on the phone (copy it over if you downloaded it on the PC)
and allow "install unknown apps" for your browser or file manager when asked. Or, with
USB debugging on:

```
adb install -r IW4LJ-Android.apk
```

**Never `adb uninstall`** and don't uninstall the app from the phone to reinstall:
that deletes the app's data folder. `install -r` keeps it.

## 3. Copy the game

Open the app once so it creates its folder, then put a `mw2` folder (containing
`zone/` and `main/` from your MW2 install) here — by USB cable and a file manager
or MTP, no adb needed:

```
Internal storage/Android/media/dev.iw4j.iw4l/mw2/zone/...
Internal storage/Android/media/dev.iw4j.iw4l/mw2/main/...
```

With adb you can also use `/sdcard/Android/data/dev.iw4j.iw4l/files/mw2`. The app
uses whichever already holds `zone/`. `mw2/zone/english/common_mp.ff` must exist.

## 4. Play

- On-screen controls: left thumb moves, right side looks and has fire / aim /
  jump / crouch / reload. **MENU** (top right) or a **three-finger tap** opens the
  mod menu: unlock all, bots, maps, skins, trickshot tools, graphics, LAN.
- A Bluetooth or USB controller works as well; its Xbox/PlayStation button opens
  the mod menu.
- **Graphics tab:** the game is held to 60 fps to keep the phone cool. Phones that
  can take more can press **UNCAP FPS**. Render resolution and texture quality
  apply at the next launch.

### LAN and online (Android ⇄ Android, Android ⇄ iPhone)

Same Wi-Fi: one phone opens the mod menu → **LAN** → *Host LAN game*, the others
*Join LAN game* (or *Join by IP* and type the host's address, shown on the host's
LAN tab). Both phones must run builds from the same revision of this repository,
which is also what lets iPhones and Android phones play together.

For online servers, drop the `.iw4l-server` file from the server operator into
`Android/media/dev.iw4j.iw4l/`.

## Settings files and logs

`env.txt`, `cmds.txt` and the mod menu's `.txt` files sit in the same folder; see
[`ANDROID-HANDOFF.md`](IW4LJ-Android/docs/ANDROID-HANDOFF.md) §7 for the switches.
Logs: `adb logcat -s iw4l:* RustStdoutStderr:*`, or the files under
`Android/data/dev.iw4j.iw4l/files/iw4l-artifacts/logs/`.
