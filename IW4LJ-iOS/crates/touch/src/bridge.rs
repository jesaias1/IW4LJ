//! Android input, handed over by `GameActivity` and replayed into Bevy.
//!
//! A `NativeActivity` normally gives its input queue to native code, where
//! winit reads touches and drops everything a controller says. The activity
//! keeps the queue instead and calls in here over JNI, so every device arrives
//! the same way and nothing is lost:
//!
//! - touches become [`TouchInput`], positioned in the window whatever size the
//!   surface is rendered at;
//! - a controller becomes a Bevy gamepad, which the runtime already plays with
//!   (sticks, aim, menus, prompts) on a desktop;
//! - the Back key is read by the pad as `Escape`.
//!
//! JNI calls arrive on the activity's thread; they only append to a queue that
//! [`pump`] drains at the start of a frame.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::input::gamepad::{
    GamepadAxis, GamepadButton, GamepadConnection, GamepadConnectionEvent,
    RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// One thing the activity saw.
pub enum BridgeEvent {
    /// `x` and `y` are fractions of the game view, `y` downward.
    Touch {
        id: u64,
        phase: TouchPhase,
        x: f32,
        y: f32,
    },
    /// `keycode` is an Android `KEYCODE_*`.
    PadButton {
        device: i32,
        keycode: i32,
        down: bool,
    },
    /// Left stick, right stick, triggers, hat — Android's signs (`y` downward).
    PadAxes { device: i32, axes: [f32; 8] },
    PadRemoved { device: i32 },
    /// A console line, from text typed into the activity's own prompt.
    Line(String),
}

/// A text prompt the mod menu wants the activity to show (0 is none). The game
/// has no text field a phone keyboard can type into, so the activity shows a
/// native dialog and sends back what was typed.
static PROMPT: AtomicU32 = AtomicU32::new(0);
static TYPED: Mutex<String> = Mutex::new(String::new());

/// The prompt that asks for the player's name.
pub const PROMPT_NAME: u32 = 1;
/// The prompt that asks for a LAN host's IP address.
pub const PROMPT_HOST_IP: u32 = 2;

pub(crate) fn request_prompt(kind: u32) {
    PROMPT.store(kind, Ordering::Relaxed);
}

/// Polled by the activity: the prompt to show, once.
pub fn take_prompt() -> u32 {
    PROMPT.swap(0, Ordering::Relaxed)
}

/// One character of a prompt's answer; [`typed_done`] ends it.
pub fn typed_char(character: u32) {
    if let (Some(character), Ok(mut typed)) = (char::from_u32(character), TYPED.lock()) {
        typed.push(character);
    }
}

pub fn typed_done(kind: u32) {
    let Ok(mut typed) = TYPED.lock() else {
        return;
    };
    let text = std::mem::take(&mut *typed);
    if kind == PROMPT_HOST_IP {
        // An address, with a port after a colon if the host did not use the usual one.
        let text = text.trim();
        let (address, port) = match text.split_once(':') {
            Some((address, port)) => (address, port.trim().parse::<u16>().ok()),
            None => (text, Some(lan::RELAY_PORT)),
        };
        match (address.trim().parse::<std::net::Ipv4Addr>(), port) {
            (Ok(host), Some(port)) => lan::join_at(host, port),
            _ => diag::warn!(Launch, "lan: `{text}` is not an IPv4 address (optionally :port)"),
        }
        return;
    }
    if kind == PROMPT_NAME {
        // A name is 15 bytes on the wire and one console argument here.
        let mut name = String::new();
        for character in text.trim().chars() {
            let plain = character.is_ascii_graphic() || character == ' ';
            if plain
                && !matches!(character, '"' | ';' | '\\')
                && name.len() + character.len_utf8() <= 15
            {
                name.push(character);
            }
        }
        if !name.is_empty() {
            push(BridgeEvent::Line(format!("seta ui_player_name \"{name}\"")));
        }
    }
}

static QUEUE: Mutex<Vec<BridgeEvent>> = Mutex::new(Vec::new());
static BACK_PRESSES: AtomicU32 = AtomicU32::new(0);

pub fn push(event: BridgeEvent) {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.push(event);
    }
}

/// The system Back key (or gesture) was pressed.
pub fn press_back() {
    BACK_PRESSES.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn take_back_presses() -> u32 {
    BACK_PRESSES.swap(0, Ordering::Relaxed)
}

/// The game view's real size in pixels and the number of lines to render.
static VIEW: Mutex<Option<(u32, u32, u32)>> = Mutex::new(None);

/// The activity laid its view out (again).
pub fn set_view(width: u32, height: u32, render_height: u32) {
    if let Ok(mut view) = VIEW.lock() {
        *view = Some((width, height, render_height));
    }
}

/// Render below the panel's resolution.
///
/// The window is told it is smaller than the surface it draws to; the swapchain
/// is created at that size and Android's compositor scales it to the screen
/// for free. A phone GPU that cannot shade every pixel of its panel at a
/// playable rate can shade half of them. The logical size is held at 720 lines
/// so the interface is laid out the same at every setting.
pub(crate) fn render_size(
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut resized: MessageWriter<bevy::window::WindowResized>,
) {
    let Some((view_width, view_height, render_height)) = VIEW.lock().ok().and_then(|view| *view)
    else {
        return;
    };
    // The view is portrait for a moment while the activity rotates.
    if view_width <= view_height || view_height == 0 {
        return;
    }
    let Ok((entity, mut window)) = windows.single_mut() else {
        return;
    };
    let height = render_height.clamp(360, view_height);
    // Even, so every render target divides cleanly.
    let width = (u64::from(view_width) * u64::from(height) / u64::from(view_height)) as u32 & !1;
    let scale = height as f32 / 720.0;
    if window.physical_width() == width
        && window.physical_height() == height
        && window.resolution.scale_factor_override() == Some(scale)
    {
        return;
    }
    window.resolution.set_scale_factor_override(Some(scale));
    window.resolution.set_physical_resolution(width, height);
    resized.write(bevy::window::WindowResized {
        window: entity,
        width: window.width(),
        height: window.height(),
    });
    diag::info!(
        Launch,
        "render size: {width}x{height} on a {view_width}x{view_height} view"
    );
}

fn button(keycode: i32) -> Option<GamepadButton> {
    Some(match keycode {
        96 => GamepadButton::South,
        97 => GamepadButton::East,
        99 => GamepadButton::West,
        100 => GamepadButton::North,
        102 => GamepadButton::LeftTrigger,
        103 => GamepadButton::RightTrigger,
        104 => GamepadButton::LeftTrigger2,
        105 => GamepadButton::RightTrigger2,
        106 => GamepadButton::LeftThumb,
        107 => GamepadButton::RightThumb,
        108 => GamepadButton::Start,
        109 => GamepadButton::Select,
        110 => GamepadButton::Mode,
        19 => GamepadButton::DPadUp,
        20 => GamepadButton::DPadDown,
        21 => GamepadButton::DPadLeft,
        22 => GamepadButton::DPadRight,
        // A pad with no key layout of its own sends numbered buttons, in the
        // order its face prints them (KEYCODE_BUTTON_1 is 188): the usual
        // DirectInput arrangement.
        188 => GamepadButton::West,
        189 => GamepadButton::South,
        190 => GamepadButton::East,
        191 => GamepadButton::North,
        192 => GamepadButton::LeftTrigger,
        193 => GamepadButton::RightTrigger,
        194 => GamepadButton::LeftTrigger2,
        195 => GamepadButton::RightTrigger2,
        196 => GamepadButton::Select,
        197 => GamepadButton::Start,
        198 => GamepadButton::LeftThumb,
        199 => GamepadButton::RightThumb,
        200 => GamepadButton::Mode,
        _ => return None,
    })
}

/// The motor levels the game last asked for, taken by the activity.
struct Rumble {
    device: i32,
    strong: f32,
    weak: f32,
    millis: u32,
    serial: u64,
    taken: u64,
}

static RUMBLE: Mutex<Rumble> = Mutex::new(Rumble {
    device: -1,
    strong: 0.0,
    weak: 0.0,
    millis: 0,
    serial: 0,
    taken: 0,
});

/// The activity polls this: a new rumble request as `(device, strong, weak,
/// milliseconds)`, once.
pub fn take_rumble() -> Option<(i32, f32, f32, u32)> {
    let mut rumble = RUMBLE.lock().ok()?;
    if rumble.serial == rumble.taken {
        return None;
    }
    rumble.taken = rumble.serial;
    Some((rumble.device, rumble.strong, rumble.weak, rumble.millis))
}

fn set_rumble(device: i32, strong: f32, weak: f32, millis: u32) {
    if let Ok(mut rumble) = RUMBLE.lock() {
        *rumble = Rumble {
            device,
            strong,
            weak,
            millis,
            serial: rumble.serial + 1,
            taken: rumble.taken,
        };
    }
}

#[derive(Default)]
pub(crate) struct Pads {
    entities: HashMap<i32, Entity>,
    /// Last value sent for a device's axis or analog button, so a report that
    /// repeats a value is not an event.
    axes: HashMap<(i32, GamepadAxis), f32>,
    buttons: HashMap<(i32, GamepadButton), f32>,
    /// Devices whose triggers or d-pad have been seen as axes; from then on the
    /// axis is the truth, including its return to rest.
    analog_triggers: HashSet<i32>,
    hats: HashSet<i32>,
    /// Whether Select is held on a device, while the trickshot teleport is on.
    select_held: HashMap<i32, bool>,
    /// Events for a pad connected this frame. Bevy gives the entity its
    /// `Gamepad` when the connection is applied, and drops input for an entity
    /// that has none yet, so they wait one frame.
    held: Vec<BridgeEvent>,
}

static TP_PAD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether Select + d-pad is the trickshot teleport (see the TRICKSHOT tab).
pub(crate) fn tp_pad() -> bool {
    TP_PAD.load(std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn set_tp_pad(on: bool) {
    TP_PAD.store(on, std::sync::atomic::Ordering::Relaxed);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn pump(
    mut commands: Commands,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut touches: MessageWriter<TouchInput>,
    mut raw: MessageWriter<RawGamepadEvent>,
    mut connections: MessageWriter<GamepadConnectionEvent>,
    mut button_events: MessageWriter<RawGamepadButtonChangedEvent>,
    mut axis_events: MessageWriter<RawGamepadAxisChangedEvent>,
    mut queue: ResMut<console::ConsoleQueue>,
    mut rumble_requests: MessageReader<bevy::input::gamepad::GamepadRumbleRequest>,
    mut pads: Local<Pads>,
) {
    let pads = &mut *pads;
    // The game asks for vibration the way it would of a gilrs pad; here the
    // request goes to the activity, which owns the phone's vibrators.
    for request in rumble_requests.read() {
        use bevy::input::gamepad::GamepadRumbleRequest as Request;
        let (gamepad, strong, weak, millis) = match request {
            Request::Add {
                gamepad,
                duration,
                intensity,
            } => (
                *gamepad,
                intensity.strong_motor,
                intensity.weak_motor,
                duration.as_millis().min(5_000) as u32,
            ),
            Request::Stop { gamepad } => (*gamepad, 0.0, 0.0, 0),
        };
        if let Some((&device, _)) = pads.entities.iter().find(|(_, entity)| **entity == gamepad) {
            set_rumble(device, strong, weak, millis);
        }
    }
    let mut events = std::mem::take(&mut pads.held);
    if let Ok(mut queue) = QUEUE.lock() {
        events.append(&mut queue);
    }
    if events.is_empty() {
        return;
    }
    let window = windows.single().ok();
    let mut fresh: Vec<i32> = Vec::new();

    for event in events {
        let device = match &event {
            BridgeEvent::Touch { id, phase, x, y } => {
                if let Some((entity, window)) = window {
                    touches.write(TouchInput {
                        phase: *phase,
                        position: Vec2::new(x * window.width(), y * window.height()),
                        window: entity,
                        force: None,
                        id: *id,
                    });
                }
                continue;
            }
            BridgeEvent::PadRemoved { device } => {
                if let Some(entity) = pads.entities.remove(device) {
                    let gone = GamepadConnectionEvent::new(entity, GamepadConnection::Disconnected);
                    raw.write(gone.clone().into());
                    connections.write(gone);
                    pads.axes.retain(|(owner, _), _| owner != device);
                    pads.buttons.retain(|(owner, _), _| owner != device);
                }
                continue;
            }
            BridgeEvent::Line(line) => {
                queue.push_line(line);
                continue;
            }
            BridgeEvent::PadButton { device, .. } | BridgeEvent::PadAxes { device, .. } => *device,
        };

        if !pads.entities.contains_key(&device) {
            let entity = commands.spawn_empty().id();
            pads.entities.insert(device, entity);
            let arrived = GamepadConnectionEvent::new(
                entity,
                GamepadConnection::Connected {
                    name: "Android controller".to_owned(),
                    vendor_id: None,
                    product_id: None,
                },
            );
            raw.write(arrived.clone().into());
            connections.write(arrived);
            fresh.push(device);
        }
        if fresh.contains(&device) {
            pads.held.push(event);
            continue;
        }
        let entity = pads.entities[&device];

        let mut press = |pads: &mut Pads,
                         raw: &mut MessageWriter<RawGamepadEvent>,
                         button: GamepadButton,
                         value: f32| {
            if pads.buttons.insert((device, button), value) == Some(value) {
                return;
            }
            let changed = RawGamepadButtonChangedEvent::new(entity, button, value);
            raw.write(changed.into());
            button_events.write(changed);
        };
        match event {
            BridgeEvent::PadButton { keycode, down, .. } => {
                if let Some(button) = button(keycode) {
                    // While the trickshot teleport is on, Select is a modifier
                    // that never reaches the game: Select + d-pad left saves,
                    // Select + d-pad right returns. The sticks stay the game's
                    // (sprint, stance), and so does every other button.
                    if tp_pad() {
                        if button == GamepadButton::Select {
                            pads.select_held.insert(device, down);
                            continue;
                        }
                        let select = pads.select_held.get(&device).copied().unwrap_or(false);
                        if select && matches!(button, GamepadButton::DPadLeft | GamepadButton::DPadRight) {
                            if down {
                                queue.push_line(if button == GamepadButton::DPadLeft { "save" } else { "load" });
                            }
                            continue;
                        }
                    }
                    press(pads,&mut raw, button, f32::from(u8::from(down)));
                }
            }
            BridgeEvent::PadAxes { axes, .. } => {
                let [lx, ly, rx, ry, lt, rt, hat_x, hat_y] = axes;
                // Android reports a stick's `y` downward; a gamepad's is upward.
                for (axis, value) in [
                    (GamepadAxis::LeftStickX, lx),
                    (GamepadAxis::LeftStickY, -ly),
                    (GamepadAxis::RightStickX, rx),
                    (GamepadAxis::RightStickY, -ry),
                ] {
                    if pads.axes.insert((device, axis), value) == Some(value) {
                        continue;
                    }
                    let changed = RawGamepadAxisChangedEvent::new(entity, axis, value);
                    raw.write(changed.into());
                    axis_events.write(changed);
                }
                // A pad that has analog triggers reports them here and may not
                // send the button at all; one that sends both agrees with itself.
                if lt > 0.0 || pads.analog_triggers.contains(&device) {
                    pads.analog_triggers.insert(device);
                    press(pads,&mut raw, GamepadButton::LeftTrigger2, lt);
                }
                if rt > 0.0 || pads.analog_triggers.contains(&device) {
                    pads.analog_triggers.insert(device);
                    press(pads,&mut raw, GamepadButton::RightTrigger2, rt);
                }
                // A d-pad that arrives as a hat axis rather than as buttons.
                if hat_x != 0.0 || hat_y != 0.0 || pads.hats.contains(&device) {
                    pads.hats.insert(device);
                    for (button, held) in [
                        (GamepadButton::DPadLeft, hat_x < -0.5),
                        (GamepadButton::DPadRight, hat_x > 0.5),
                        (GamepadButton::DPadUp, hat_y < -0.5),
                        (GamepadButton::DPadDown, hat_y > 0.5),
                    ] {
                        if tp_pad()
                            && matches!(button, GamepadButton::DPadLeft | GamepadButton::DPadRight)
                            && pads.select_held.get(&device).copied().unwrap_or(false)
                        {
                            let value = f32::from(u8::from(held));
                            if pads.buttons.insert((device, button), value) != Some(value) && held {
                                queue.push_line(if button == GamepadButton::DPadLeft { "save" } else { "load" });
                            }
                            continue;
                        }
                        press(pads,&mut raw, button, f32::from(u8::from(held)));
                    }
                }
            }
            BridgeEvent::Touch { .. } | BridgeEvent::PadRemoved { .. } | BridgeEvent::Line(_) => {}
        }
    }
}
