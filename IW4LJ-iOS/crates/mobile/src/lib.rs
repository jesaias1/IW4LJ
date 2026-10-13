//! Startup shared by the phone ports.
//!
//! Android and iOS differ in where a phone keeps its files and in how input and
//! windows reach the runtime, and nothing else. A port resolves its directories
//! ([`Dirs`]) and calls [`launch`]; everything from the environment variables
//! the runtime reads to the community descriptor that makes LAN play work is
//! here, once, so a change to it is a change to both ports.

use std::path::{Path, PathBuf};

/// Where a port keeps things.
pub struct Dirs {
    /// The folder the player can open with a file manager: settings written by
    /// the mod menu, `cmds.txt`, `env.txt`, `.iw4l-server` descriptors, and the
    /// game install under `mw2/` when it is not in `media`.
    pub files: PathBuf,
    /// A second folder the player can drop things into (on Android the shared
    /// `Android/media/<package>` one, which a file manager can write to). A
    /// port with only one such folder passes the same path twice.
    pub media: PathBuf,
    /// Private storage for the profile, the account and generated descriptors.
    pub config: PathBuf,
}

/// Where the game install lives. Either location works: `media` is what a
/// player can drag in over USB, `files` is what `adb push` writes. Whichever
/// already holds a `zone/` wins; with neither, `media` is chosen so the folder
/// a player is told to fill is the one the game will read.
pub fn games_root(files: &Path, media: &Path) -> PathBuf {
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

/// Prepare the process environment for the runtime. Returns the game install
/// and the artifacts directory.
///
/// Call it before any other thread exists: it sets environment variables.
pub fn prepare(dirs: &Dirs) -> (PathBuf, PathBuf) {
    let Dirs {
        files,
        media,
        config,
    } = dirs;
    let games = games_root(files, media);
    let artifacts = files.join("iw4l-artifacts");
    if let Err(error) = std::fs::create_dir_all(config) {
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
        std::env::set_var("HOME", files);
        std::env::set_var("XDG_CONFIG_HOME", config);
        // `env.txt` beside the game: `NAME=value` lines set before anything
        // reads the environment. A phone has no shell to launch from, and
        // every tuning switch in the runtime is an environment variable.
        for base in [media, files] {
            let Ok(text) = std::fs::read_to_string(base.join("env.txt")) else {
                continue;
            };
            for line in text.lines() {
                if let Some((name, value)) = line.split_once('=')
                    && !name.trim().is_empty()
                    && !name.trim_start().starts_with('#')
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
            let script = [media, files]
                .into_iter()
                .find_map(|base| std::fs::read_to_string(base.join("cmds.txt")).ok())
                .filter(|text| !text.trim().is_empty())
                .unwrap_or_else(|| "wait progression; unlock all".to_owned());
            std::env::set_var("IW4L_CMDS", script);
        }
        // A `.iw4l-server` descriptor dropped next to the game selects the
        // community master, so two phones on one network can meet without a
        // console. Discovery has to be told where to look: this machine's files
        // directory is not the launcher's own directory.
        if std::env::var_os("IW4L_COMMUNITY").is_none() {
            for base in [media, files] {
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
        std::env::set_var("IW4L_COMMUNITY_ROOT", config);
        if std::env::var_os("IW4L_COMMUNITY").is_none() {
            let builtin = config.join("lan.iw4l-server");
            match std::fs::write(&builtin, lan::descriptor()) {
                Ok(()) => std::env::set_var("IW4L_COMMUNITY", &builtin),
                Err(error) => eprintln!("LAN descriptor {}: {error}", builtin.display()),
            }
        }
    }
    (games, artifacts)
}

/// Prepare the environment and hand control to the runtime. Returns when the
/// runtime exits.
pub fn launch(dirs: &Dirs) {
    let (games, artifacts) = prepare(dirs);

    // The desktop entry does this before launching. The selected community
    // descriptor is what makes a master reachable, so a phone entry must select
    // one too; on a non-Windows target this performs no update.
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
