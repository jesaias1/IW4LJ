//! Touch controls, the mod menu, and the Android and iOS input bridges.
//!
//! The runtime already turns keyboard and mouse state into gameplay through
//! `console`'s input system: `ButtonInput<KeyCode>`, `ButtonInput<MouseButton>`
//! and `MouseMotion`, applied against the default control bindings. This plugin
//! invents no new input path â€” it draws an on-screen pad and presses exactly
//! those keys and buttons, and emits exactly those mouse deltas, so the rest of
//! the runtime is unaware it is not a desktop.
//!
//! A touch means one of three things, decided by what is on screen:
//!
//! - in play, it is the pad: a floating move stick on the left, a view drag on
//!   the right, and the buttons;
//! - with one of the game's own menus up, it is a mouse: the finger is the
//!   cursor and a tap is a click, which is what those menus already read;
//! - with the mod menu open, it belongs to the mod menu ([`menu`]).
//!
//! Keys pressed here are held only while their control is touched, so a virtual
//! button behaves like a physical one; the previous set is diffed each frame so
//! `just_pressed`/`just_released` stay true edges.
//!
//! A controller does not come through here at all: [`bridge`] hands it to Bevy
//! as a gamepad, and the pad simply gets out of the way while one is in use.

mod bridge;
mod frame_cap;
#[cfg(target_os = "ios")]
mod ios;
mod killcam_bars;
mod menu;
mod skins;

use std::collections::{HashMap, HashSet};

use bevy::input::InputSystems;
use bevy::input::mouse::{MouseButton, MouseMotion};
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::{Display, GlobalZIndex};
use bevy::window::PrimaryWindow;

pub use bridge::{
    BridgeEvent, press_back, push, set_view, take_prompt, take_rumble, typed_char, typed_done,
};

/// What a drawn control does while it is held.
#[derive(Clone, Copy, PartialEq)]
enum Control {
    Key(KeyCode),
    Mouse(MouseButton),
    /// Opens the mod menu.
    Mods,
}

/// Binds the mouse side button to "aim while held", for a controller or mouse
/// that wants hold-to-aim instead of the default toggle.
const BIND_AIM_HOLD: &str = "bind MOUSE4 +speed_throw";

/// When a control is on screen.
#[derive(Clone, Copy, PartialEq)]
enum Shown {
    /// Only while the player is in control of a soldier.
    InPlay,
    /// On every screen: the way into the mod menu.
    Always,
}

struct ButtonDef {
    /// Which screen edges `at` is measured from.
    right: bool,
    bottom: bool,
    /// Centre and radius in hundredths of the window height, so the layout is
    /// the same physical size at any render resolution.
    at: (f32, f32),
    radius: f32,
    control: Control,
    label: &'static str,
    shown: Shown,
    /// The primary action gets the accent ring.
    accent: bool,
}

const fn play(
    right: bool,
    bottom: bool,
    at: (f32, f32),
    radius: f32,
    control: Control,
    label: &'static str,
) -> ButtonDef {
    ButtonDef {
        right,
        bottom,
        at,
        radius,
        control,
        label,
        shown: Shown::InPlay,
        accent: false,
    }
}

/// A small button in the row along the top, counted from the right edge.
const fn top(slot: f32, control: Control, label: &'static str) -> ButtonDef {
    play(true, false, (7.0 + 11.0 * slot, 7.0), 4.3, control, label)
}

/// The keys the default bindings use (`console/assets/default_controls.cfg`),
/// laid out the way a phone shooter is held: the things done once in a while
/// in a row along the top, and under the right thumb one big fire button,
/// ringed by aim, jump, crouch and reload.
const BUTTONS: [ButtonDef; 15] = [
    ButtonDef {
        accent: true,
        ..play(
            true,
            true,
            (34.0, 25.0),
            11.0,
            Control::Mouse(MouseButton::Left),
            "FIRE",
        )
    },
    play(
        true,
        true,
        (34.0, 50.0),
        5.8,
        Control::Mouse(MouseButton::Right),
        "AIM",
    ),
    play(
        true,
        true,
        (12.0, 38.0),
        6.8,
        Control::Key(KeyCode::Space),
        "JUMP",
    ),
    play(
        true,
        true,
        (12.0, 12.0),
        6.2,
        Control::Key(KeyCode::KeyC),
        "CROUCH",
    ),
    play(
        true,
        true,
        (61.0, 18.0),
        6.2,
        Control::Key(KeyCode::KeyR),
        "RELOAD",
    ),
    play(
        true,
        true,
        (79.0, 32.0),
        5.2,
        Control::Key(KeyCode::KeyE),
        "USE",
    ),
    play(
        true,
        true,
        (78.0, 8.0),
        4.8,
        Control::Key(KeyCode::ControlLeft),
        "PRONE",
    ),
    ButtonDef {
        shown: Shown::Always,
        ..top(0.0, Control::Mods, "MENU")
    },
    top(1.0, Control::Mouse(MouseButton::Middle), "SWAP"),
    top(2.0, Control::Key(KeyCode::KeyV), "MELEE"),
    top(3.0, Control::Key(KeyCode::KeyF), "NADE"),
    top(4.0, Control::Key(KeyCode::KeyG), "TAC"),
    top(5.0, Control::Key(KeyCode::Digit4), "STREAK"),
    top(6.0, Control::Key(KeyCode::Tab), "SCORE"),
    top(7.0, Control::Key(KeyCode::Escape), "PAUSE"),
];

/// Extra hit area beyond a control's drawn radius, so a near miss still lands.
const HIT_SLOP: f32 = 1.25;
/// Where the move stick rests, from the left and bottom edges.
const STICK_REST: (f32, f32) = (30.0, 27.0);
const STICK_BASE_RADIUS: f32 = 11.0;
const STICK_KNOB_RADIUS: f32 = 5.0;
/// Pushing the stick this far past its ring sprints.
const SPRINT_REACH: f32 = 1.35;
/// View turn, in mouse counts, for a drag as long as the window is tall.
const LOOK_PER_HEIGHT: f32 = 430.0;

const FILL: Color = Color::srgba(0.04, 0.05, 0.07, 0.34);
const FILL_HELD: Color = Color::srgba(0.20, 0.82, 0.76, 0.42);
const RING: Color = Color::srgba(1.0, 1.0, 1.0, 0.42);
const RING_ACCENT: Color = Color::srgba(0.20, 0.82, 0.76, 0.85);
const LABEL: Color = Color::srgba(1.0, 1.0, 1.0, 0.82);

#[derive(Resource, Default)]
struct VirtualPad {
    /// The window size the layout was computed for; relayout happens when it
    /// changes (the surface is resized once after the activity starts).
    size: Vec2,
    /// One hundredth of the window height, in logical pixels.
    unit: f32,
    button_nodes: Vec<Entity>,
    button_labels: Vec<Entity>,
    /// Hit-test geometry for the buttons, in logical pixels.
    geometry: Vec<(Vec2, f32)>,
    knob: Option<Entity>,
    base: Option<Entity>,
    stick: Option<u64>,
    stick_origin: Vec2,
    stick_current: Vec2,
    look: Option<u64>,
    look_last: Vec2,
    /// The finger acting as the mouse while a game menu is up.
    pointer: Option<u64>,
    button_touches: HashMap<u64, usize>,
    held_keys: HashSet<KeyCode>,
    held_mouse: HashSet<MouseButton>,
    /// Frames `Escape` stays down for a Back press.
    back_frames: u8,
    /// Controls touched this frame. A quick tap can land and lift between two
    /// frames; it is still a press, held for the one frame it was seen in.
    tapped: Vec<usize>,
    pointer_tapped: bool,
    /// Whether the last frame was play, to drop every hold when that changes.
    was_play: bool,
    /// Every touch currently down, for the three-finger gesture.
    active: HashSet<u64>,
    gesture_latched: bool,
}

pub struct TouchPlugin;

impl Plugin for TouchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VirtualPad>()
            .init_resource::<menu::ModMenu>()
            .add_systems(Startup, (spawn_pad, menu::spawn, killcam_bars::spawn))
            .add_systems(
                Update,
                (
                    layout_pad,
                    style_pad,
                    menu::update,
                    menu::apply_preset,
                    menu::pad_menu,
                    menu::drive_lan,
                    menu::fps_readout,
                    frame_log,
                    killcam_bars::update,
                    skins::paint,
                    late_commands,
                ),
            )
            // Last of the frame, so the cap holds the whole of it.
            .add_systems(Last, frame_cap::limit)
            .add_systems(
                PreUpdate,
                (
                    (bridge::render_size, bridge::pump).before(InputSystems),
                    (menu::touch, drive_pad).chain().after(InputSystems),
                ),
            );
        // winit delivers iOS touches itself, in the real screen's pixels; they
        // are put back into the render size before the pad reads them.
        #[cfg(target_os = "ios")]
        app.add_systems(
            PreUpdate,
            (
                ios::report_view,
                ios::remap_touches.before(bridge::render_size),
            )
                .chain()
                .before(InputSystems),
        );
    }
}

/// A ringed disc of UI, non-pickable so it never blocks the game's own menus.
fn disc(commands: &mut Commands, fill: Color, ring: Color, label: Option<&str>) -> (Entity, Entity) {
    let text = commands
        .spawn((
            Text::new(label.unwrap_or_default()),
            TextFont::default(),
            TextColor(LABEL),
            Pickable::IGNORE,
        ))
        .id();
    let entity = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(ring),
            GlobalZIndex(1000),
            Pickable::IGNORE,
        ))
        .add_child(text)
        .id();
    (entity, text)
}

fn spawn_pad(
    mut commands: Commands,
    mut pad: ResMut<VirtualPad>,
    mut queue: ResMut<console::ConsoleQueue>,
) {
    queue.push_line(BIND_AIM_HOLD);
    let base = disc(
        &mut commands,
        Color::srgba(0.04, 0.05, 0.07, 0.18),
        Color::srgba(1.0, 1.0, 1.0, 0.22),
        None,
    );
    let knob = disc(&mut commands, Color::srgba(1.0, 1.0, 1.0, 0.30), RING, None);
    pad.base = Some(base.0);
    pad.knob = Some(knob.0);
    for def in &BUTTONS {
        let ring = if def.accent { RING_ACCENT } else { RING };
        let (node, label) = disc(&mut commands, FILL, ring, Some(def.label));
        pad.button_nodes.push(node);
        pad.button_labels.push(label);
    }
}

/// Frame times, written to the log every five seconds: the average rate, the
/// worst frame, and how many frames took over 50 ms (a dip a player feels as a
/// stall). A readout that only shows an average cannot tell a steady 30 from a
/// 60 with a hitch every second, and they are different problems.
fn frame_log(time: Res<Time<Real>>, mut window: Local<(u32, u32, f32, f32)>) {
    let delta = time.delta_secs();
    window.0 += 1;
    window.2 += delta;
    window.3 = window.3.max(delta);
    if delta > 0.05 {
        window.1 += 1;
    }
    if window.2 >= 5.0 {
        diag::info!(
            Launch,
            "frames: {:.1} fps average, worst {:.0} ms, {} over 50 ms",
            window.0 as f32 / window.2,
            window.3 * 1000.0,
            window.1
        );
        *window = (0, 0, 0.0, 0.0);
    }
}

/// Place a disc so its centre is `center`.
fn place(nodes: &mut Query<&mut Node>, entity: Option<Entity>, center: Vec2, radius: f32) {
    if let Some(entity) = entity
        && let Ok(mut node) = nodes.get_mut(entity)
    {
        node.left = Val::Px(center.x - radius);
        node.top = Val::Px(center.y - radius);
        node.width = Val::Px(radius * 2.0);
        node.height = Val::Px(radius * 2.0);
    }
}

fn stick_rest(pad: &VirtualPad) -> Vec2 {
    Vec2::new(
        STICK_REST.0 * pad.unit,
        pad.size.y - STICK_REST.1 * pad.unit,
    )
}

/// Place every control. Runs when the window size changes (the surface is
/// portrait until the activity rotates, and is then resized to the render
/// resolution, so a one-shot layout at `Startup` would be wrong twice).
fn layout_pad(
    mut pad: ResMut<VirtualPad>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<&mut Node>,
    mut fonts: Query<&mut TextFont>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    if size.x <= 1.0 || size.y <= 1.0 || size == pad.size {
        return;
    }
    pad.size = size;
    pad.unit = size.y / 100.0;
    let unit = pad.unit;

    pad.geometry.clear();
    for (index, def) in BUTTONS.iter().enumerate() {
        let x = def.at.0 * unit;
        let y = def.at.1 * unit;
        let center = Vec2::new(
            if def.right { size.x - x } else { x },
            if def.bottom { size.y - y } else { y },
        );
        let radius = def.radius * unit;
        pad.geometry.push((center, radius));
        place(&mut nodes, Some(pad.button_nodes[index]), center, radius);
        if let Ok(mut font) = fonts.get_mut(pad.button_labels[index]) {
            // A label fits inside its disc whatever its length.
            let width = def.label.len().max(3) as f32;
            font.font_size = bevy::text::FontSize::Px((radius * 2.6 / width).min(radius * 0.62));
        }
    }
    let rest = stick_rest(&pad);
    place(&mut nodes, pad.base, rest, STICK_BASE_RADIUS * unit);
    place(&mut nodes, pad.knob, rest, STICK_KNOB_RADIUS * unit);
    diag::info!(
        Launch,
        "touch: layout {}x{} scale={}",
        size.x,
        size.y,
        window.scale_factor()
    );
}

/// The menu can always be opened. In a match whose host turned cheats off (or
/// that someone else hosts) it holds only the skin picker; see [`skins_only`].
pub(crate) fn menu_allowed(
    _screen: Option<&frame::AppScreen>,
    _authority: Option<&net::AuthorityWorld>,
) -> bool {
    true
}

/// Whether the menu is cut down to the skin picker: in a match where the mod
/// menu's cheats are not on.
pub(crate) fn skins_only(
    screen: Option<&frame::AppScreen>,
    authority: Option<&net::AuthorityWorld>,
) -> bool {
    let in_match = screen.is_some_and(|screen| {
        matches!(
            screen,
            frame::AppScreen::ClassSelect | frame::AppScreen::InGame
        )
    });
    in_match && !authority.is_some_and(|authority| authority.0.cheats_enabled())
}

/// Whether the player is steering a soldier, as opposed to using a menu.
fn in_play(pointer: Option<&frame::GameplayPointer>) -> bool {
    pointer.is_some_and(|pointer| pointer.0)
}

/// Show what applies to this screen, and light what is held.
fn style_pad(
    pad: Res<VirtualPad>,
    menu: Res<menu::ModMenu>,
    pointer: Option<Res<frame::GameplayPointer>>,
    devices: Option<Res<frame::InputDevices>>,
    screen: Option<Res<frame::AppScreen>>,
    authority: Option<Res<net::AuthorityWorld>>,
    mut nodes: Query<(&mut Node, &mut BackgroundColor)>,
) {
    let allowed = menu_allowed(screen.as_deref(), authority.as_deref());
    let play = in_play(pointer.as_deref());
    // A controller in use replaces the pad; the way into the mod menu stays.
    let pad_in_use = devices.is_some_and(|devices| devices.pad_prompts);
    let loading = screen.is_some_and(|screen| matches!(*screen, frame::AppScreen::Loading));
    let held: HashSet<usize> = pad.button_touches.values().copied().collect();

    let mut show = |entity: Option<Entity>, visible: bool, fill: Option<Color>| {
        let Some((mut node, mut background)) = entity.and_then(|entity| nodes.get_mut(entity).ok())
        else {
            return;
        };
        let display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        if let Some(fill) = fill
            && background.0 != fill
        {
            background.0 = fill;
        }
    };
    for (index, def) in BUTTONS.iter().enumerate() {
        let visible = !menu.open
            && !loading
            && match def.shown {
                Shown::InPlay => play && !pad_in_use,
                Shown::Always => allowed,
            };
        let fill = if held.contains(&index) {
            FILL_HELD
        } else {
            FILL
        };
        show(Some(pad.button_nodes[index]), visible, Some(fill));
    }
    let stick = play && !pad_in_use && !menu.open;
    show(pad.base, stick, None);
    show(pad.knob, stick, None);
}

#[allow(clippy::too_many_arguments)]
fn drive_pad(
    mut pad: ResMut<VirtualPad>,
    mut menu: ResMut<menu::ModMenu>,
    pointer: Option<Res<frame::GameplayPointer>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut motion: MessageWriter<MouseMotion>,
    mut touches: MessageReader<TouchInput>,
    mut nodes: Query<&mut Node>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    screen: Option<Res<frame::AppScreen>>,
    authority: Option<Res<net::AuthorityWorld>>,
) {
    let allowed = menu_allowed(screen.as_deref(), authority.as_deref());
    let play = in_play(pointer.as_deref());
    if play != pad.was_play || menu.open {
        // The meaning of a touch changed under the fingers: nothing stays held.
        pad.was_play = play;
        pad.stick = None;
        pad.look = None;
        pad.pointer = None;
        pad.button_touches.clear();
    }
    let unit = pad.unit.max(1.0);
    let mut look_delta = Vec2::ZERO;

    for touch in touches.read() {
        match touch.phase {
            TouchPhase::Started => {
                pad.active.insert(touch.id);
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                pad.active.remove(&touch.id);
            }
            TouchPhase::Moved => {}
        }
        if pad.active.len() < 3 {
            pad.gesture_latched = false;
        }
        if menu.open {
            continue;
        }
        let ended = matches!(touch.phase, TouchPhase::Ended | TouchPhase::Canceled);

        if pad.stick == Some(touch.id) {
            pad.stick_current = touch.position;
            if ended {
                pad.stick = None;
            }
            continue;
        }
        if pad.look == Some(touch.id) {
            look_delta += touch.position - pad.look_last;
            pad.look_last = touch.position;
            if ended {
                pad.look = None;
            }
            continue;
        }
        if pad.pointer == Some(touch.id) {
            if let Ok(mut window) = windows.single_mut() {
                window.set_cursor_position(Some(touch.position));
            }
            if ended {
                pad.pointer = None;
            }
            continue;
        }
        if pad.button_touches.contains_key(&touch.id) {
            if ended {
                pad.button_touches.remove(&touch.id);
            }
            continue;
        }
        if touch.phase != TouchPhase::Started {
            continue;
        }

        // Three fingers down also opens the mod menu, for when a controller
        // has taken the pad off the screen.
        if pad.active.len() >= 3 {
            if !pad.gesture_latched {
                pad.gesture_latched = true;
                menu.open = allowed;
            }
            continue;
        }
        let hit = pad.geometry.iter().position(|(center, radius)| {
            center.distance(touch.position) <= *radius * HIT_SLOP
        });
        if let Some(index) = hit
            && (play || BUTTONS[index].shown == Shown::Always)
        {
            if BUTTONS[index].control == Control::Mods {
                menu.open = allowed;
            } else {
                pad.button_touches.insert(touch.id, index);
                pad.tapped.push(index);
            }
            continue;
        }
        if !play {
            // One of the game's menus is up: the finger is the mouse.
            if pad.pointer.is_none() {
                pad.pointer = Some(touch.id);
                pad.pointer_tapped = true;
                if let Ok(mut window) = windows.single_mut() {
                    window.set_cursor_position(Some(touch.position));
                }
            }
            continue;
        }
        if touch.position.x < pad.size.x * 0.45 && pad.stick.is_none() {
            // Floating stick: anchored where the thumb lands, not at a fixed
            // spot, which is what makes movement controllable.
            pad.stick = Some(touch.id);
            pad.stick_origin = touch.position;
            pad.stick_current = touch.position;
        } else if pad.look.is_none() {
            pad.look = Some(touch.id);
            pad.look_last = touch.position;
        }
    }

    // Desired keys and buttons this frame.
    let mut want_keys: HashSet<KeyCode> = HashSet::new();
    let mut want_mouse: HashSet<MouseButton> = HashSet::new();

    let base_radius = STICK_BASE_RADIUS * unit;
    if pad.stick.is_some() {
        let offset = pad.stick_current - pad.stick_origin;
        let dead = base_radius * 0.22;
        if offset.y < -dead {
            want_keys.insert(KeyCode::KeyW);
        }
        if offset.y > dead {
            want_keys.insert(KeyCode::KeyS);
        }
        if offset.x < -dead {
            want_keys.insert(KeyCode::KeyA);
        }
        if offset.x > dead {
            want_keys.insert(KeyCode::KeyD);
        }
        if offset.y < -base_radius * SPRINT_REACH {
            want_keys.insert(KeyCode::ShiftLeft);
        }
        let knob = pad.stick_origin + offset.clamp_length_max(base_radius);
        place(&mut nodes, pad.base, pad.stick_origin, base_radius);
        place(&mut nodes, pad.knob, knob, STICK_KNOB_RADIUS * unit);
    } else if pad.size.y > 1.0 {
        let rest = stick_rest(&pad);
        place(&mut nodes, pad.base, rest, base_radius);
        place(&mut nodes, pad.knob, rest, STICK_KNOB_RADIUS * unit);
    }
    let tapped = std::mem::take(&mut pad.tapped);
    for &index in pad.button_touches.values().chain(&tapped) {
        match BUTTONS[index].control {
            Control::Key(code) => {
                want_keys.insert(code);
            }
            Control::Mouse(button) => {
                want_mouse.insert(button);
            }
            Control::Mods => {}
        }
    }
    if pad.pointer.is_some() || std::mem::take(&mut pad.pointer_tapped) {
        want_mouse.insert(MouseButton::Left);
    }

    // The system Back key is `Escape`: it backs out of a menu and pauses a
    // match. A press and its release can land in one frame, so it is held for
    // a frame of its own.
    if bridge::take_back_presses() > 0 {
        if menu.open {
            menu.open = false;
        } else {
            pad.back_frames = 2;
        }
    }
    if pad.back_frames > 0 {
        pad.back_frames -= 1;
        if pad.back_frames > 0 {
            want_keys.insert(KeyCode::Escape);
        }
    }

    // Diff against the previous frame so only edges are pressed.
    for &code in want_keys.difference(&pad.held_keys) {
        keys.press(code);
    }
    for &code in pad.held_keys.difference(&want_keys) {
        keys.release(code);
    }
    for &button in want_mouse.difference(&pad.held_mouse) {
        mouse.press(button);
    }
    for &button in pad.held_mouse.difference(&want_mouse) {
        mouse.release(button);
    }
    pad.held_keys = want_keys;
    pad.held_mouse = want_mouse;

    if look_delta != Vec2::ZERO && pad.size.y > 1.0 {
        motion.write(MouseMotion {
            delta: look_delta / pad.size.y * LOOK_PER_HEIGHT,
        });
    }
}

/// Console lines from `IW4L_LATE_CMDS` (separated by `;`), run once the player
/// has been in a match for forty-five seconds. For testing on a phone, where the
/// startup commands run before there is anyone alive to apply them to.
fn late_commands(
    time: Res<Time<Real>>,
    scope: Option<Res<State<frame::MatchScope>>>,
    mut queue: ResMut<console::ConsoleQueue>,
    mut since: Local<Option<f32>>,
    mut sent: Local<u32>,
) {
    if *sent >= 4 {
        return;
    }
    let live = scope.is_some_and(|scope| *scope.get() == frame::MatchScope::Live);
    if !live {
        *since = None;
        return;
    }
    let started = *since.get_or_insert(time.elapsed_secs());
    if time.elapsed_secs() - started < 30.0 + 15.0 * *sent as f32 {
        return;
    }
    *sent += 1;
    if let Ok(lines) = std::env::var("IW4L_LATE_CMDS") {
        for line in lines.split(';').map(str::trim).filter(|line| !line.is_empty()) {
            queue.push_line(line);
        }
    }
}
