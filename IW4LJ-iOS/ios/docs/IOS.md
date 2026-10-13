# IW4LJ on iOS — port state

Target: iPhone 12 / A14 or newer (and iPads with an M-series or A14+ chip),
iOS 16+, Metal through wgpu. Same engine, same game, same menus as Android.

## How it is built

The iOS port is the Android project's engine plus an iOS entry. Nothing in the
runtime is forked: where the engine already had a "phone" path (`target_os =
"android"`), it now also covers iOS.

| Area | Android | iOS |
|---|---|---|
| Entry | `crates/android` (cdylib, `GameActivity.java`) | `crates/ios` (executable `iw4lj`, Rust + objc2) |
| Startup | `crates/android` | `crates/mobile` (iOS only so far; same code as the Android entry's startup) |
| GPU | Vulkan | Metal (wgpu) |
| Textures | BC → ETC2 at load | BC → ETC2 at load (every Apple GPU decodes ETC2) |
| Depth-as-colour | `R16Float` | `R16Float` |
| Touch | Java forwards touches over JNI | winit delivers them; `touch/src/ios.rs` remaps them to the render size |
| Controller | `GameActivity` axes/keys | GameController framework → the same `touch::BridgeEvent`s (`crates/ios/src/gamepad.rs`) |
| Text prompts | `AlertDialog` | `UIAlertController` (`crates/ios/src/prompt.rs`) |
| Render scale | surface smaller than the view | same (`touch/src/ios.rs::report_view`, `bridge::render_size`) |
| Core pinning | big cores via `sched_setaffinity` | none (iOS has no affinity API) |
| Allocator | mimalloc | system allocator |
| Fatal errors | logcat | an alert / message screen (`crates/ios/src/notice.rs`) |
| Perfetto | on | off (developer tool) |
| Pipelined rendering | on | **off** by default, like macOS (the Metal surface must be created on the main thread). `IW4L_PIPELINED_RENDERING=1` in `env.txt` turns it on |
| Clipboard / folder picker | off | off |

Changes to shared crates (all `cfg`-gated, desktop and Android behaviour is
unchanged): `asset_material` (ETC2 + memory tiers on iOS), `bootstrap`
(mobile GPU settings, window flags for the home bar/status bar), `console`,
`ui`, `replay`, `render_gpu`, `asset_transport`, `perf`, `diag` (a launch-error
hook), `touch`.

## Folders on the device

Everything the player touches is in the app's **Documents** folder, which the
Files app shows as *On My iPhone → IW4LJ*:

```
Documents/mw2/zone/...        the MW2 multiplayer install (not backed up)
Documents/mw2/main/...
Documents/iw4l-artifacts/     caches, logs, demos (not backed up)
Documents/*.iw4l-server       a community descriptor for online play
Documents/cmds.txt            startup console script (default: unlock all)
Documents/env.txt             NAME=value tuning switches
Documents/*.txt               settings written by the mod menu
```

Profile and account are in the app's private `Library/Application Support/iw4l`.

## Crossplay

Both ports run the same network code; the protocol numbers
(`net::PROTOCOL_VERSION`, `master_protocol::PROTOCOL_VERSION`) are constants, so
an iPhone and an Android phone built from the same engine revision meet in the
same lobby. **Both must be built from the same revision**: a change to the
netcode or the game content on one side only breaks the match. See
[PORTS.md](PORTS.md).

## Verified

- The whole workspace type-checks for `aarch64-apple-ios` and
  `aarch64-apple-ios-sim` (`check-ios.ps1`), and for the Windows host.

## Not verified (no device, no Mac)

- That it links and launches. The first real build is on macOS (CI).
- Metal: that the adapter exposes what the renderer uses (binding arrays,
  texture formats) and that nothing hits a Metal validation error. On Android
  such an error made Bevy quit the app (see `docs/ANDROID-HANDOFF.md` §5);
  expect to read the log first.
- Render-scale on Metal (drawable smaller than the view) and the touch remap.
  If touches land in the wrong place or the picture is the wrong size, set
  `render_height.txt` to the screen's pixel height, or edit
  `touch/src/ios.rs`.
- Controller mapping, rumble (**not implemented** on iOS yet), text prompts.
- LAN discovery by broadcast; "JOIN BY IP" in the LAN tab is the fallback.
- Memory: an iPhone with 6 GB gets texture tier 3 (the biggest three mip
  levels dropped); 8 GB gets tier 2. The Increased Memory Limit entitlement
  (`ios/app/Entitlements-paid.plist`) needs a paid developer account.

## Device-side extras (written, untested)

- **Rumble**: the game's vibration goes to the pad's two handles through
  GameController haptics (`crates/ios/src/rumble.rs`).
- **Safe area**: the on-screen buttons keep clear of the rounded corners and
  notch (`touch/src/safe_area.rs`, fed once a second by `platform.rs`). The
  game's own HUD already sits inside the engine's 85 % safe area.
- **Thermal**: when iOS reports a *serious* thermal state the 60 fps cap is held
  even if uncapped; at *critical* the game drops to 30 fps
  (`touch/src/frame_cap.rs`, iOS only).
- **Memory**: the texture tier follows `os_proc_available_memory()` (what iOS
  will still let the app use), not the phone's RAM; a memory warning is written
  to the log with the headroom left.
- **Audio**: the audio session is reactivated after a call/alarm and when the
  app returns to the front; the runtime's own audio supervisor then reopens the
  output.

## Known gaps / next steps

1. Everything above needs a real device to tune: the memory tier thresholds,
   the thermal behaviour, haptic strength.
2. Audio interruptions rely on the audio supervisor noticing the stalled stream.
3. The mod menu has no iOS-specific entries.

Done: the 60 fps cap with an "Uncap FPS" button in the Graphics tab (both ports,
see [PORTS.md](PORTS.md)).
