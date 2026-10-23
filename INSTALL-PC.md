# Play IW4LJ on PC (with the mod menu)

The same engine as the phone ports, built for Windows, with the **mod menu**: trickshot
tools, explosive bullets, bots, maps, unlock all, weapon skins and more.

You need: Windows 10 or 11 (64-bit), a GPU that supports Vulkan or DirectX 12, and your own copy
of **Modern Warfare 2 multiplayer** (the Steam version works).

> **Status:** experimental. The PC build was added after the phone ports and has had far less
> testing. Please report what you find.

## 1. Download

Get **`IW4LJ-PC.zip`** from the release:
**https://github.com/jesaias1/IW4LJ/releases/tag/pc-v0.1.0**

Extract it into an empty folder you can write to (not *Program Files*). It contains `IW4LJ.exe`.

## 2. Start it

1. Run `IW4LJ.exe`. It looks for Modern Warfare 2 in your Steam libraries and makes a
   `Modern Warfare 2.lnk` shortcut next to itself.
2. If it can't find the game (a non-Steam install, for example, or an IW4x folder), make that shortcut
   yourself: right-click inside the folder, *New, Shortcut*, paste the path of your MW2 folder (the one
   that holds `zone` and `main`) and name it **Modern Warfare 2**.
3. Caches, saves and logs go in an `iw4l-artifacts` folder beside the exe.

## 3. The mod menu

| | |
|---|---|
| Open or close | **F1** or **Insert** (or the controller's home / Guide button) |
| Close | **Esc**, the **X** button, or F1 again |
| Press a button | left mouse click (the pointer is released while the menu is open), or the controller D-pad and **A** |

Start a match first (**Create Game** from the main menu, or use the console with the backtick
key: `map mp_terminal`). Then open the menu. Cheats are on in a match you host, so the
**TRICKSHOT** tab works: noclip, god mode, **explosive bullets**, save and load position, a still
bot, "next kill wins" and more. **MATCH** has **QUICK START**, which replays your last map and
bots.

With a controller, **PAD SAVE/LOAD** lets you save and load a position with Select + D-pad
left / right.

### What does not work on PC yet

- The **LAN** tab is built for phones. Use the game's own multiplayer menus for online play.
- **RENDER** and **TEXTURES** buttons in the Graphics tab do nothing on PC.
- **CHANGE NAME** and **JOIN BY IP** need a phone keyboard; use the console (`` ` ``) instead.
- On PC the frame rate is not capped by default; the **FPS** buttons in Graphics set a cap.

## No game files here

This download contains the engine only. It does not contain any Activision game data.
Call of Duty and Modern Warfare are trademarks of Activision; this project is unaffiliated
with them.

## Problems

Open an [issue](https://github.com/jesaias1/IW4LJ/issues) with your Windows version, graphics
card, what you did and what happened. Attach the newest file from `iw4l-artifacts\logs`.
