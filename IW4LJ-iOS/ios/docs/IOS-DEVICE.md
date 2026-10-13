# Installing and running IW4LJ on an iPhone

You need an iPhone or iPad (iOS 16+), a copy of **Modern Warfare 2 multiplayer**
you own (about 12 GB), and a way to install an unsigned app.

## 1. Get the app

Build the IPA: push this folder to GitHub and run the **iOS build** workflow
(see [../README.md](../README.md)), or on a Mac run `ios/scripts/build-ios.sh`.
The result is `IW4LJ.ipa`, unsigned.

## 2. Install it (from Windows, no Mac)

- **Sideloadly** (sideloadly.io): connect the phone, drag in `IW4LJ.ipa`, sign
  in with a free Apple ID, Start. On the phone: Settings → General → VPN & Device
  Management → trust your Apple ID. A free Apple ID signature lasts **7 days**;
  reinstall (keeping data) when it lapses.
- **AltStore** works the same way and can refresh over Wi-Fi.
- With a paid developer account, sign with
  `ios/app/Entitlements-paid.plist` (more memory, multicast) — see the header of
  `ios/scripts/build-ios.sh`.

Do not delete the app to reinstall: that deletes the 12 GB of game data with it.
Installing over the top keeps it.

## 3. Copy the game

Run the app once. It creates the folder and then tells you the game is missing.
Put a `mw2` folder, containing `zone/` and `main/` from your MW2 install, here:

```
Files app → On My iPhone → IW4LJ → mw2
```

Ways to get 12 GB there:

- **Windows**: install *Apple Devices* (Microsoft Store) or iTunes, select the
  phone → File Sharing → IW4LJ → drag in `mw2`.
- **Files app**: connect a USB-C drive (iPhone 15+/iPad) or an SMB share
  (Files → ⋯ → Connect to Server) and copy the folder.
- **Mac**: Finder → the phone → Files → IW4LJ.

`mw2/zone/english/common_mp.ff` must exist.

## 4. Play

- Touch controls are the same as Android. Three fingers (or the MENU button)
  open the mod menu: unlock all, bots, graphics, LAN.
- A Bluetooth controller (Xbox, PlayStation, MFi) works once paired in Settings.

### LAN / online with Android phones

IW4L is master-mediated. For two phones on one Wi-Fi no extra setup is needed:
one hosts from the mod menu's LAN tab, the other joins. The first time, iOS
asks to allow **Local Network** access — allow it, or LAN play cannot work.
If the host is not found, use **JOIN BY IP** in the LAN tab and type the address
the host's LAN tab shows.

For online play, drop the `.iw4l-server` file from a server operator into the
IW4LJ folder in Files, same as on Android.

Android and iPhone players join the same lobbies as long as both apps were
built from the same revision of this project.

## Settings files and switches

`Documents/env.txt`, `cmds.txt` and the mod menu's `.txt` files work exactly as
on Android; see `docs/ANDROID-HANDOFF.md` §7 for the environment switches
(`IW4L_TEXTURE_SKIP`, `IW4L_PIPELINED_RENDERING`, …). Logs are in
`Documents/iw4l-artifacts/logs/` — share one from the Files app when reporting a
problem.

## If it does not start

- It opens and closes at once: it crashed before it could say why. Open
  `iw4l-artifacts/logs/` and send the newest file.
- A message about a missing folder: the game is not in `Documents/mw2`.
- Black screen: a graphics error; the log says which (Metal validation).
- "Unable to install": the 7-day signature expired, or the bundle id is taken
  by an older sideload; install over it.
