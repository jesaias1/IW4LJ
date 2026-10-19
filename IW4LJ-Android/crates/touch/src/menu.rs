//! The mod menu.
//!
//! A phone has no console to type into, so everything a player would type is a
//! button here: each one pushes the console line it shows the result of. The
//! menu is laid out by Bevy's UI (it reflows to any render resolution) and
//! reads touches itself â€” while it is open the pad and the game's own menus
//! get none.

use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::text::{FontSize, Justify, TextLayout};
use bevy::ui::{ComputedNode, Display, GlobalZIndex, UiGlobalTransform};
use bevy::window::PrimaryWindow;
use crate::skins::SKINS;
use console::{ConsoleLine, ConsoleQueue};

/// A device setting kept in a file beside the game and read at launch.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Setting {
    /// Lines the game is rendered at; the display scales it to the screen.
    RenderHeight,
    /// How many of a texture's largest mip levels are left out.
    TextureSkip,
    /// Which effects are drawn: see [`PRESETS`].
    Preset,
    /// The frame-rate readout.
    ShowFps,
    /// Whether the 60 fps cap is lifted.
    UncapFps,
    /// The skin on the gun, kept between matches and launches.
    Skin,
}

impl Setting {
    fn file(self) -> &'static str {
        match self {
            Self::RenderHeight => "render_height.txt",
            Self::TextureSkip => "texture_skip.txt",
            Self::Preset => "graphics_preset.txt",
            Self::ShowFps => "show_fps.txt",
            Self::UncapFps => "uncap_fps.txt",
            Self::Skin => "skin.txt",
        }
    }
}

/// The effect switches of each graphics preset, as console lines. Measured on
/// a Snapdragon 8 Gen 2 at 720 lines: shadow maps, depth of field, glow and
/// distortion together are the difference between about 45 and 60 frames a
/// second, and most of the heat.
///
/// The sun's shadow has a switch of its own: `sm_enable` alone left the sun
/// shadow pass running, and in a profile that pass and the shadow-caster bake
/// feeding it were about a twelfth of the CPU time of a frame.
const PRESETS: [&str; 3] = [
    // Quality.
    "sm_enable 1; sm_sunEnable 1; r_dof_enable 1; r_glow 1; r_distortion 1",
    // Balanced.
    "sm_enable 0; sm_sunEnable 0; r_dof_enable 0; r_glow 1; r_distortion 1",
    // Performance.
    "sm_enable 0; sm_sunEnable 0; r_dof_enable 0; r_glow 0; r_distortion 0",
];
const PRESET_DEFAULT: i32 = 2;

#[derive(Clone, Copy)]
enum Act {
    /// Run a console line.
    Line(&'static str),
    /// A switch the menu remembers: `on` and `off` are the lines for each way.
    Toggle {
        slot: usize,
        on: &'static str,
        off: &'static str,
    },
    /// Write a setting; `-1` removes the file, which means "let the device
    /// decide".
    Set(Setting, i32),
    /// Ask the activity for text (a phone keyboard cannot type into the game).
    Prompt(u32),
    /// Start, join or stop a LAN game (see the `lan` crate).
    Lan(Lan),
    /// Paint the current gun with a skin (`-1` puts the normal one back).
    Skin(i32),
}

#[derive(Clone, Copy, PartialEq)]
enum Lan {
    Host,
    /// Host for players outside this Wi-Fi as well.
    HostOnline,
    Join,
    Stop,
}

/// The lobby steps that follow a LAN action, once its network side is up.
/// Hosting opens a lobby and makes it public on the in-process relay; joining
/// opens the lobby list, which now shows the host's.
const LAN_HOST_LINES: &str = "ui_create_lobby; ui_lobby_privacy";
const LAN_JOIN_LINES: &str = "openmenu find_lobbies; ui_browser_refresh";
/// Seconds the relay is given to start, and the browser to reach a found host
/// (it retries about every five seconds).
const LAN_HOST_DELAY: f32 = 1.0;
const LAN_JOIN_DELAY: f32 = 7.0;

struct Item {
    label: &'static str,
    act: Act,
}

const fn line(label: &'static str, line: &'static str) -> Item {
    Item {
        label,
        act: Act::Line(line),
    }
}

const fn toggle(label: &'static str, slot: usize, on: &'static str, off: &'static str) -> Item {
    Item {
        label,
        act: Act::Toggle { slot, on, off },
    }
}

const fn set(label: &'static str, setting: Setting, value: i32) -> Item {
    Item {
        label,
        act: Act::Set(setting, value),
    }
}

const TOGGLES: usize = 6;
/// The toggle that turns Select + d-pad left/right into save/load position.
const TP_SLOT: usize = 5;

struct Tab {
    name: &'static str,
    hint: &'static str,
    items: &'static [Item],
}

const TABS: [Tab; 8] = [
    Tab {
        name: "LAN",
        hint: "From the main menu: one phone hosts, the others join - on the same Wi-Fi, or over the internet by IP. The host picks the map and starts.",
        items: &[
            Item {
                label: "HOST LAN GAME",
                act: Act::Lan(Lan::Host),
            },
            Item {
                label: "HOST ONLINE (BY IP)",
                act: Act::Lan(Lan::HostOnline),
            },
            Item {
                label: "JOIN LAN GAME",
                act: Act::Lan(Lan::Join),
            },
            Item {
                label: "JOIN BY IP",
                act: Act::Prompt(crate::bridge::PROMPT_HOST_IP),
            },
            Item {
                label: "STOP LAN",
                act: Act::Lan(Lan::Stop),
            },
        ],
    },
    Tab {
        name: "TRICKSHOT",
        hint: "Noclip to a spot, save it, and every respawn puts you back there. Controller: turn on PAD SAVE/LOAD, then hold SELECT and press D-PAD LEFT to save, D-PAD RIGHT to load.",
        items: &[
            line("SAVE POSITION", "save"),
            line("LOAD POSITION", "load"),
            toggle("SPAWN AT SAVED", 0, "loadspawn on", "loadspawn off"),
            toggle("NOCLIP", 1, "noclip on", "noclip off"),
            toggle("GOD MODE", 2, "god", "god"),
            toggle("PAD SAVE/LOAD", 5, "", ""),
            line("BOT HERE", "stillbot"),
            line("NEXT KILL WINS", "lastkill"),
            line("TRICKSHOT S&D", "trickshot"),
            line("1 HP", "damage 99"),
            line("REFILL AMMO", "give ammo"),
            line("RESPAWN", "kill"),
        ],
    },
    Tab {
        name: "SKINS",
        hint: "Paints your current gun. (V) skins move. Choose NO SKIN to put the normal one back.",
        items: &[
            Item {
                label: "NO SKIN",
                act: Act::Skin(-1),
            },
            Item {
                label: SKINS[0].name,
                act: Act::Skin(0),
            },
            Item {
                label: SKINS[1].name,
                act: Act::Skin(1),
            },
            Item {
                label: SKINS[2].name,
                act: Act::Skin(2),
            },
            Item {
                label: SKINS[3].name,
                act: Act::Skin(3),
            },
            Item {
                label: SKINS[4].name,
                act: Act::Skin(4),
            },
            Item {
                label: SKINS[5].name,
                act: Act::Skin(5),
            },
            Item {
                label: SKINS[6].name,
                act: Act::Skin(6),
            },
            Item {
                label: SKINS[7].name,
                act: Act::Skin(7),
            },
            Item {
                label: SKINS[8].name,
                act: Act::Skin(8),
            },
            Item {
                label: SKINS[9].name,
                act: Act::Skin(9),
            },
            Item {
                label: SKINS[10].name,
                act: Act::Skin(10),
            },
            Item {
                label: SKINS[11].name,
                act: Act::Skin(11),
            },
            Item {
                label: SKINS[12].name,
                act: Act::Skin(12),
            },
            Item {
                label: SKINS[13].name,
                act: Act::Skin(13),
            },
            Item {
                label: SKINS[14].name,
                act: Act::Skin(14),
            },
            Item {
                label: SKINS[15].name,
                act: Act::Skin(15),
            },
            Item {
                label: SKINS[16].name,
                act: Act::Skin(16),
            },
            Item {
                label: SKINS[17].name,
                act: Act::Skin(17),
            },
            Item {
                label: SKINS[18].name,
                act: Act::Skin(18),
            },
            Item {
                label: SKINS[19].name,
                act: Act::Skin(19),
            },
            Item {
                label: SKINS[20].name,
                act: Act::Skin(20),
            },
            Item {
                label: SKINS[21].name,
                act: Act::Skin(21),
            },
            Item {
                label: SKINS[22].name,
                act: Act::Skin(22),
            },
            Item {
                label: SKINS[23].name,
                act: Act::Skin(23),
            },
            Item {
                label: SKINS[24].name,
                act: Act::Skin(24),
            },
            Item {
                label: SKINS[25].name,
                act: Act::Skin(25),
            },
            Item {
                label: SKINS[26].name,
                act: Act::Skin(26),
            },
        ],
    },
    Tab {
        name: "MATCH",
        hint: "Start a private match from the main menu, then control it from here.",
        items: &[
            line("START MATCH", "force_match_start"),
            line("SPAWN", "spawn 0"),
            line("RESTART MAP", "map_restart"),
            line("LEAVE MATCH", "disconnect"),
            toggle("THIRD PERSON", 3, "thirdperson", "thirdperson"),
        ],
    },
    Tab {
        name: "BOTS",
        hint: "Bots join the match you are hosting. A still bot never moves or shoots.",
        items: &[
            line("ADD 1 BOT", "bot add 1"),
            line("ADD 5 BOTS", "bot add 5"),
            line("ADD STILL BOT", "bot dummy 1"),
            toggle("FREEZE BOTS", 4, "bot hold on", "bot hold off"),
            line("BOTS FIRE", "bot fire all"),
        ],
    },
    Tab {
        name: "MAPS",
        hint: "Loads the map straight away as a private match.",
        items: &[
            line("AFGHAN", "map mp_afghan"),
            line("DERAIL", "map mp_derail"),
            line("ESTATE", "map mp_estate"),
            line("FAVELA", "map mp_favela"),
            line("HIGHRISE", "map mp_highrise"),
            line("INVASION", "map mp_invasion"),
            line("KARACHI", "map mp_checkpoint"),
            line("QUARRY", "map mp_quarry"),
            line("RUNDOWN", "map mp_rundown"),
            line("RUST", "map mp_rust"),
            line("SCRAPYARD", "map mp_boneyard"),
            line("SKIDROW", "map mp_nightshift"),
            line("SUB BASE", "map mp_subbase"),
            line("TERMINAL", "map mp_terminal"),
            line("UNDERPASS", "map mp_underpass"),
            line("WASTELAND", "map mp_brecourt"),
        ],
    },
    Tab {
        name: "ACCOUNT",
        hint: "Unlocks are saved from the main menu, not during a match. A new name shows from the next spawn.",
        items: &[
            Item {
                label: "CHANGE NAME",
                act: Act::Prompt(crate::bridge::PROMPT_NAME),
            },
            line("UNLOCK ALL", "unlock all"),
            line("RESET LEVEL", "unlock reset"),
            line("NO PRESTIGE", "prestige 0"),
            line("PRESTIGE 1", "prestige 1"),
            line("PRESTIGE 2", "prestige 2"),
            line("PRESTIGE 3", "prestige 3"),
            line("PRESTIGE 4", "prestige 4"),
            line("PRESTIGE 5", "prestige 5"),
            line("PRESTIGE 6", "prestige 6"),
            line("PRESTIGE 7", "prestige 7"),
            line("PRESTIGE 8", "prestige 8"),
            line("PRESTIGE 9", "prestige 9"),
            line("PRESTIGE 10", "prestige 10"),
        ],
    },
    Tab {
        name: "GRAPHICS",
        hint: "Performance holds the frame rate and keeps the phone cool. The game is held to 60 fps so it stays cool in a long session; UNCAP FPS lifts that for phones that can take it. Render and texture settings apply the next time the game starts.",
        items: &[
            set("PERFORMANCE", Setting::Preset, 2),
            set("BALANCED", Setting::Preset, 1),
            set("QUALITY", Setting::Preset, 0),
            set("SHOW FPS", Setting::ShowFps, 1),
            set("HIDE FPS", Setting::ShowFps, 0),
            set("CAP FPS 60", Setting::UncapFps, 0),
            set("UNCAP FPS", Setting::UncapFps, 1),
            set("RENDER AUTO", Setting::RenderHeight, -1),
            set("RENDER 540P", Setting::RenderHeight, 540),
            set("RENDER 720P", Setting::RenderHeight, 720),
            set("RENDER 900P", Setting::RenderHeight, 900),
            set("RENDER 1080P", Setting::RenderHeight, 1080),
            set("TEXTURES AUTO", Setting::TextureSkip, -1),
            set("TEXTURES FULL", Setting::TextureSkip, 0),
            set("TEXTURES HALF", Setting::TextureSkip, 1),
            set("TEXTURES LOW", Setting::TextureSkip, 2),
        ],
    },
];

const DIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.62);
const PANEL: Color = Color::srgba(0.085, 0.095, 0.105, 0.97);
const ACCENT: Color = Color::srgb(0.20, 0.82, 0.76);
const EDGE: Color = Color::srgba(1.0, 1.0, 1.0, 0.14);
const BUTTON: Color = Color::srgba(1.0, 1.0, 1.0, 0.07);
const BUTTON_ON: Color = Color::srgba(0.20, 0.82, 0.76, 0.30);
const BUTTON_FOCUS: Color = Color::srgba(0.20, 0.82, 0.76, 0.50);
const BUTTON_TAP: Color = Color::srgba(0.20, 0.82, 0.76, 0.65);
const TEXT: Color = Color::srgba(1.0, 1.0, 1.0, 0.94);
const MUTED: Color = Color::srgba(1.0, 1.0, 1.0, 0.55);

#[derive(Clone, Copy)]
enum Target {
    Tab(usize),
    Item(usize, usize),
    Close,
}

#[derive(Resource, Default)]
pub(crate) struct ModMenu {
    pub(crate) open: bool,
    tab: usize,
    /// Only the skin picker is on offer (cheats are not on in this match).
    skins_only: bool,
    toggles: [bool; TOGGLES],
    render_height: i32,
    texture_skip: i32,
    preset: i32,
    /// Set when the preset has to be sent to the game (again).
    preset_due: bool,
    show_fps: bool,
    uncap_fps: bool,
    fps: Option<Entity>,
    /// The control a controller has selected, as an index into the open tab.
    focus: usize,
    /// Whether the last input was a controller, which is when the focus shows.
    pad_used: bool,
    /// Lobby lines owed after a LAN action, and when to send them.
    lan_next: Option<(f32, &'static str)>,
    /// The host a LAN join last reached, so the lobby list opens once per host
    /// whether it was found by search or typed in.
    lan_joined: Option<std::net::IpAddr>,
    root: Option<Entity>,
    pages: Vec<Entity>,
    hint: Option<Entity>,
    status: Option<Entity>,
    targets: Vec<(Entity, Target)>,
    /// The button last tapped and when its highlight ends.
    tapped: Option<(Entity, f32)>,
}

fn setting_path(setting: Setting) -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(setting.file()))
}

pub(crate) fn read_setting(setting: Setting) -> i32 {
    setting_path(setting)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(-1)
}

pub(crate) fn write_setting(setting: Setting, value: i32) {
    let Some(path) = setting_path(setting) else {
        return;
    };
    let result = if value < 0 {
        match std::fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    } else {
        std::fs::write(&path, value.to_string())
    };
    if let Err(error) = result {
        diag::warn!(Launch, "mod menu: {}: {error}", path.display());
    }
}

fn label(commands: &mut Commands, text: &str, size: f32, color: Color) -> Entity {
    commands
        .spawn((
            Text::new(text),
            TextFont {
                font_size: FontSize::Vh(size),
                ..default()
            },
            TextColor(color),
            TextLayout {
                justify: Justify::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// The small print under a button's name.
fn small_print(act: Act) -> &'static str {
    match act {
        Act::Lan(Lan::HostOnline) => "may need port forwarding (UDP 28960)",
        Act::Prompt(kind) if kind == crate::bridge::PROMPT_HOST_IP => "friend's IP, or IP:port",
        _ => "",
    }
}

fn button(commands: &mut Commands, node: Node, text: &str, size: f32) -> Entity {
    button_with(commands, node, text, size, "")
}

fn button_with(commands: &mut Commands, node: Node, text: &str, size: f32, small: &str) -> Entity {
    let text = label(commands, text, size, TEXT);
    if !small.is_empty() {
        let small = label(commands, small, size * 0.62, MUTED);
        return commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Vh(0.6),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Vh(1.4)),
                    ..node
                },
                BackgroundColor(BUTTON),
                BorderColor::all(EDGE),
                Pickable::IGNORE,
            ))
            .add_children(&[text, small])
            .id();
    }
    commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Vh(1.4)),
                ..node
            },
            BackgroundColor(BUTTON),
            BorderColor::all(EDGE),
            Pickable::IGNORE,
        ))
        .add_child(text)
        .id()
}

pub(crate) fn spawn(mut commands: Commands, mut menu: ResMut<ModMenu>) {
    menu.render_height = read_setting(Setting::RenderHeight);
    menu.texture_skip = read_setting(Setting::TextureSkip);
    menu.preset = match read_setting(Setting::Preset) {
        preset @ 0..=2 => preset,
        _ => PRESET_DEFAULT,
    };
    menu.show_fps = read_setting(Setting::ShowFps) == 1;
    menu.uncap_fps = read_setting(Setting::UncapFps) == 1;
    crate::frame_cap::set_uncapped(menu.uncap_fps);

    // The frame-rate readout: small, under the button row, out of the way.
    let fps = commands
        .spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Vh(2.6),
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                top: Val::Vh(13.5),
                right: Val::Vh(3.0),
                ..default()
            },
            GlobalZIndex(1000),
            Pickable::IGNORE,
        ))
        .id();
    menu.fps = Some(fps);

    // Header: title, tabs, close.
    let title = label(&mut commands, "IW4LJ  //  MENU", 4.2, ACCENT);
    let header = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: Val::Vh(1.6),
                padding: UiRect::axes(Val::Vh(3.0), Val::Vh(2.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(EDGE),
            Pickable::IGNORE,
        ))
        .add_child(title)
        .id();
    let spacer = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(header).add_child(spacer);
    for (index, tab) in TABS.iter().enumerate() {
        let entity = button(
            &mut commands,
            Node {
                height: Val::Vh(7.6),
                padding: UiRect::horizontal(Val::Vh(2.4)),
                ..default()
            },
            tab.name,
            2.7,
        );
        menu.targets.push((entity, Target::Tab(index)));
        commands.entity(header).add_child(entity);
    }
    let close = button(
        &mut commands,
        Node {
            width: Val::Vh(9.0),
            height: Val::Vh(7.6),
            margin: UiRect::left(Val::Vh(1.6)),
            ..default()
        },
        "X",
        3.6,
    );
    menu.targets.push((close, Target::Close));
    commands.entity(header).add_child(close);

    // One page of buttons per tab; only the selected one is displayed.
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (tab_index, tab) in TABS.iter().enumerate() {
        let page = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    display: Display::None,
                    flex_wrap: FlexWrap::Wrap,
                    align_content: AlignContent::FlexStart,
                    column_gap: Val::Vh(1.8),
                    row_gap: Val::Vh(1.8),
                    padding: UiRect::all(Val::Vh(3.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        for (item_index, item) in tab.items.iter().enumerate() {
            let entity = button_with(
                &mut commands,
                if columns(tab_index) > MENU_COLUMNS {
                    Node {
                        width: Val::Percent(15.2),
                        height: Val::Vh(7.6),
                        ..default()
                    }
                } else {
                    Node {
                        width: Val::Percent(23.4),
                        height: Val::Vh(11.0),
                        ..default()
                    }
                },
                item.label,
                if columns(tab_index) > MENU_COLUMNS { 2.3 } else { 2.9 },
                small_print(item.act),
            );
            menu.targets.push((entity, Target::Item(tab_index, item_index)));
            commands.entity(page).add_child(entity);
        }
        menu.pages.push(page);
        commands.entity(body).add_child(page);
    }

    // Footer: what the tab is for, and what the game last said.
    let hint = label(&mut commands, "", 2.5, MUTED);
    let status = label(&mut commands, "", 2.5, ACCENT);
    let footer = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Vh(0.8),
                padding: UiRect::axes(Val::Vh(3.0), Val::Vh(1.8)),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(EDGE),
            Pickable::IGNORE,
        ))
        .add_children(&[hint, status])
        .id();
    menu.hint = Some(hint);
    menu.status = Some(status);

    let panel = commands
        .spawn((
            Node {
                width: Val::Percent(92.0),
                height: Val::Percent(92.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Vh(2.4)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(ACCENT.with_alpha(0.55)),
            Pickable::IGNORE,
        ))
        .add_children(&[header, body, footer])
        .id();
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(DIM),
            GlobalZIndex(1100),
            Pickable::IGNORE,
        ))
        .add_child(panel)
        .id();
    menu.root = Some(root);
}

/// Taps while the menu is open.
pub(crate) fn touch(
    mut menu: ResMut<ModMenu>,
    mut touches: MessageReader<TouchInput>,
    windows: Query<&Window, With<PrimaryWindow>>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut queue: ResMut<ConsoleQueue>,
    time: Res<Time>,
) {
    if !menu.open {
        touches.clear();
        return;
    }
    let scale = windows.single().map_or(1.0, Window::scale_factor);
    for touch in touches.read() {
        if touch.phase != TouchPhase::Started {
            continue;
        }
        let point = touch.position * scale;
        let page = menu.pages.get(menu.tab).copied();
        let hit = menu.targets.iter().copied().find(|(entity, target)| {
            // A page that is not displayed keeps its last layout.
            let displayed = match target {
                Target::Item(tab, _) => Some(menu.pages[*tab]) == page,
                Target::Tab(_) | Target::Close => true,
            };
            displayed
                && nodes
                    .get(*entity)
                    .is_ok_and(|(node, transform)| node.contains_point(*transform, point))
        });
        let Some((entity, target)) = hit else {
            continue;
        };
        menu.pad_used = false;
        menu.tapped = Some((entity, time.elapsed_secs() + 0.18));
        run(&mut menu, target, &mut queue, time.elapsed_secs());
    }
}

/// What a tap, or the A button on a focused control, does.
fn run(menu: &mut ModMenu, target: Target, queue: &mut ConsoleQueue, now: f32) {
    match target {
        Target::Close => menu.open = false,
        Target::Tab(tab) => {
            menu.tab = tab;
            menu.focus = 0;
        }
        Target::Item(tab, item) => match TABS[tab].items[item].act {
            Act::Line(line) => queue.push_line(line),
            Act::Toggle { slot, on, off } => {
                menu.toggles[slot] = !menu.toggles[slot];
                if slot == TP_SLOT {
                    crate::bridge::set_tp_pad(menu.toggles[slot]);
                }
                queue.push_line(if menu.toggles[slot] { on } else { off });
            }
            Act::Prompt(kind) => {
                crate::bridge::request_prompt(kind);
                menu.open = false;
            }
            Act::Skin(index) => crate::skins::select(index),
            Act::Lan(Lan::Host) => {
                lan::host();
                menu.lan_next = Some((now + LAN_HOST_DELAY, LAN_HOST_LINES));
            }
            Act::Lan(Lan::HostOnline) => {
                lan::host_online();
                menu.lan_next = Some((now + LAN_HOST_DELAY, LAN_HOST_LINES));
            }
            Act::Lan(Lan::Join) => {
                lan::join();
                menu.lan_next = None;
            }
            Act::Lan(Lan::Stop) => {
                lan::stop();
                menu.lan_next = None;
                menu.lan_joined = None;
                queue.push_line("ui_leave_lobby");
            }
            Act::Set(setting, value) => {
                write_setting(setting, value);
                match setting {
                    Setting::RenderHeight => menu.render_height = value,
                    Setting::TextureSkip => menu.texture_skip = value,
                    Setting::Preset => {
                        menu.preset = value;
                        menu.preset_due = true;
                    }
                    Setting::ShowFps => menu.show_fps = value == 1,
                    Setting::UncapFps => {
                        menu.uncap_fps = value == 1;
                        crate::frame_cap::set_uncapped(menu.uncap_fps);
                    }
                    // Skins are set by their own action, not by this one.
                    Setting::Skin => {}
                }
            }
        },
    }
}

/// The menu on a controller. The Xbox or PlayStation button opens it, as the
/// guide button opens a console's; the shoulder buttons change tab, the d-pad
/// moves between controls, A presses one and B closes. While it is open the
/// game's own input is held off, so A does not also jump.
pub(crate) fn pad_menu(
    mut menu: ResMut<ModMenu>,
    pads: Query<&bevy::input::gamepad::Gamepad>,
    mut mod_open: ResMut<frame::ModMenuOpen>,
    mut queue: ResMut<ConsoleQueue>,
    time: Res<Time>,
    screen: Option<Res<frame::AppScreen>>,
    authority: Option<Res<net::AuthorityWorld>>,
) {
    use bevy::input::gamepad::GamepadButton as B;
    let pressed = |button: B| pads.iter().any(|pad| pad.just_pressed(button));
    let allowed = crate::menu_allowed(screen.as_deref(), authority.as_deref());
    if !allowed {
        menu.open = false;
    }
    if allowed && pressed(B::Mode) {
        menu.open = !menu.open;
        menu.pad_used = true;
        menu.focus = 0;
    }
    // Hold the game's input off for exactly as long as this menu is up.
    if menu.open != mod_open.0 {
        mod_open.0 = menu.open;
    }
    if !menu.open {
        return;
    }
    let items = TABS[menu.tab].items.len();
    let mut moved = false;
    let mut step = |menu: &mut ModMenu, delta: i32| {
        let next = menu.focus as i32 + delta;
        if (0..items as i32).contains(&next) {
            menu.focus = next as usize;
        }
        moved = true;
    };
    if pressed(B::DPadRight) && menu.focus % columns(menu.tab) != columns(menu.tab) - 1 {
        step(&mut menu, 1);
    }
    if pressed(B::DPadLeft) && menu.focus % columns(menu.tab) != 0 {
        step(&mut menu, -1);
    }
    if pressed(B::DPadDown) {
        let cols = columns(menu.tab) as i32;
        step(&mut menu, cols);
    }
    if pressed(B::DPadUp) {
        let cols = columns(menu.tab) as i32;
        step(&mut menu, -cols);
    }
    if pressed(B::RightTrigger) && !menu.skins_only {
        menu.tab = (menu.tab + 1) % TABS.len();
        menu.focus = 0;
        moved = true;
    }
    if pressed(B::LeftTrigger) && !menu.skins_only {
        menu.tab = (menu.tab + TABS.len() - 1) % TABS.len();
        menu.focus = 0;
        moved = true;
    }
    if moved {
        menu.pad_used = true;
    }
    if pressed(B::East) {
        menu.open = false;
    }
    if pressed(B::South) {
        let (tab, focus) = (menu.tab, menu.focus.min(items.saturating_sub(1)));
        menu.pad_used = true;
        let target = Target::Item(tab, focus);
        if let Some(&(entity, _)) = menu
            .targets
            .iter()
            .find(|(_, candidate)| matches!(candidate, Target::Item(t, i) if *t == tab && *i == focus))
        {
            menu.tapped = Some((entity, time.elapsed_secs() + 0.18));
        }
        run(&mut menu, target, &mut queue, time.elapsed_secs());
    }
}

/// How many controls a row of a menu page holds.
const MENU_COLUMNS: usize = 4;

/// A long tab (the skins) is laid out in more, smaller buttons.
fn columns(tab: usize) -> usize {
    if TABS[tab].items.len() > 12 { 6 } else { MENU_COLUMNS }
}

/// Send the graphics preset to the game: when it is chosen, and again each
/// time a match comes up, since the switches belong to the match.
pub(crate) fn apply_preset(
    mut menu: ResMut<ModMenu>,
    screen: Option<Res<frame::AppScreen>>,
    mut queue: ResMut<ConsoleQueue>,
    mut was_in_match: Local<bool>,
) {
    let in_match = screen.is_some_and(|screen| {
        matches!(
            *screen,
            frame::AppScreen::ClassSelect | frame::AppScreen::InGame
        )
    });
    let entered = in_match && !*was_in_match;
    *was_in_match = in_match;
    if in_match && (entered || menu.preset_due) {
        menu.preset_due = false;
        queue.push_line(PRESETS[menu.preset.clamp(0, 2) as usize]);
    }
}

/// Follow a LAN action through: send the lobby lines once the network side is
/// ready for them.
pub(crate) fn drive_lan(
    mut menu: ResMut<ModMenu>,
    time: Res<Time>,
    mut queue: ResMut<ConsoleQueue>,
) {
    let now = time.elapsed_secs();
    match lan::status() {
        lan::Status::Joined(host) if menu.lan_joined != Some(host) => {
            menu.lan_joined = Some(host);
            menu.lan_next = Some((now + LAN_JOIN_DELAY, LAN_JOIN_LINES));
        }
        lan::Status::Joined(_) => {}
        _ => menu.lan_joined = None,
    }
    if let Some((at, lines)) = menu.lan_next
        && now >= at
    {
        menu.lan_next = None;
        // The lobby and the lobby list are the game's own screens.
        menu.open = false;
        queue.push_line(lines);
    }
}

/// The frame-rate readout, refreshed twice a second.
pub(crate) fn fps_readout(
    menu: Res<ModMenu>,
    time: Res<Time<Real>>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text>,
    mut window: Local<(u32, f32)>,
) {
    let Some(entity) = menu.fps else {
        return;
    };
    if let Ok(mut node) = nodes.get_mut(entity) {
        let display = if menu.show_fps {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    if !menu.show_fps {
        return;
    }
    window.0 += 1;
    window.1 += time.delta_secs();
    if window.1 >= 0.5 {
        if let Ok(mut text) = texts.get_mut(entity) {
            text.0 = format!("{:.0} FPS", window.0 as f32 / window.1);
        }
        *window = (0, 0.0);
    }
}

/// Bring what is drawn in line with the menu's state.
pub(crate) fn update(
    mut menu: ResMut<ModMenu>,
    last_line: Option<Res<ConsoleLine>>,
    time: Res<Time>,
    mut nodes: Query<&mut Node>,
    mut fills: Query<&mut BackgroundColor>,
    mut texts: Query<&mut Text>,
    screen: Option<Res<frame::AppScreen>>,
    authority: Option<Res<net::AuthorityWorld>>,
) {
    menu.skins_only = crate::skins_only(screen.as_deref(), authority.as_deref());
    if menu.skins_only
        && let Some(skins) = TABS.iter().position(|tab| tab.name == "SKINS")
        && menu.tab != skins
    {
        menu.tab = skins;
        menu.focus = 0;
    }
    let show = |nodes: &mut Query<&mut Node>, entity: Option<Entity>, visible: bool| {
        let Some(mut node) = entity.and_then(|entity| nodes.get_mut(entity).ok()) else {
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
    };
    show(&mut nodes, menu.root, menu.open);
    if !menu.open {
        return;
    }
    for (index, page) in menu.pages.iter().enumerate() {
        show(&mut nodes, Some(*page), index == menu.tab);
    }
    // Only the skins tab is offered when the rest is cheats.
    for (entity, target) in &menu.targets {
        if let Target::Tab(tab) = *target {
            show(&mut nodes, Some(*entity), !menu.skins_only || TABS[tab].name == "SKINS");
        }
    }
    if menu
        .tapped
        .is_some_and(|(_, until)| time.elapsed_secs() >= until)
    {
        menu.tapped = None;
    }
    let tapped = menu.tapped.map(|(entity, _)| entity);
    let lan_status = lan::status();
    for (entity, target) in &menu.targets {
        let on = match *target {
            Target::Tab(tab) => tab == menu.tab,
            Target::Close => false,
            Target::Item(tab, item) => match TABS[tab].items[item].act {
                Act::Line(_) | Act::Prompt(_) => false,
                Act::Lan(Lan::Host | Lan::HostOnline) => lan_status == lan::Status::Hosting,
                Act::Lan(Lan::Join) => {
                    matches!(lan_status, lan::Status::Searching | lan::Status::Joined(_))
                }
                Act::Lan(Lan::Stop) => false,
                Act::Skin(index) => crate::skins::selected() == usize::try_from(index).ok(),
                Act::Toggle { slot, .. } => menu.toggles[slot],
                Act::Set(Setting::Skin, _) => false,
                Act::Set(Setting::RenderHeight, value) => value == menu.render_height,
                Act::Set(Setting::TextureSkip, value) => value == menu.texture_skip,
                Act::Set(Setting::Preset, value) => value == menu.preset,
                Act::Set(Setting::ShowFps, value) => (value == 1) == menu.show_fps,
                Act::Set(Setting::UncapFps, value) => (value == 1) == menu.uncap_fps,
            },
        };
        let focused = menu.pad_used
            && matches!(*target, Target::Item(tab, item) if tab == menu.tab && item == menu.focus);
        let fill = if tapped == Some(*entity) {
            BUTTON_TAP
        } else if focused {
            BUTTON_FOCUS
        } else if on {
            BUTTON_ON
        } else {
            BUTTON
        };
        if let Ok(mut background) = fills.get_mut(*entity)
            && background.0 != fill
        {
            background.0 = fill;
        }
    }
    let mut write = |entity: Option<Entity>, value: &str| {
        if let Some(mut text) = entity.and_then(|entity| texts.get_mut(entity).ok())
            && text.0 != value
        {
            text.0 = value.to_owned();
        }
    };
    write(menu.hint, TABS[menu.tab].hint);
    // The LAN tab reports the network; every other tab, what the game said.
    if TABS[menu.tab].name == "LAN" {
        write(menu.status, &lan_status.line());
    } else {
        write(
            menu.status,
            last_line.as_ref().map_or("", |line| line.0.as_str()),
        );
    }
}
