#![allow(deprecated)]
//! What the app asks of iOS once, at launch: stay awake, play sound, keep the
//! big folders out of backups.

use std::path::Path;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::MainThreadMarker;
use objc2_avf_audio::{
    AVAudioSession, AVAudioSessionCategoryPlayback, AVAudioSessionInterruptionNotification,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSNotificationName, NSNumber, NSOperationQueue,
    NSProcessInfo, NSString, NSURL, NSURLIsExcludedFromBackupKey,
};
use objc2_ui_kit::{
    UIApplication, UIApplicationDidBecomeActiveNotification,
    UIApplicationDidReceiveMemoryWarningNotification,
};

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
        observe_system();
        APP_RUNNING.store(true, Ordering::Release);
    });
    watch_device();
}

fn observe(name: &NSNotificationName, handler: impl Fn() + 'static) {
    let block = RcBlock::new(move |_note: NonNull<NSNotification>| handler());
    // SAFETY: the name is the framework's own; the observer lives as long as the
    // process, so it is never removed.
    unsafe {
        let token = NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(name),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &block,
        );
        std::mem::forget(token);
    }
}

/// What the system tells the app about itself.
fn observe_system() {
    // A call, an alarm or another app's sound stops the output; the runtime's
    // audio supervisor notices the stall and reopens it, which only works once
    // the session is active again.
    // SAFETY: the notification names are the framework's own.
    unsafe {
        if let Some(name) = AVAudioSessionInterruptionNotification {
            observe(name, reactivate_audio);
        }
        observe(UIApplicationDidBecomeActiveNotification, reactivate_audio);
        observe(UIApplicationDidReceiveMemoryWarningNotification, || {
            diag::warn!(Launch, "ios: memory warning ({} MiB left)", headroom_mib());
        });
    }
}

fn reactivate_audio() {
    // SAFETY: the shared session is always valid.
    unsafe {
        let _ = AVAudioSession::sharedInstance().setActive_error(true);
    }
}

/// How much memory the app may still use before iOS ends it.
fn headroom_mib() -> usize {
    unsafe extern "C" {
        fn os_proc_available_memory() -> usize;
    }
    // SAFETY: a plain query with no arguments (iOS 13 and later).
    unsafe { os_proc_available_memory() >> 20 }
}

/// Once a second, on the main thread: the safe area (it changes when the phone
/// is turned over) and how hot the phone says it is.
fn watch_device() {
    let spawned = std::thread::Builder::new()
        .name("iw4l-device".into())
        .spawn(|| {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                DispatchQueue::main().exec_async(read_device);
            }
        });
    if let Err(error) = spawned {
        eprintln!("device thread: {error}");
    }
}

fn read_device() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    // Nominal, fair, serious, critical: 0 to 3.
    let thermal = NSProcessInfo::processInfo().thermalState().0 as u8;
    touch::set_thermal_level(thermal);
    let windows = UIApplication::sharedApplication(mtm).windows();
    if let Some(window) = windows.firstObject() {
        let insets = window.safeAreaInsets();
        touch::set_safe_insets(
            insets.left as f32,
            insets.right as f32,
            window.bounds().size.height as f32,
        );
    }
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
