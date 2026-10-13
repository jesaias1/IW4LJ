# IW4L on Android — port plan

Status: **active**. Target: a launchable `.apk` running the IW4L runtime on a
modern ARM64 phone (Android 12+/API 31+), tuned for a Samsung S23 class device
(Snapdragon 8 Gen 2 / Adreno 740, Vulkan 1.3). Unlock-all, bots and LAN reuse
existing engine systems.

IW4L is **multiplayer-only**: it has no campaign runtime, so there is no
MP/campaign split to make — the Android build is already the MP build.

## What already exists (do not rebuild)

| Goal | Where it already lives |
|---|---|
| Unlock all weapons/unlocks | console command `unlock all` — `crates/console/src/local_account.rs` |
| Bots | `crates/bots` + console `bot add <n>`; `force_match_start` (docs/BOTS.md) |
| LAN | the IW4L master: run `iw4l-master` on a machine on the same network and give both devices its `.iw4l-server` descriptor |
| Menus / class setup | `crates/ui`, `crates/hud` |
| Map/asset loading | `crates/assets`, `crates/asset_transport` (plain `std::fs`) |

## Build route (working)

- Toolchain: Rust `aarch64-linux-android`, Android SDK + NDK r28, JDK 17,
  `cargo-apk` (NativeActivity).
- Target: `aarch64-linux-android`, `target_sdk 34`, `min_sdk 31`.
- Probe app at `D:\IW4J\bevy-probe` proved the pipeline end-to-end (a signed
  APK from a Bevy 0.20 app).
- Bare target builds need `D:\IW4J\Android\target-env.ps1` (CC/CXX/AR/LINKER to
  the NDK's versioned wrappers; NDK bin deliberately off PATH so build scripts
  that probe for a bare `clang++` use the wrapper and keep the API level).
- `crates/android/package.ps1` builds and signs the **full** APK: the native
  library, the Java `GameActivity` (controller bridge + immersive fullscreen),
  the manifest, dex, alignment and signing. `cargo apk build` still works but
  produces a plain `NativeActivity` with no controller and no fullscreen.

## Port changes made for Android

- `crates/android`: the `cdylib` entry Android loads; resolves the device's
  writable files dir and calls `bootstrap::launch`.
- Workspace Bevy features: added `android-native-activity`, `bevy_clipboard`.
- `system_clipboard` is desktop-only (`arboard` has no Android backend);
  `bevy_clipboard` still provides the `Clipboard` type via its fallback.
- `rfd` (native folder picker) is desktop-only; the Android stub logs instead.
- **BC textures** (`asset_material`): mobile GPUs have no BC hardware decode, so
  `DecodedMips::into_rgba8` decodes every level to RGBA when the target is
  Android (`to_platform_storage`), and `bootstrap` no longer requests
  `TEXTURE_COMPRESSION_BC` there. Without both, device creation fails and the
  app is a black screen.
- `perfetto-sdk-sys` builds against API 31 (needs `pthread_cond_clockwait`).
- **No explicit wgpu features on Android** (`bootstrap`): Bevy's default
  `Functionality` priority already enables every feature the adapter reports;
  requesting one the GPU lacks fails `request_device`, which is a black screen
  before anything draws.
- **`crates/touch`**: the on-screen pad; it presses the keys and mouse the bind
  system already reads (`console/assets/default_controls.cfg`), plus the mod
  menu that pushes console lines (`unlock all`, `bot add 8`, …), opened by a
  three-finger tap.
- **Controller** (`crates/android/android/java/.../GameActivity.java` +
  `crates/touch::GAMEPAD`): `gilrs` has no Android backend and winit drops a
  pad's axes, so the activity reads the pad and the JNI bridge writes the state
  the plugin turns into the same synthetic keys and mouse.
- **Immersive fullscreen** (same activity): the system bars are hidden on focus.
- **M5 defaults** (`crates/android`, `crates/touch`): `unlock all` when
  progression loads; the `MODS` overlay for everything else. Bots are opt-in on
  purpose so a 1v1 LAN is not spoiled.
- **Community selection on Android** (`crates/android`): the entry calls
  `updater::startup()` so a `.iw4l-server` descriptor in the files directory is
  selected. Without that call `master_target()` sees no community and networking
  stays offline.

## Milestones

- **M0 toolchain** — done.
- **M1 probe APK** — done (pipeline validated).
- **M2 engine boots** — done: the whole engine checks and links for
  `aarch64-linux-android`; a signed optimized APK builds (`profile play`, 37 MB,
  stripped).
- **M3 render** — done in code: BC levels decode to RGBA on Android and the
  build requests no explicit device features (Bevy's `Functionality` priority
  already enables everything the adapter reports). Verify a map draws on device.
- **M4 touch** — done in code (`crates/touch`): a virtual pad that presses the
  bound keys and mouse and emits look deltas. Verify on device.
- **M5 features** — the phone auto-runs `unlock all` once progression loads (and
  nothing else: a 1v1 LAN against a friend is the point, so **bots are never
  automatic**). A touch **mod menu** (`crates/touch`, the `MODS` button) runs
  the console lines on demand — unlock all / reset level / add bots / start
  match / spawn. LAN needs the IW4L master on the same network: run `iw4l-master`,
  drop its `.iw4l-server` descriptor in the app files directory, and both phones
  find it (the Android entry selects the first descriptor it sees).
- **M6 perf (S23)** — pending: render scale, FPS cap, persistent wgpu pipeline
  cache, Adreno warmup, core affinity, thermal budget.

## Open risks

- Adreno support for the requested bindless/array features must be verified on
  device; if any are missing the request must shrink to the adapter's set.
- Game data (multi-GB) is sideloaded to the app's external files dir; the
  loader must be pointed there.
- Touch input is the gating feature for "playable": M4 decides whether the
  menus and a match are usable on a phone.
