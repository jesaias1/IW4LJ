# Install IW4LJ on iPhone / iPad

You need: an iPhone or iPad (iOS 16+, iPhone 12 or newer recommended), a Windows
PC or a Mac, a copy of **Modern Warfare 2 multiplayer** you own (about 12 GB: the
`zone` and `main` folders), and a free Apple ID.

> **Status:** the iOS app builds, but it has not been run on a device yet. Expect
> rough edges; please report what you see (see the end of this guide).

## 1. Get the app (`IW4LJ.ipa`)

The IPA is built by GitHub, so no Mac is needed:

1. Open this repository's **Actions** tab → **iOS build**.
2. Open the newest successful run and download the **IW4LJ-ipa** artifact; unzip it
   to get `IW4LJ.ipa`. (Or press **Run workflow** to build a fresh one, about an
   hour.)

On a Mac you can instead run `IW4LJ-iOS/ios/scripts/build-ios.sh`.

## 2. Install it on the phone

The IPA is unsigned; Sideloadly signs it with your Apple ID.

1. Install **Sideloadly** (sideloadly.io) on Windows, plus iTunes or *Apple
   Devices* so the PC can talk to the phone.
2. Connect the phone with a cable and tap **Trust**.
3. In Sideloadly pick the phone, drag in `IW4LJ.ipa`, enter your Apple ID, press
   **Start**.
4. On the phone: **Settings → General → VPN & Device Management →** your Apple ID
   **→ Trust**. On iOS 16+ also turn on **Settings → Privacy & Security →
   Developer Mode** and restart when asked.

A free Apple ID signature lasts **7 days**: reinstall over the top with Sideloadly
when the app stops opening. **Do not delete the app** to reinstall — that deletes
the game files with it. AltStore does the same job and can refresh over Wi-Fi.

## 3. Copy the game

Open the app once. It will say the game is missing — that creates its folder. Then
put your `mw2` folder (containing `zone/` and `main/`) into it:

- **From Windows:** *Apple Devices* (or iTunes) → select the phone → **File
  Sharing** → **IW4LJ** → drag in `mw2`.
- **From the phone:** copy it with the **Files** app (*On My iPhone → IW4LJ*) from
  a USB-C drive or a network share (*Files → ⋯ → Connect to Server*).
- **From a Mac:** Finder → the phone → **Files** → IW4LJ.

`mw2/zone/english/common_mp.ff` must exist. Close and reopen the app.

## 4. Play

- On-screen controls are the same as on Android; **MENU** or a **three-finger
  tap** opens the mod menu (unlock all, bots, maps, skins, trickshot, graphics,
  LAN). Bluetooth controllers (Xbox, PlayStation, MFi) work once paired in Settings.
- **Graphics tab:** the game is held to 60 fps to keep the phone cool; **UNCAP
  FPS** lifts it. If the phone gets very hot iOS makes the game slow down anyway.
- **LAN / online, crossplay with Android:** mod menu → **LAN** → *Host* or *Join*.
  iOS asks once to allow **Local Network** — say yes. If the host isn't found, use
  *Join by IP*. Both players must be on builds from the same revision of this
  repository. For online servers, copy the `.iw4l-server` file into the IW4LJ
  folder in Files.

## If something goes wrong

- Opens and closes at once, or a black screen: open the Files app →
  *IW4LJ → iw4l-artifacts → logs*, and send the newest file with your report.
- A message about a missing folder: the game isn't in `IW4LJ/mw2`.
- Can't install / "Unable to verify": the 7-day signature ran out — reinstall.

More detail: [`IW4LJ-iOS/ios/docs/IOS-DEVICE.md`](IW4LJ-iOS/ios/docs/IOS-DEVICE.md)
and [`IW4LJ-iOS/ios/docs/IOS.md`](IW4LJ-iOS/ios/docs/IOS.md).
