# Two ports, one engine

Android lives in `C:\Games\iw4x\iw4L-master`, iOS in `C:\Games\iw4x\IW4LJ iOS`.
They are separate copies on purpose (the Android project must keep working
exactly as it does), so **a change to the game is made twice**: once in each
folder. Anything that touches the network protocol, the game rules or the
content MUST be in both, or an iPhone and an Android phone cannot play together.

## What is shared and what is not

Identical in both folders (change both): every crate except those below.

iOS only (this folder): `crates/ios`, `crates/mobile`, `crates/touch/src/ios.rs`,
`crates/touch/src/safe_area.rs` and the thermal hook in `frame_cap.rs`,
`ios/`, `.github/`, the iOS `cfg` branches in the shared crates listed in
[IOS.md](IOS.md), `diag::set_launch_error_hook`.

Android only (`iw4L-master`): `crates/android`.

## Checking two folders have not drifted

```powershell
fc /b "C:\Games\iw4x\iw4L-master\crates\net\src\lib.rs" "C:\Games\iw4x\IW4LJ iOS\crates\net\src\lib.rs"
```

or compare the whole crates folders with any diff tool, ignoring the files above.
`net::PROTOCOL_VERSION` and `master_protocol::PROTOCOL_VERSION` must be equal.

## Done in both folders

- **FPS cap**: the game is held to 60 fps by default (`crates/touch/src/frame_cap.rs`,
  a sleep at the end of the main frame) to keep phones cool in long sessions.
  The mod menu's Graphics tab has `CAP FPS 60` / `UNCAP FPS` (saved in
  `uncap_fps.txt`, applied at once). `IW4L_FPS_CAP=<n>` in `env.txt` changes the
  rate; `0` removes the cap. Applied to both projects on 2026-10-13.
