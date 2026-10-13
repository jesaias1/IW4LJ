# IW4LJ

Phone ports of IW4L (by vladtrc) — an open-source runtime for
Call of Duty: Modern Warfare 2 (2009), written in Rust with Bevy — for **Android**
and **iOS**. Android and iPhone players can play together on LAN and online.

| Folder | Port | Start here |
|---|---|---|
| [`IW4LJ-Android`](IW4LJ-Android) | Android (arm64, Android 12+) | [`docs/ANDROID.md`](IW4LJ-Android/docs/ANDROID.md), [`docs/ANDROID-DEVICE.md`](IW4LJ-Android/docs/ANDROID-DEVICE.md) |
| [`IW4LJ-iOS`](IW4LJ-iOS) | iPhone / iPad (iOS 16+) | [`ios/README.md`](IW4LJ-iOS/ios/README.md), [`ios/docs/IOS-DEVICE.md`](IW4LJ-iOS/ios/docs/IOS-DEVICE.md) |

The two folders are separate, complete copies of the engine so that each build
stands on its own. A change to the game, the netcode or the menus must be made in
both, or the two ports stop being able to play together — see
[`IW4LJ-iOS/ios/docs/PORTS.md`](IW4LJ-iOS/ios/docs/PORTS.md).

## No game files here

This repository contains source code only. It contains **no** Activision game
data: no zone files, IWDs, textures, sounds or maps. You need your own copy of
Modern Warfare 2 multiplayer, which you copy onto the phone yourself (the device
guides above explain how). Call of Duty and Modern Warfare are trademarks of
Activision; this project is unaffiliated with them.

## Status

- Android: playable on a Galaxy S23 (menus, matches, bots, controller, LAN).
- iOS: written and type-checked, **not yet run on a device**. The IPA is built by
  the *iOS build* GitHub Actions workflow and installed with Sideloadly/AltStore.

Source licence: Apache 2.0 (see each folder's `LICENSE` and `NOTICE`).
