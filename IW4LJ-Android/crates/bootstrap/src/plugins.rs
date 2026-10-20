use assets::AssetPlugin;
use audio::AudioPlugin;
use bevy::{
    log::LogPlugin,
    prelude::*,
    render::{
        RenderPlugin as BevyRenderPlugin,
        pipelined_rendering::PipelinedRenderingPlugin,
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
    },
};
use bots::BotsPlugin;
use console::ConsolePlugin;
use frame::RuntimeRole;
use hud::HudPlugin;
use net::NetPlugin;
use render::RenderPlugin;
use replay::ReplayPlugin;
use session::SessionPlugin;
use ui::UiPlugin;

pub fn add_runtime_plugins(app: &mut App) {
    add_runtime_plugins_with_role(app, RuntimeRole::Listen);
}

pub fn add_runtime_plugins_with_role(app: &mut App, role: RuntimeRole) {
    let net = match role {
        RuntimeRole::Listen => NetPlugin::listen(),
        RuntimeRole::Dedicated => NetPlugin::dedicated(),
        RuntimeRole::Client => NetPlugin::client(),
        RuntimeRole::Replay => NetPlugin::replay(),
    };
    if crate::bench::enabled() {
        // Unfocused benchmarks must not inherit the window runner's 60 Hz sleep.
        app.insert_resource(bevy::winit::WinitSettings::continuous());
    }
    app.insert_resource(audio::AudioRuntime::new(
        role != RuntimeRole::Dedicated && !audio::AudioSilent::active(),
    ));
    app.add_plugins(AssetPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(ConsolePlugin)
        .add_plugins(net)
        .add_plugins((BotsPlugin, HudPlugin))
        .add_plugins(AudioPlugin)
        .add_plugins(ReplayPlugin)
        .add_plugins(theater::TheaterPlugin)
        .add_plugins(RenderPlugin)
        .add_plugins(SessionPlugin);

    // A phone has no keyboard or mouse; the on-screen pad feeds the bind
    // system the input it already knows how to consume.
    #[cfg(target_os = "android")]
    app.add_plugins(touch::TouchPlugin);

    // A PC gets the same mod menu, opened with F1 and clicked with the mouse.
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    app.add_plugins(touch::DesktopMenuPlugin);

    if single_threaded_schedules() {
        // On a phone (Snapdragon 8 Gen 2) the parallel executor spends about a
        // tenth of all CPU time waking workers for systems that each run for
        // microseconds. `IW4L_SCHEDULE_THREADS=multi` restores it.
        use bevy::app::{
            First, FixedFirst, FixedLast, FixedPostUpdate, FixedPreUpdate, FixedUpdate, Last,
            PostUpdate, PreUpdate, Update,
        };
        macro_rules! in_order {
            ($($label:expr),+) => {$(
                app.edit_schedule($label, |schedule| {
                    schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
                });
            )+};
        }
        in_order!(
            First,
            PreUpdate,
            FixedFirst,
            FixedPreUpdate,
            FixedUpdate,
            FixedPostUpdate,
            FixedLast,
            Update,
            PostUpdate,
            Last
        );
    }

    if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
        // Measured: multithreaded render schedules compete with camera prepare's own
        // ComputeTaskPool lanes for the same workers and lose ~5% fps.
        render_app.edit_schedule(bevy::render::renderer::RenderGraph, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        render_app.edit_schedule(bevy::core_pipeline::Core3d, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        render_app.edit_schedule(bevy::core_pipeline::Core2d, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        render_app.edit_schedule(bevy::render::Render, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        render_app.edit_schedule(bevy::render::ExtractSchedule, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        if cfg!(target_os = "macos") && pipelined_rendering() {
            // The Metal surface must be created on the main thread; pipelined, only
            // extraction runs there.
            render_app.add_systems(
                bevy::render::ExtractSchedule,
                bevy::render::view::create_surfaces
                    .run_if(bevy::render::view::need_surface_configuration)
                    .after(bevy::render::camera::extract_cameras),
            );
        }
    }
}

pub fn assemble_listen_app() -> App {
    let mut app = App::new();
    app.add_plugins(default_plugins_with_quiet_log(WindowPlugin {
        primary_window: None,
        ..default()
    }));
    add_runtime_plugins(&mut app);
    app
}

pub fn default_plugins_with_quiet_log(mut window: WindowPlugin) -> bevy::app::PluginGroupBuilder {
    if let Some(primary) = window.primary_window.as_mut() {
        primary.desired_maximum_frame_latency = core::num::NonZeroU32::new(frame_latency());
    }
    #[allow(unused_mut)]
    let mut wgpu = WgpuSettings::default();
    // Bevy's default `Functionality` priority already enables every feature the
    // adapter reports. On a phone, explicitly requesting one the GPU lacks makes
    // `request_device` fail outright — a black screen before anything draws — so
    // Android asks for nothing extra and takes the adapter's own set. Desktop
    // keeps the explicit list the D3D9-era path was written against.
    #[cfg(not(target_os = "android"))]
    {
        wgpu.features |= WgpuFeatures::TEXTURE_FORMAT_16BIT_NORM
            | WgpuFeatures::TEXTURE_COMPRESSION_BC
            | WgpuFeatures::POLYGON_MODE_LINE
            | WgpuFeatures::TEXTURE_BINDING_ARRAY
            | WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
            | WgpuFeatures::PARTIALLY_BOUND_BINDING_ARRAY;
    }
    let plugins = DefaultPlugins
        .set(window)
        .set(LogPlugin {
            // On a phone the touch pad places the cursor for the game's menus;
            // winit cannot move a pointer there and says so on every tap.
            filter: if cfg!(target_os = "android") {
                "warn,iw4l=info,bevy_winit::system=off".into()
            } else {
                "warn,iw4l=info".into()
            },
            level: bevy::log::Level::WARN,
            ..default()
        })
        .set(BevyRenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(wgpu)),
            ..default()
        });
    if pipelined_rendering() {
        plugins
    } else {
        plugins.disable::<PipelinedRenderingPlugin>()
    }
}

/// Whether the main schedules run their systems in order on one thread
/// (`IW4L_SCHEDULE_THREADS=single`).
fn single_threaded_schedules() -> bool {
    match std::env::var("IW4L_SCHEDULE_THREADS").as_deref() {
        Ok("single") => true,
        Ok("multi") => false,
        // Measured on a Galaxy S23 with a cool phone: Afghan 30 fps with the
        // parallel executor, 66 with the systems run in order on one thread;
        // the parallel executor's wake-ups cost more than the parallelism
        // returns for systems that each run for microseconds.
        _ => cfg!(target_os = "android"),
    }
}

const PIPELINED_RENDERING_ENV: &str = "IW4L_PIPELINED_RENDERING";

/// Overlap rendering with the next main frame. Extraction remains the ownership
/// boundary; the bounded render channel permits one outstanding frame.
/// Set IW4L_PIPELINED_RENDERING=0 for synchronous presentation.
///
fn pipelined_rendering() -> bool {
    match std::env::var_os(PIPELINED_RENDERING_ENV) {
        None => !cfg!(target_os = "macos"),
        Some(_) => perf::switch(PIPELINED_RENDERING_ENV),
    }
}

const FRAME_LATENCY_ENV: &str = "IW4L_FRAME_LATENCY";

/// How many frames the surface is asked to let the CPU run ahead of the GPU.
///
/// wgpu calls this a hint and the backend is free to clamp it: on Vulkan it is
/// bound to the number of swapchain images, so a run that asked for two did
/// not necessarily get two, and only a measurement says which. One is the
/// default because it is what the runtime shipped; the variable exists so the
/// other arm needs no rebuild, and the manifest records the number that was
/// asked for — never the number the driver granted, which this process cannot
/// read back.
///
/// The value is a count, not a switch: anything unparseable or zero is the
/// default, and says so rather than silently picking an arm. Read once, so the
/// window and the manifest cannot disagree and the complaint is made once.
pub(crate) fn frame_latency() -> u32 {
    // Measured on a Galaxy S23 (Afghan): one frame in flight makes the CPU and
    // GPU take turns, about 20 fps; two lets them overlap, about 32.
    const DEFAULT: u32 = if cfg!(target_os = "android") { 2 } else { 1 };
    static FRAMES: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *FRAMES.get_or_init(|| {
        let Some(asked) = std::env::var_os(FRAME_LATENCY_ENV) else {
            return DEFAULT;
        };
        match asked.to_str().map(str::trim).and_then(|v| v.parse().ok()) {
            Some(frames) if frames > 0 => frames,
            _ => {
                diag::warn!(
                    Launch,
                    "{FRAME_LATENCY_ENV}={} is not a frame count; using {DEFAULT}",
                    asked.to_string_lossy(),
                );
                DEFAULT
            }
        }
    })
}
