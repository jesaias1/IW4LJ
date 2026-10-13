# Android port — handoff

Status: **playable on a Galaxy S23**: menus, matches, bots, controller, trickshot menu and LAN work.
The open problem is **frame rate** (see section 7). Everything marked *untested* below was written
while the phone was not attached.

Read this file top to bottom before touching anything. It is the whole state.

---

## 1. The goal

A launchable `.apk` of IW4L on a modern ARM64 phone (Android 12+), eventually
scaling down to weaker phones. Features wanted: unlock-all, bots, LAN, and a
trickshot mode. IW4L is **multiplayer-only** (no campaign).

---

## 2. Environment (this Windows machine)

Installed and working:

| Thing | Where |
|---|---|
| Rust + Android targets | `%USERPROFILE%\.cargo\bin`, targets `aarch64-linux-android` |
| JDK 17 | `C:\Program Files\Microsoft\jdk-17.0.20.101-hotspot` |
| Android SDK | `D:\IW4J\Android\Sdk` |
| SDK packages | `platform-tools`, `platforms;android-34`, `build-tools;34.0.0`, `cmake;3.22.1` |
| NDK | `D:\IW4J\Android\Sdk\ndk\28.2.13676358` |
| Env scripts | `D:\IW4J\Android\env.ps1` (base), `D:\IW4J\Android\target-env.ps1` (adds CC/CXX/AR/linker for `cargo check`/`build`) |
| Device helper | `D:\IW4J\android-test.ps1` |
| Phone package | `D:\IW4J\phone\` (apk + `mw2/` data + README) |
| Build output | `D:\IW4J\android-build\iw4l.apk` |

**Target device:** Galaxy S23 (`SM-S911B`), Android 16, arm64, Adreno 740,
7.2 GB RAM. Attached by USB with `adb` (platform-tools).

`target-env.ps1` deliberately keeps the NDK bin **off** `PATH`: a crate whose
build script probes for a bare `clang++` (perfetto-sdk-sys) otherwise compiles
with `--target=aarch64-linux-android` and no API level, hiding the Android
headers. The versioned wrappers carry the API level.

---

## 3. Building and installing

`cargo apk` cannot carry Java, so the full APK is assembled by hand
(**this is the supported path now**):

```powershell
cd C:\Games\iw4x\iw4L-master\crates\android
powershell -ExecutionPolicy Bypass -File .\package.ps1   # → D:\IW4J\android-build\iw4l.apk
adb install -r D:\IW4J\android-build\iw4l.apk            # KEEP DATA
```

`package.ps1` runs: `cargo build --profile play --target aarch64-linux-android
-p iw4l-android` → `javac GameActivity.java` → `d8` → `aapt2 link` → add
`classes.dex` + `lib/arm64-v8a/{libiw4l_android.so,libc++_shared.so}` (stored
uncompressed) → `zipalign -p` → `apksigner`.

### Hard-won rules

- **Never `adb uninstall`.** It deletes `/sdcard/Android/data/dev.iw4j.iw4l/`,
  which is the 12 GB of game data. `install -r` keeps it.
- **`versionCode` must not go down.** The manifest uses `20000000` because the
  earlier `cargo-apk` build used a git-derived `16777472`. Keep raising it.
- **`libc++_shared.so` must be in the APK** (`lib/arm64-v8a/`), or the loader
  fails before `main` with `library "libc++_shared.so" not found`. `package.ps1`
  takes it from the NDK sysroot.
- Fast iteration without packaging: `. D:\IW4J\Android\target-env.ps1; cargo check -p iw4l-android --target aarch64-linux-android`.
- The `play` profile is used (opt-level 3, no fat LTO). Release LTO links are
  ~13 min; don't.

---

## 4. What was changed in the repo (all of it still stands)

New crates:

- **`crates/android`** — the `cdylib` Android loads. Resolves storage, sets
  `IW4L_GAMES` / `IW4L_ARTIFACTS_DIR` / `HOME` / `XDG_CONFIG_HOME` /
  `IW4L_CMDS` / `IW4L_COMMUNITY`, calls `updater::startup()`, then
  `bootstrap::launch(..., LaunchMode::Menu, ...)`. Also the JNI exports
  `Java_dev_iw4j_iw4l_GameActivity_native{Sticks,Triggers,Button}` that write
  `touch::GAMEPAD`.
  - `crates/android/android/AndroidManifest.xml` — activity `GameActivity`,
    `minSdk 31`, `largeHeap`, fullscreen theme, `hasCode=true`, lib_name
    `iw4l_android`.
  - `crates/android/android/java/dev/iw4j/iw4l/GameActivity.java` — a
    `NativeActivity` subclass that forwards pad input and logs `IW4LGame:`.
  - `crates/android/package.ps1` — the builder (above).
  - `crates/android/iw4l-dev.keystore` — dev key, git-ignored.
- **`crates/touch`** — the on-screen pad and the mod menu, plus the
  `Gamepad`/`GAMEPAD` state. Layout is recomputed when the window resizes.
  Radii are logical (dp) pixels. Three-finger tap opens the mod menu; it runs
  console lines through `console::ConsoleQueue` (`unlock all`, `bot add 8`, …).

Workspace `Cargo.toml`: added members `crates/android`, `crates/touch`; Bevy
features `android-native-activity`, `bevy_clipboard`, `default_font`; removed
`system_clipboard` (arboard has no Android backend — re-enabled desktop-only in
`crates/bootstrap/Cargo.toml`).

Other edits:

- `crates/bootstrap/src/plugins.rs` — adds `touch::TouchPlugin` (Android);
  **Android requests no explicit wgpu features** (Bevy's `Functionality`
  priority already enables everything the adapter reports; requesting an
  unsupported one fails device creation, a black screen).
- `crates/bootstrap/src/launch.rs` — `exit_condition()` returns `DontExit` on
  Android. (This did **not** fix the blocker; kept anyway.)
- `crates/console` — `rfd` is desktop-only; `game_folders.rs` `#[cfg]`-gated.
- **`crates/asset_material`** — the texture pipeline. `MipStorage` gained
  `Etc2Rgb`/`Etc2Rgba`; `to_platform_storage` calls `into_etc2()` on Android
  (was `into_rgba8`); `encode_etc2` uses `ctt-etcpak`; `texture_format` maps to
  `TextureFormat::Etc2*`; `disk_cache` tags 7/8 and `MIP_CACHE_FORMAT = 3`
  (bumped so old RGBA caches are ignored — important).

Docs written: `docs/ANDROID.md` (plan + changes), `docs/ANDROID-DEVICE.md`
(install, data, controls, LAN).

---

## 5. The foreground bug (fixed, kept for the lesson)

Symptom: the activity was "stopped" a moment after a match started. It was not an Android lifecycle
problem. Bevy 0.20's default RenderErrorHandler **quits the app on any wgpu validation error**, and the
app was producing two:

1. ETC2 textures whose base level was not a multiple of 4 (a 2-texel-high strip): into_etc2 now keeps
   such images as RGBA (etc2_base_size_ok).
2. A 32-bit float texture bound where the layout declares a *filterable* one. Adreno does not report
   FLOAT32_FILTERABLE, so depth-as-colour targets (shadow maps, FloatZ) are R16Float on Android
   (DEPTH_COLOUR_FORMAT), and float images are converted at decode.

If a match ever vanishes again, read logcat for Caught rendering error first.
## 6. The test loop (use it; it is how all evidence here was gathered)

```powershell
$adb = "D:\IW4J\Android\Sdk\platform-tools\adb.exe"

# log everything, then launch and read only your process's lines
& $adb logcat -c
& $adb shell am start -n dev.iw4j.iw4l/dev.iw4j.iw4l.GameActivity
Start-Sleep 20
$p = (& $adb shell pidof dev.iw4j.iw4l).Trim()
& $adb logcat -d --pid=$p -v brief | Select-Object -Last 40

# screenshot (adb pull, not `>` — PowerShell corrupts binary)
& $adb shell screencap -p /sdcard/s.png; & $adb pull /sdcard/s.png D:\IW4J\s.png

# who has focus / is anything killed
& $adb shell dumpsys window | Select-String mCurrentFocus
& $adb logcat -d -v brief | Select-String "lowmemorykiller|reaped|Fatal signal"
```

**Drive a match without the menus**: write a console script to
`/sdcard/Android/media/dev.iw4j.iw4l/cmds.txt` (e.g. `map mp_shipment`), then
launch. The entry runs it as `IW4L_CMDS`. This is how matches were loaded
repeatedly. `map mp_afghan` for a big map, `mp_shipment` for a small one.

**Measure memory** during a load:

```powershell
for ($i=0; $i -lt 60; $i++) {
  $m = & $adb shell dumpsys meminfo dev.iw4j.iw4l
  foreach ($l in $m) { if ($l -cmatch '^\s+TOTAL\s+(\d+)') { $Matches[1] } }
  Start-Sleep 1
}
```

(`-cmatch` — the header row also says "Total"; a case-insensitive match reads
the wrong one.)

---

## 7. Where things stand

**Works (device-tested):** menu, class select, Rust and Shipment matches, bots, unlock/progression
(Android refuses hard links, which broke account creation: fixed in local_account.rs), name change,
noclip, touch layout, mod menu, FPS readout, LAN host half (the relay answers iw4l-master list from a PC).

**Written, not yet tested on a device** (the phone was disconnected):

- *Frame rate work.* Core pinning on Android (ootstrap/src/frame_owner.rs; it had never run there),
  60 Hz refresh request, sun shadows off in the Performance preset, texture tier 2 on phones under ~10 GB,
  a rames: line in the log every 5 s (average, worst frame, count over 50 ms).
- *Controller.* GameActivity now merges a trigger's axis and key into one value per pad
  (PadState). Previously each source was forwarded as its own button and the two disagreed, which is
  the likely cause of "left trigger does not aim". Read IW4LPad lines in logcat: they name the pad
  and say which axes it has.
- *LAN.* Host broadcast now also goes to each interface's own broadcast address; "JOIN BY IP" in the
  LAN tab is the fallback if discovery fails. Two-phone play has **not** been run end to end.
- *UI colours.* etc2_to_rgba no longer swaps R and B (perk icons and weapon art were tinted blue).

**Frame rate, measured on the S23 (Rust, 720 lines, cool phone):** about 44 fps with default effects,
about 60 with shadows, depth of field, glow and distortion off. The user reports 10 to 30 fps in long
sessions even on the lowest settings: suspects, in order, are (a) thermal throttling plus threads on
efficiency cores, (b) memory pressure (the phone was swapping 2 GB of the game's heap), (c) bots.
A CPU profile showed 12 % of all time in syscall (futex wake-ups from the task pools), 13 % in
wgpu CommandEncoder::finish, 3.5 % in kCreateFramebuffer every frame, and 19 % in malloc/free.
**Next step: run the game for ten minutes with the rames: log on and see whether the dips line up
with bots, a map area, or heat.** Profiling recipes are in scratchpad-style scripts: simpleperf record
--app dev.iw4j.iw4l -g, symbolised with llvm-addr2line against an unstripped libiw4l_android.so
(package.ps1 leaves it in 	arget\aarch64-linux-android\play).

**Environment switches** (put NAME=value lines in /sdcard/Android/media/dev.iw4j.iw4l/env.txt):
IW4L_PIN_CORES=0 (no core pinning), IW4L_SCHEDULE_THREADS=single, IW4L_TEXTURE_SKIP=0..3,
IW4L_PIPELINED_RENDERING=0.

**Heap probe:** package.ps1 -Features heap-probe records large allocations by call site and prints the
top holders every 75 s (heap-probe: lines; resolve addresses with llvm-addr2line).

## 8. Priorities, in order

1. Frame rate (section 7): measure, then cut. Candidates: kCreateFramebuffer per frame, the
   executor wake-up storm, per-frame allocations in ender_gpu::drawsurf::colour_submit.
2. Test the controller and LAN on real hardware.
3. Trickshot: 	rickshot (Search and Destroy, no round timer, first round wins), lastkill, 
oclip,
   save/load position and loadspawn exist. The score/timelimit dvars used by lastkill and
   	rickshot (scr_sd_*, scr_*_scorelimit) are **not verified against the game scripts** (they are
   not in the repo): check that a match really ends on the first kill.
4. More menu settings; a proper Shipment pass (the DLC texture archives are now on the phone).
## 9. Device data (already on the phone)

`/sdcard/Android/data/dev.iw4j.iw4l/files/mw2/{zone,main}` — a full MW2 MP
install (12 GB). `mw2/zone/english/common_mp.ff` is the anchor. The app also
reads `/sdcard/Android/media/dev.iw4j.iw4l/mw2` if present (that path is
reachable from a file manager / MTP on Android 11+; `Android/data` is not).
A copy of the data and the APK is in `D:\IW4J\phone\`.
