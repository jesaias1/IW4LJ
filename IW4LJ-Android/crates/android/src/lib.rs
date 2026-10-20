//! Android entry point.
//!
//! Android loads a shared library and calls into it, so this crate is a
//! `cdylib` with a `main` that the platform starts, unlike the desktop
//! `iw4l` binary. It resolves the device's writable storage for the game
//! data and artifacts, then hands control to `bootstrap`, which assembles
//! and runs the same `App` the desktop build does.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

#[cfg(feature = "heap-probe")]
mod heap_probe;

#[cfg(feature = "heap-probe")]
#[global_allocator]
static HEAP: heap_probe::Probe = heap_probe::Probe;

/// The phone's own allocator is the bottleneck of a frame: a profile spent about
/// a fifth of the CPU in its malloc, free and thread-cache flushes, mostly for
/// memory one thread frees that another allocated. mimalloc keeps those frees
/// cheap.
#[cfg(not(feature = "heap-probe"))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

const PACKAGE: &str = "dev.iw4j.iw4l";

#[bevy_main]
fn main() {
    #[cfg(feature = "heap-probe")]
    heap_probe::start();

    // A panic on any thread is written where `LaunchActivity` reads it, so a
    // start that dies can say why on the next launch.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let report = format!(
            "{info}\nthread: {}\n{}",
            std::thread::current().name().unwrap_or("?"),
            std::backtrace::Backtrace::force_capture()
        );
        let path = format!("/data/data/{PACKAGE}/files/crash.txt");
        let mut text = std::fs::read_to_string(&path).unwrap_or_default();
        if text.len() < 12_000 {
            text.push_str(&report);
            text.push('\n');
            let _ = std::fs::write(&path, text);
        }
        default_hook(info);
    }));

    let files = first_existing(&[
        format!("/sdcard/Android/data/{PACKAGE}/files"),
        format!("/storage/emulated/0/Android/data/{PACKAGE}/files"),
    ])
    .unwrap_or_else(|| PathBuf::from(format!("/data/data/{PACKAGE}/files")));

    // `Android/media/<pkg>` is the one shared location a file manager and MTP
    // can write to on Android 11+, unlike `Android/data`. The app creates it so
    // a player can drag the game folder in over USB.
    let media = first_existing(&[
        format!("/sdcard/Android/media/{PACKAGE}"),
        format!("/storage/emulated/0/Android/media/{PACKAGE}"),
    ])
    .unwrap_or_else(|| PathBuf::from(format!("/sdcard/Android/media/{PACKAGE}")));
    let _ = std::fs::create_dir_all(&media);

    let games = games_root(&files, &media);
    let artifacts = files.join("iw4l-artifacts");
    // Settings, the profile and the account live in the app's private storage.
    // The shared `files` directory can hold folders the app does not own (a
    // game copied in over adb or MTP), and a profile that cannot be written
    // means unlocks and the player's name never survive a launch.
    let config = PathBuf::from(format!("/data/data/{PACKAGE}/files/config"));
    if let Err(error) = std::fs::create_dir_all(&config) {
        eprintln!("config directory {}: {error}", config.display());
    }

    // Sound only because this runs before any other thread exists; other code
    // reads `IW4L_GAMES` for content discovery and display.
    unsafe {
        std::env::set_var("IW4L_GAMES", &games);
        std::env::set_var("IW4L_ARTIFACTS_DIR", &artifacts);
        // The settings and local profile are found through `HOME` or
        // `XDG_CONFIG_HOME`; without one they are session-only and the unlocked
        // progression would not survive a restart.
        std::env::set_var("HOME", &files);
        std::env::set_var("XDG_CONFIG_HOME", &config);
        // `env.txt` beside the game: `NAME=value` lines set before anything
        // reads the environment. A phone has no shell to launch from, and
        // every tuning switch in the runtime is an environment variable.
        for base in [&media, &files] {
            let Ok(text) = std::fs::read_to_string(base.join("env.txt")) else {
                continue;
            };
            for line in text.lines() {
                if let Some((name, value)) = line.split_once('=')
                    && !name.trim().is_empty()
                    && !name.trim_start().starts_with('#')
                    // A development test once left this line on a phone, and the
                    // file could not be removed from inside the app.
                    && !value.contains("give weapon/iw4:weapon/uzi_mp akimbo")
                {
                    std::env::set_var(name.trim(), value.trim());
                }
            }
        }
        // The mod menu's texture setting, written beside the game and applied
        // from the next launch; without it the device's memory decides.
        if std::env::var_os("IW4L_TEXTURE_SKIP").is_none()
            && let Ok(skip) = std::fs::read_to_string(files.join("texture_skip.txt"))
        {
            std::env::set_var("IW4L_TEXTURE_SKIP", skip.trim());
        }
        // A phone has no console to type into, so maximum progression is the
        // default script. Bots are never automatic: a 1v1 LAN is the point, so
        // they are a mod-menu action. A `cmds.txt` dragged in beside the game,
        // or `IW4L_CMDS`, overrides it; a script is how a match is started
        // without the menus.
        if std::env::var_os("IW4L_CMDS").is_none() {
            let script = [&media, &files]
                .into_iter()
                .find_map(|base| std::fs::read_to_string(base.join("cmds.txt")).ok())
                .filter(|text| !text.trim().is_empty())
                // The same test's startup script (see above).
                .filter(|text| !text.contains("map mp_terminal; wait world; spawn 0; force_match_start; bot add 1"))
                .unwrap_or_else(|| "wait progression; unlock all".to_owned());
            std::env::set_var("IW4L_CMDS", script);
        }
        // A `.iw4l-server` descriptor dropped next to the game selects the
        // community master, so two phones on one network can meet without a
        // console. Discovery has to be told where to look: this machine's files
        // directory is not the launcher's own directory.
        if std::env::var_os("IW4L_COMMUNITY").is_none() {
            for base in [&media, &files] {
                if let Some(path) = first_descriptor(base) {
                    std::env::set_var("IW4L_COMMUNITY", path);
                    break;
                }
            }
        }
        // With no community of its own, a phone is its own: the built-in LAN
        // descriptor points the lobby browser at this device's loopback, where
        // the relay runs when the player hosts and a forwarder to the host
        // runs when they join (see the `lan` crate). It is kept apart from the
        // folders a player drops descriptors into, so it never shadows one.
        std::env::set_var("IW4L_COMMUNITY_ROOT", &config);
        if std::env::var_os("IW4L_COMMUNITY").is_none() {
            let builtin = config.join("lan.iw4l-server");
            match std::fs::write(&builtin, lan::descriptor()) {
                Ok(()) => std::env::set_var("IW4L_COMMUNITY", &builtin),
                Err(error) => eprintln!("LAN descriptor {}: {error}", builtin.display()),
            }
        }
    }

    // The desktop entry does this before launching. The selected community
    // descriptor is what makes a master reachable, so the Android entry must
    // select one too; on a non-Windows target this performs no update.
    if let Err(error) = updater::startup() {
        eprintln!("community startup: {error}");
    }

    bootstrap::launch(
        asset_transport::GamesRoot(games),
        artifacts,
        bootstrap::LaunchMode::Menu,
        None,
        sim::HostCheats(true),
    );
}

fn first_existing(candidates: &[String]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_dir())
}

/// Where the game install lives. Either location works: `media` is what a
/// player can drag in over USB, `files` is what `adb push` writes. Whichever
/// already holds a `zone/` wins; with neither, `media` is chosen so the folder
/// a player is told to fill is the one the game will read.
fn games_root(files: &Path, media: &Path) -> PathBuf {
    for base in [media, files] {
        let candidate = base.join("mw2");
        if candidate.join("zone").is_dir() {
            return candidate;
        }
    }
    media.join("mw2")
}

fn first_descriptor(base: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(base).ok()?;
    entries.flatten().map(|entry| entry.path()).find(|path| {
        path.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("iw4l-server"))
    })
}

// The input bridge: `GameActivity.java` keeps the input queue a NativeActivity
// would hand to native code and calls these instead, on its own thread. They
// only queue what happened; `touch::bridge` replays it into Bevy each frame.

type Jni = *mut core::ffi::c_void;

/// `phase`: 0 down, 1 move, 2 up, 3 cancel. `x`/`y`: fractions of the view.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativeTouch(
    _env: Jni,
    _class: Jni,
    id: i32,
    phase: i32,
    x: f32,
    y: f32,
) {
    use bevy::input::touch::TouchPhase;
    touch::push(touch::BridgeEvent::Touch {
        id: id as u64,
        phase: match phase {
            0 => TouchPhase::Started,
            1 => TouchPhase::Moved,
            2 => TouchPhase::Ended,
            _ => TouchPhase::Canceled,
        },
        x,
        y,
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativePadButton(
    _env: Jni,
    _class: Jni,
    device: i32,
    keycode: i32,
    down: u8,
) {
    touch::push(touch::BridgeEvent::PadButton {
        device,
        keycode,
        down: down != 0,
    });
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativePadAxes(
    _env: Jni,
    _class: Jni,
    device: i32,
    lx: f32,
    ly: f32,
    rx: f32,
    ry: f32,
    lt: f32,
    rt: f32,
    hat_x: f32,
    hat_y: f32,
) {
    touch::push(touch::BridgeEvent::PadAxes {
        device,
        axes: [lx, ly, rx, ry, lt, rt, hat_x, hat_y],
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativePadRemoved(
    _env: Jni,
    _class: Jni,
    device: i32,
) {
    touch::push(touch::BridgeEvent::PadRemoved { device });
}

/// The game view's size in pixels, and how many lines to render into it.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativeView(
    _env: Jni,
    _class: Jni,
    width: i32,
    height: i32,
    render_height: i32,
) {
    touch::set_view(
        width.max(0) as u32,
        height.max(0) as u32,
        render_height.max(0) as u32,
    );
}

/// The text prompt the game wants shown, or 0. Polled by the activity.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativeTakePrompt(
    _env: Jni,
    _class: Jni,
) -> i32 {
    touch::take_prompt() as i32
}

/// The vibration last taken by `nativeTakeRumble`: device, strong, weak, ms.
static TAKEN_RUMBLE: std::sync::Mutex<[f32; 4]> = std::sync::Mutex::new([0.0; 4]);

/// Take the vibration the game last asked for, once. Returns whether there was
/// one; its four values are then read with `nativeRumbleField`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativeTakeRumble(
    _env: Jni,
    _class: Jni,
) -> u8 {
    let Some((device, strong, weak, millis)) = touch::take_rumble() else {
        return 0;
    };
    if let Ok(mut taken) = TAKEN_RUMBLE.lock() {
        *taken = [device as f32, strong, weak, millis as f32];
    }
    1
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativeRumbleField(
    _env: Jni,
    _class: Jni,
    index: i32,
) -> f32 {
    TAKEN_RUMBLE
        .lock()
        .ok()
        .and_then(|taken| taken.get(usize::try_from(index).ok()?).copied())
        .unwrap_or(0.0)
}

/// A prompt's answer, one character at a time, then `nativeTypedDone`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativeTypedChar(
    _env: Jni,
    _class: Jni,
    character: i32,
) {
    touch::typed_char(character as u32);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativeTypedDone(
    _env: Jni,
    _class: Jni,
    kind: i32,
) {
    touch::typed_done(kind as u32);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_iw4j_iw4l_GameActivity_nativeBack(_env: Jni, _class: Jni) {
    touch::press_back();
}
