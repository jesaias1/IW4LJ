//! iOS: the view's size, and touches that winit delivers on its own.
//!
//! Android hands every touch over a bridge, as fractions of the view; on iOS
//! winit already delivers them, in physical pixels of the real screen. The game
//! draws below the panel's resolution and holds its interface at 720 lines (see
//! `bridge::render_size`), so a touch has to be put back into that space before
//! the pad reads it.

use bevy::ecs::system::NonSendMarker;
use bevy::input::touch::TouchInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::bridge;
use crate::menu::{Setting, read_setting};

/// The lines the game renders at when nothing is set: the device's memory stands
/// in for how strong its GPU is.
fn default_render_height() -> u32 {
    let mut bytes: u64 = 0;
    let mut size = std::mem::size_of::<u64>();
    // SAFETY: `hw.memsize` is a 64-bit integer and `size` says so.
    let status = unsafe {
        libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&raw mut bytes).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    let mebibytes = if status == 0 { bytes >> 20 } else { 0 };
    if mebibytes >= 10_500 {
        900
    } else if mebibytes >= 6_500 {
        720
    } else {
        540
    }
}

fn render_height() -> u32 {
    static HEIGHT: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *HEIGHT.get_or_init(|| match read_setting(Setting::RenderHeight) {
        height if height > 0 => (height as u32).clamp(360, 2160),
        _ => default_render_height(),
    })
}

/// Tell the bridge how big the real view is. It is the one place the native
/// size can still be read once the window has been told it is smaller.
pub(crate) fn report_view(
    _main_thread: NonSendMarker,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut last: Local<(u32, u32)>,
) {
    let Ok(entity) = windows.single() else {
        return;
    };
    let size = bevy::winit::WINIT_WINDOWS
        .with_borrow(|winit| winit.get_window(entity).map(|window| window.inner_size()));
    let Some(size) = size else {
        return;
    };
    if (size.width, size.height) == *last || size.width == 0 || size.height == 0 {
        return;
    }
    *last = (size.width, size.height);
    bridge::set_view(size.width, size.height, render_height());
}

/// Put every touch of this frame where the render size says it is.
///
/// winit converts a touch with the window's scale factor, which is the one
/// `render_size` overrides, so the position here is the real one in pixels
/// divided by that factor. The real one as a fraction of the view, times the
/// window's logical size, is what the pad expects.
pub(crate) fn remap_touches(
    mut touches: ResMut<Messages<TouchInput>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut raw_view: Local<Option<Vec2>>,
) {
    if let Some(view) = bridge::view_size() {
        *raw_view = Some(view);
    }
    let (Some(view), Ok(window)) = (*raw_view, windows.single()) else {
        return;
    };
    if touches.is_empty() {
        return;
    }
    let scale = window.resolution.scale_factor();
    let logical = Vec2::new(window.width(), window.height());
    let pending: Vec<TouchInput> = touches.drain().collect();
    for mut touch in pending {
        let pixels = touch.position * scale;
        touch.position = (pixels / view) * logical;
        touches.write(touch);
    }
}
