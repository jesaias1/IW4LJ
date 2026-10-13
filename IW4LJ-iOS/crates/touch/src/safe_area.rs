//! The screen's safe area: the rounded corners and camera cut-out a phone's
//! controls must stay clear of.
//!
//! A port reports the left and right insets and the screen's height, all in the
//! same units (iOS: points); the layout takes them as fractions of the height,
//! so they hold at any render resolution.

use std::sync::Mutex;

static INSETS: Mutex<(f32, f32)> = Mutex::new((0.0, 0.0));

/// The side insets and the screen height, in the same units.
pub fn set_safe_insets(left: f32, right: f32, height: f32) {
    if height <= 0.0 {
        return;
    }
    if let Ok(mut insets) = INSETS.lock() {
        *insets = (
            (left / height).clamp(0.0, 0.25),
            (right / height).clamp(0.0, 0.25),
        );
    }
}

/// Left and right insets as fractions of the screen's height.
pub(crate) fn fractions() -> (f32, f32) {
    INSETS.lock().map(|insets| *insets).unwrap_or((0.0, 0.0))
}
