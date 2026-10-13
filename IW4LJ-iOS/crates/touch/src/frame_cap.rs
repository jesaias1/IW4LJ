//! The frame-rate cap.
//!
//! A phone that draws as many frames as it can runs hot, and a hot phone
//! throttles its clocks and plays worse for the rest of the session than a cool
//! one that was held at a steady 60. So the game is held to 60 frames a second
//! unless the player lifts the cap (the mod menu's UNCAP FPS in the Graphics
//! tab, for phones that can sustain more).
//!
//! The cap sleeps at the end of the main frame; it does not touch the render
//! schedule, vsync or the display's refresh rate.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use bevy::prelude::*;

/// Set from the mod menu: whether the cap is lifted.
static UNCAPPED: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_uncapped(uncapped: bool) {
    UNCAPPED.store(uncapped, Ordering::Relaxed);
}

/// The frame time the cap holds to. `IW4L_FPS_CAP=<n>` changes the rate (for
/// testing); `0` removes the cap.
fn target() -> Option<Duration> {
    static TARGET: std::sync::OnceLock<Option<Duration>> = std::sync::OnceLock::new();
    *TARGET.get_or_init(|| {
        let fps = std::env::var("IW4L_FPS_CAP")
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(60);
        (fps > 0).then(|| Duration::from_nanos(1_000_000_000 / u64::from(fps)))
    })
}

/// Hold the frame to the cap. Runs last in the main schedule.
pub(crate) fn limit(mut next: Local<Option<Instant>>) {
    let Some(frame) = target() else {
        return;
    };
    if UNCAPPED.load(Ordering::Relaxed) {
        *next = None;
        return;
    }
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
