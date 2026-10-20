//! The frame-rate cap.
//!
//! A phone that draws as many frames as it can runs hot, and a hot phone
//! throttles its clocks and plays worse for the rest of the session than a cool
//! one that was held at a steady 60. So the game is held to 60 frames a second
//! unless the player picks 90 or 120 (the mod menu's FPS buttons in the
//! Graphics tab, for phones that can sustain more).
//!
//! The cap sleeps at the end of the main frame; it does not touch the render
//! schedule, vsync or the display's refresh rate.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::time::{Duration, Instant};

use bevy::prelude::*;

/// Set from the mod menu: the frame rate to hold (60, 90 or 120).
static CAP_FPS: AtomicU32 = AtomicU32::new(60);

/// How hot the device says it is: 0 nominal, 1 fair, 2 serious, 3 critical
/// (iOS's thermal states). Serious holds the 60 fps cap even if the player
/// picked 90 or 120; critical halves it.
static THERMAL: AtomicU8 = AtomicU8::new(0);

pub fn set_thermal_level(level: u8) {
    THERMAL.store(level, Ordering::Relaxed);
}

pub(crate) fn set_cap(fps: u32) {
    CAP_FPS.store(fps, Ordering::Relaxed);
}

/// The frame time the cap holds to. `IW4L_FPS_CAP=<n>` overrides the menu (for
/// testing); `0` removes the cap.
fn target() -> Option<Duration> {
    static ENV: std::sync::OnceLock<Option<u32>> = std::sync::OnceLock::new();
    let fps = ENV
        .get_or_init(|| {
            std::env::var("IW4L_FPS_CAP")
                .ok()
                .and_then(|value| value.trim().parse::<u32>().ok())
        })
        .unwrap_or_else(|| CAP_FPS.load(Ordering::Relaxed));
    (fps > 0).then(|| Duration::from_nanos(1_000_000_000 / u64::from(fps)))
}

/// Hold the frame to the cap. Runs last in the main schedule.
pub(crate) fn limit(mut next: Local<Option<Instant>>) {
    let thermal = THERMAL.load(Ordering::Relaxed);
    let sixty = Duration::from_nanos(1_000_000_000 / 60);
    let frame = match (target(), thermal) {
        (_, 3..) => sixty * 2,
        (Some(frame), 2) => frame.max(sixty),
        (Some(frame), _) => frame,
        (None, 2) => sixty,
        (None, _) => return,
    };
    let now = Instant::now();
    let due = match *next {
        Some(due) => due,
        None => {
            *next = Some(now + frame);
            return;
        }
    };
    if now >= due {
        // On time or late. Far behind (a load, the menu opening) means the
        // schedule restarts from here instead of racing to catch up.
        *next = Some(if now - due > frame { now + frame } else { due + frame });
        return;
    }
    // Sleep most of the wait, then spin the last stretch: a sleep returns a
    // little late, and a late frame is a visible stutter.
    let spin = Duration::from_micros(1500);
    let wait = due - now;
    if wait > spin {
        std::thread::sleep(wait - spin);
    }
    while Instant::now() < due {
        std::hint::spin_loop();
    }
    *next = Some(due + frame);
}
