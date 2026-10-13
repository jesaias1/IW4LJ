//! iOS entry point.
//!
//! An iOS app is an executable the system starts, like the desktop `iw4l`, but
//! its files live in a sandbox. The player sees one folder of it, `Documents`,
//! through the Files app (and Finder or the Apple Devices app on a computer),
//! so that is where the game install, the community descriptor and the
//! settings files go: the same arrangement Android has under its `files`
//! directory. Everything after the folders is the startup the Android port
//! shares ([`mobile`]).

#[cfg(target_os = "ios")]
mod gamepad;
#[cfg(target_os = "ios")]
mod notice;
#[cfg(target_os = "ios")]
mod platform;
#[cfg(target_os = "ios")]
mod prompt;
#[cfg(target_os = "ios")]
mod rumble;

#[cfg(target_os = "ios")]
use std::path::PathBuf;

/// Start the game. Returns when the runtime exits.
#[cfg(target_os = "ios")]
pub fn run() {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/mobile"));
    let documents = home.join("Documents");
    let _ = std::fs::create_dir_all(&documents);
    // Settings, the profile and the account live where the player cannot trip
    // over them and a backup does not carry them to another device by accident.
    let config = home.join("Library").join("Application Support").join("iw4l");
    let dirs = mobile::Dirs {
        files: documents.clone(),
        media: documents,
        config,
    };

    // Caches and the game install are far too big for a backup. The game
    // folder is created empty so the player has somewhere to put it.
    let games = dirs.files.join("mw2");
    let _ = std::fs::create_dir_all(&games);
    platform::exclude_from_backup(&games);
    platform::exclude_from_backup(&dirs.files.join("iw4l-artifacts"));

    // Anything the runtime reports as fatal reaches the player as a message
    // instead of a window that closes by itself.
    notice::install();

    // Runs once UIKit is up, because the application object does not exist yet.
    platform::after_launch();

    gamepad::start();
    prompt::start();
    rumble::start();

    mobile::launch(&dirs);
}
