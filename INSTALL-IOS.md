# Install IW4LJ on iPhone / iPad

You need: an iPhone or iPad (iOS 16+, iPhone 12 or newer recommended), a Windows
PC or a Mac, a copy of **Modern Warfare 2 multiplayer** you own (about 12 GB: the
`zone` and `main` folders), and a free Apple ID.

> **Status:** the iOS app builds, but it has not been run on a device yet. Expect
> rough edges; please report what you see (see the end of this guide).

## 1. Get the app (`IW4LJ.ipa`)

Download **IW4LJ.ipa** from this repository's **Releases** page (right-hand side of
the repo's main page, then the newest *iOS* release):
<https://github.com/jesaias1/IW4LJ/releases/tag/ios-v0.2.0>

The IPA is built by GitHub, so no Mac is needed. If you want a fresher build than
the release, open the **Actions** tab, then **iOS build**, pick the newest
successful run and download the **IW4LJ-ipa** artifact (unzip it to get the IPA).

On a Mac you can instead run `IW4LJ-iOS/ios/scripts/build-ios.sh`.

## 2. Install it on the phone

The IPA is unsigned: an installer app signs it with your own (free) Apple ID. Two ways;
**SideStore is the easier and more reliable one**, Sideloadly is the fallback.

### Option A (recommended): SideStore

[SideStore](https://sidestore.io) needs a computer **once**, to put itself on the phone.
After that it runs on the phone and **refreshes your apps in the background**, so you do
not have to plug in every week. (A free Apple ID still signs for 7 days; SideStore renews
the signature before then, as long as it keeps working. Open SideStore now and then and
check the days counter under *My Apps*.)

You need: iOS 15 or newer, Wi-Fi, a free Apple ID, and a PC or Mac for the first install.

1. On the phone, install **LocalDevVPN** from the App Store.
2. On the computer, install **iloader** (it is listed on
   [docs.sidestore.io/docs/installation/prerequisites](https://docs.sidestore.io/docs/installation/prerequisites)).
   On Windows it also needs iTunes or *Apple Devices*.
3. Connect the phone with a cable and tap **Trust**. In iloader sign in with your Apple ID
   and choose **Install SideStore**.
4. On the phone: **Settings → General → VPN & Device Management →** your Apple ID **→
   Trust**, and turn on **Developer Mode** (*Privacy & Security*) if asked. Restart if asked.
5. Open **LocalDevVPN** and tap **Connect**, then open **SideStore** and sign in.
6. Add the IW4LJ source and install: in SideStore go to **Sources → +** and add
   `https://raw.githubusercontent.com/jesaias1/IW4LJ/main/altstore/source.json`
   then tap **Get** next to IW4LJ. (Or download `IW4LJ.ipa` and open it with SideStore.)

Free Apple IDs allow **3 apps at a time, including SideStore itself**, and 10 new app IDs a
week, which is plenty for SideStore plus IW4LJ. The VPN has to be on while SideStore
installs or refreshes. If SideStore stops refreshing it may need a new "pairing file";
SideStore's docs explain how.

### Option B: Sideloadly (needs a PC every 7 days)

1. Install **Sideloadly** (sideloadly.io) on Windows, plus iTunes or *Apple
   Devices* so the PC can talk to the phone.
2. Connect the phone with a cable and tap **Trust**.
3. In Sideloadly pick the phone, drag in `IW4LJ.ipa`, enter your Apple ID, press
   **Start**.
4. On the phone: **Settings → General → VPN & Device Management →** your Apple ID
   **→ Trust**. On iOS 16+ also turn on **Settings → Privacy & Security →
   Developer Mode** and restart when asked.

A free Apple ID signature lasts **7 days**: reinstall over the top with Sideloadly
when the app stops opening. AltStore does the same job and can refresh over Wi-Fi.

**Do not delete the app** to reinstall it: that deletes the game files with it.

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
- **Graphics tab:** the game is held to 60 fps to keep the phone cool; **FPS 90** and
  **FPS 120** raise it. If the phone gets very hot iOS makes the game slow down anyway.
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

### Sideloadly fails with "GSA returned a non-plist response (HTTP 503)"

That is Apple's sign-in server (`gsa.apple.com`) refusing the request; the IPA is
not involved and it is not something this app can fix. Usually it passes:

1. Wait 10-30 minutes and try again; Apple's servers have bad spells.
2. Turn off any VPN, proxy or DNS filter, or switch network (a phone hotspot
   works). Antivirus "HTTPS scanning" can also break the connection.
3. Update Sideloadly to the latest version (sideloadly.io).
4. On Windows, install iTunes and iCloud from apple.com (not the Microsoft Store
   versions); Sideloadly depends on them.
5. If the Apple ID uses two-factor authentication, create an app-specific password
   at appleid.apple.com (Sign-In and Security, then App-Specific Passwords) and
   use that instead of the normal password.
6. Try a different (new, free) Apple ID.
7. Try AltStore instead; it signs through a different route.

If it still fails, send the full text from Sideloadly's log with your report.

More detail: [`IW4LJ-iOS/ios/docs/IOS-DEVICE.md`](IW4LJ-iOS/ios/docs/IOS-DEVICE.md)
and [`IW4LJ-iOS/ios/docs/IOS.md`](IW4LJ-iOS/ios/docs/IOS.md).
