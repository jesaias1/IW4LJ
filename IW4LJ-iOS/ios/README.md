# IW4LJ for iOS

The iPhone/iPad port of IW4L (the Rust + Bevy runtime for Modern Warfare 2),
built from the same engine as the Android port and able to play LAN and online
matches with Android phones.

This folder (`C:\Games\iw4x\IW4LJ iOS`) is a **complete, separate copy** of the
project. The Android project (`C:\Games\iw4x\iw4L-master`) is untouched and keeps
building exactly as before.

**Status: written and type-checked, never run on a device.** No iPhone and no
Mac were available. Everything below says what was verified and what was not.
Read [docs/IOS.md](docs/IOS.md) for the state of the port and
[docs/IOS-DEVICE.md](docs/IOS-DEVICE.md) for installing it and copying the game.

## Where things are

| Path | What |
|---|---|
| `crates/ios` | the iOS entry: app folders, controller, text prompts, launch-error screen, audio session |
| `crates/mobile` | startup shared by phone ports (environment, community descriptor, launch) |
| `crates/touch/src/ios.rs` | iOS touch handling: real screen size, touches remapped to the render size |
| `ios/app` | `Info.plist`, entitlements, app icon |
| `ios/scripts/build-ios.sh` | macOS build: `.app` and unsigned `.ipa` |
| `ios/scripts/run-simulator.sh` | build and run in the iOS simulator (Mac only) |
| `ios/scripts/windows-check/check-ios.ps1` | type-check the iOS build on Windows |
| `.github/workflows/ios.yml` | builds the IPA on a GitHub Mac, no Mac needed |
| `ios/docs` | this port's documentation |

## Getting an IPA without a Mac

1. Put this folder in a GitHub repository (private is fine).
2. Actions tab, run **iOS build**, download the `IW4LJ-ipa` artifact (about an hour).
3. Install `IW4LJ.ipa` from Windows with Sideloadly or AltStore (free Apple ID,
   the app must be re-signed every 7 days) — see [docs/IOS-DEVICE.md](docs/IOS-DEVICE.md).

## Checking the code on Windows

```powershell
powershell -ExecutionPolicy Bypass -File ios\scripts\windows-check\check-ios.ps1
```

This runs `cargo check` for `aarch64-apple-ios` with a stand-in C compiler. It
proves the Rust compiles for iOS. It cannot link or run anything.
