//! What the app asks of iOS once, at launch: stay awake, play sound, keep the
//! big folders out of backups.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use dispatch2::DispatchQueue;
use objc2::MainThreadMarker;
use objc2_avf_audio::{AVAudioSession, AVAudioSessionCategoryPlayback};
use objc2_foundation::{NSNumber, NSString, NSURL, NSURLIsExcludedFromBackupKey};
use objc2_ui_kit::UIApplication;

/// Set once UIKit's run loop is running and the application object exists.
static APP_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn app_running() -> bool {
    APP_RUNNING.load(Ordering::Acquire)
}

/// Keep a folder out of iCloud and computer backups. The game install is
/// gigabytes and can be copied again; the caches are rebuilt.
pub fn exclude_from_backup(path: &Path) {
    let Some(path) = path.to_str() else {
        return;
    };
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    let yes = NSNumber::numberWithBool(true);
    // SAFETY: the key is the documented one and the value is an `NSNumber`.
    unsafe {
        let _ = url.setResourceValue_forKey_error(Some(&yes), NSURLIsExcludedFromBackupKey);
    }
}

/// Queue the setup that needs the application object, which does not exist
/// until the windowing library has started UIKit.
pub fn after_launch() {
    DispatchQueue::main().exec_async(|| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        // A match has long stretches with no touch (a pad, a killcam); the
        // display must not time out under it.
        UIApplication::sharedApplication(mtm).setIdleTimerDisabled(true);
        configure_audio();
        APP_RUNNING.store(true, Ordering::Release);
    });
}

/// Sound plays with the ringer switch off, as it does on Android, and takes
/// the output while the game is in front.
fn configure_audio() {
    // SAFETY: the shared session is always valid on the main thread.
    unsafe {
        let session = AVAudioSession::sharedInstance();
        if let Some(category) = AVAudioSessionCategoryPlayback {
            let _ = session.setCategory_error(category);
        }
        let _ = session.setActive_error(true);
    }
}
