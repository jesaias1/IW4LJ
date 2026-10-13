use input_iw4::{ClientInput, command_id_lookup, command_name, key_event};

use std::collections::HashMap;

use bevy::input::ButtonInput;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseButton, MouseScrollUnit};
use bevy::prelude::Resource;

pub const DEFAULT_CONTROLS: &str = include_str!("../assets/default_controls.cfg");

mod key_names;

pub use key_names::{
    BINDABLE_KEYS, display_button, host_keynum, parse_button_name, parse_key_name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindButton {
    Key(KeyCode),
    Mouse(MouseButton),
    WheelUp,
    WheelDown,
    Pad(PadButton),
}

impl BindButton {
    pub fn is_pad(self) -> bool {
        matches!(self, Self::Pad(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PadButton {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Select,
    Start,
}

impl PadButton {
    pub const ALL: [Self; 16] = [
        Self::South,
        Self::East,
        Self::West,
        Self::North,
        Self::LeftBumper,
        Self::RightBumper,
        Self::LeftTrigger,
        Self::RightTrigger,
        Self::LeftStick,
        Self::RightStick,
        Self::DpadUp,
        Self::DpadDown,
        Self::DpadLeft,
        Self::DpadRight,
        Self::Select,
        Self::Start,
    ];

    pub const fn gamepad_button(self) -> GamepadButton {
        match self {
            Self::South => GamepadButton::South,
            Self::East => GamepadButton::East,
            Self::West => GamepadButton::West,
            Self::North => GamepadButton::North,
            Self::LeftBumper => GamepadButton::LeftTrigger,
            Self::RightBumper => GamepadButton::RightTrigger,
            Self::LeftTrigger => GamepadButton::LeftTrigger2,
            Self::RightTrigger => GamepadButton::RightTrigger2,
            Self::LeftStick => GamepadButton::LeftThumb,
            Self::RightStick => GamepadButton::RightThumb,
            Self::DpadUp => GamepadButton::DPadUp,
            Self::DpadDown => GamepadButton::DPadDown,
            Self::DpadLeft => GamepadButton::DPadLeft,
            Self::DpadRight => GamepadButton::DPadRight,
            Self::Select => GamepadButton::Select,
            Self::Start => GamepadButton::Start,
        }
    }

    pub fn from_gamepad_button(button: GamepadButton) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|pad| pad.gamepad_button() == button)
    }

    pub const fn console_name(self) -> &'static str {
        match self {
            Self::South => "BUTTON_A",
            Self::East => "BUTTON_B",
            Self::West => "BUTTON_X",
            Self::North => "BUTTON_Y",
            Self::LeftBumper => "BUTTON_LSHLDR",
            Self::RightBumper => "BUTTON_RSHLDR",
            Self::LeftTrigger => "BUTTON_LTRIG",
            Self::RightTrigger => "BUTTON_RTRIG",
            Self::LeftStick => "BUTTON_LSTICK",
            Self::RightStick => "BUTTON_RSTICK",
            Self::DpadUp => "DPAD_UP",
            Self::DpadDown => "DPAD_DOWN",
            Self::DpadLeft => "DPAD_LEFT",
            Self::DpadRight => "DPAD_RIGHT",
            Self::Select => "BUTTON_BACK",
            Self::Start => "BUTTON_START",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::South => "A",
            Self::East => "B",
            Self::West => "X",
            Self::North => "Y",
            Self::LeftBumper => "LB",
            Self::RightBumper => "RB",
            Self::LeftTrigger => "LT",
            Self::RightTrigger => "RT",
            Self::LeftStick => "LS",
            Self::RightStick => "RS",
            Self::DpadUp => "D-UP",
            Self::DpadDown => "D-DOWN",
            Self::DpadLeft => "D-LEFT",
            Self::DpadRight => "D-RIGHT",
            Self::Select => "BACK",
            Self::Start => "START",
        }
    }

    pub const fn icon_prompt(self) -> &'static str {
        use frame::ControllerIcon;
        match self {
            Self::South => ControllerIcon::A.prompt(),
            Self::East => ControllerIcon::B.prompt(),
            Self::West => ControllerIcon::X.prompt(),
            Self::North => ControllerIcon::Y.prompt(),
            Self::DpadUp => ControllerIcon::DpadUp.prompt(),
            Self::DpadDown => ControllerIcon::DpadDown.prompt(),
            Self::DpadLeft => ControllerIcon::DpadLeft.prompt(),
            Self::DpadRight => ControllerIcon::DpadRight.prompt(),
            _ => self.label(),
        }
    }

    pub const fn generic_label(self) -> &'static str {
        match self {
            Self::South => "SOUTH",
            Self::East => "EAST",
            Self::West => "WEST",
            Self::North => "NORTH",
            Self::LeftBumper => "L-BUMPER",
            Self::RightBumper => "R-BUMPER",
            Self::LeftTrigger => "L-TRIGGER",
            Self::RightTrigger => "R-TRIGGER",
            Self::LeftStick => "L-STICK",
            Self::RightStick => "R-STICK",
            Self::Select => "SELECT",
            _ => self.label(),
        }
    }

    const fn keynum(self) -> usize {
        190 + self as usize
    }
}

pub(crate) fn gameplay_binding(button: BindButton, command: u32, akimbo: bool) -> u32 {
    if !akimbo || !button.is_pad() {
        return command;
    }
    let mapped = match command_name(command) {
        Some("+attack") => "+speed_throw",
        Some("+speed_throw") => "+attack",
        _ => return command,
    };
    command_id_lookup(mapped).expect("built-in controller action")
}

pub fn pad_layout(layout: usize) -> Vec<(PadButton, &'static str)> {
    use PadButton::*;
    let mut binds = vec![
        (RightTrigger, "+attack"),
        (LeftTrigger, "+speed_throw"),
        (RightBumper, "+frag"),
        (LeftBumper, "+smoke"),
        (South, "+gostand"),
        (East, "+stance"),
        (West, "+usereload"),
        (North, "weapnext"),
        (LeftStick, "+breath_sprint"),
        (RightStick, "+melee_zoom"),
        (DpadUp, "+actionslot 1"),
        (DpadDown, "+actionslot 2"),
        (DpadLeft, "+actionslot 3"),
        (DpadRight, "+actionslot 4"),
        (Select, "+scores"),
    ];
    let mut set = |button: PadButton, command: &'static str| {
        binds.retain(|(b, _)| *b != button);
        binds.push((button, command));
    };
    let tactical = |set: &mut dyn FnMut(PadButton, &'static str)| {
        set(East, "+melee_zoom");
        set(RightStick, "+stance");
    };
    match layout {
        1 => tactical(&mut set),
        2 => {
            set(LeftTrigger, "+attack");
            set(RightTrigger, "+speed_throw");
            set(LeftBumper, "+frag");
            set(RightBumper, "+smoke");
        }
        3 | 4 => {
            set(LeftBumper, "+gostand");
            set(South, "+smoke");
            if layout == 4 {
                tactical(&mut set);
            }
        }
        _ => {}
    }
    binds
}

pub(crate) fn wheel_button(y: f32) -> Option<BindButton> {
    if y > 0.0 {
        Some(BindButton::WheelUp)
    } else if y < 0.0 {
        Some(BindButton::WheelDown)
    } else {
        None
    }
}

pub(crate) fn wheel_detents(unit: MouseScrollUnit, y: f32, carry: &mut f32) -> i32 {
    if !y.is_finite() {
        return 0;
    }
    let delta = match unit {
        MouseScrollUnit::Line => y,
        MouseScrollUnit::Pixel => y / 100.0,
    };
    *carry = (*carry + delta).clamp(-32.0, 32.0);
    let detents = carry.trunc() as i32;
    *carry -= detents as f32;
    detents
}

pub struct BindInputs<'a> {
    pub keys: &'a ButtonInput<KeyCode>,
    pub mouse: &'a ButtonInput<MouseButton>,
    pub pad: Option<&'a Gamepad>,
}

impl<'a> BindInputs<'a> {
    pub fn new(keys: &'a ButtonInput<KeyCode>, mouse: &'a ButtonInput<MouseButton>) -> Self {
        Self {
            keys,
            mouse,
            pad: None,
        }
    }

    pub fn with_pad(mut self, pad: Option<&'a Gamepad>) -> Self {
        self.pad = pad;
        self
    }

    pub fn pressed(&self, button: BindButton) -> bool {
        match button {
            BindButton::Key(key) => self.keys.pressed(key),
            BindButton::Mouse(btn) => self.mouse.pressed(btn),
            BindButton::WheelUp | BindButton::WheelDown => false,
            BindButton::Pad(btn) => self
                .pad
                .is_some_and(|pad| pad.pressed(btn.gamepad_button())),
        }
    }

    pub fn just_pressed(&self, button: BindButton) -> bool {
        match button {
            BindButton::Key(key) => self.keys.just_pressed(key),
            BindButton::Mouse(btn) => self.mouse.just_pressed(btn),
            BindButton::WheelUp | BindButton::WheelDown => false,
            BindButton::Pad(btn) => self
                .pad
                .is_some_and(|pad| pad.just_pressed(btn.gamepad_button())),
        }
    }

    pub fn just_released(&self, button: BindButton) -> bool {
        match button {
            BindButton::Key(key) => self.keys.just_released(key),
            BindButton::Mouse(btn) => self.mouse.just_released(btn),
            BindButton::WheelUp | BindButton::WheelDown => false,
            BindButton::Pad(btn) => self
                .pad
                .is_some_and(|pad| pad.just_released(btn.gamepad_button())),
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct KeyBinds {
    map: HashMap<BindButton, u32>,
}

impl KeyBinds {
    pub fn apply_defaults(&mut self) {
        self.map.clear();
        let _ = self.apply_script(DEFAULT_CONTROLS);
        self.apply_pad_layout(0);
    }

    pub fn apply_script(&mut self, script: &str) -> Vec<String> {
        self.apply_script_inner(script, true)
    }

    pub(crate) fn apply_config_script(&mut self, script: &str) -> Vec<String> {
        self.apply_script_inner(script, false)
    }

    fn apply_script_inner(&mut self, script: &str, echo_success: bool) -> Vec<String> {
        let mut output = Vec::new();
        for raw in script.split([';', '\n']) {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let Some(command) = crate::ConsoleCommand::parse(line) else {
                continue;
            };
            match command.name.as_str() {
                "bind" => match self.cmd_bind(&command.args) {
                    Ok(Some(msg)) if echo_success => output.push(msg),
                    Ok(Some(_)) => {}
                    Ok(None) => {}
                    Err(msg) => output.push(msg),
                },
                "unbind" => match self.cmd_unbind(&command.args) {
                    Ok(Some(msg)) if echo_success => output.push(msg),
                    Ok(Some(_)) => {}
                    Ok(None) => {}
                    Err(msg) => output.push(msg),
                },
                "unbindall" => {
                    self.map.clear();
                    if echo_success {
                        output.push("unbindall".into());
                    }
                }
                other => output.push(format!("unknown bind-script command `{other}`")),
            }
        }
        output
    }

    pub fn set(&mut self, button: BindButton, id: u32) {
        self.map.insert(button, id);
    }

    pub fn clear_button(&mut self, button: BindButton) -> bool {
        self.map.remove(&button).is_some()
    }

    pub fn clear_command(&mut self, id: u32) -> bool {
        let before = self.map.len();
        self.map.retain(|_, bound| *bound != id);
        self.map.len() != before
    }

    pub fn clear_all(&mut self) {
        self.map.clear();
    }

    pub fn clear_command_on(&mut self, id: u32, pad: bool) -> bool {
        let before = self.map.len();
        self.map
            .retain(|button, bound| *bound != id || button.is_pad() != pad);
        self.map.len() != before
    }

    pub fn apply_pad_layout(&mut self, layout: usize) {
        self.map.retain(|button, _| !button.is_pad());
        for (button, command) in pad_layout(layout) {
            if let Some(id) = command_id_lookup(command) {
                self.set(BindButton::Pad(button), id);
            }
        }
    }

    pub fn has_pad_binds(&self) -> bool {
        self.map.keys().any(|button| button.is_pad())
    }

    pub fn get(&self, button: BindButton) -> Option<u32> {
        self.map.get(&button).copied()
    }

    pub fn binding_name(&self, button: BindButton) -> Option<&'static str> {
        self.get(button).and_then(command_name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (BindButton, u32)> + '_ {
        self.map.iter().map(|(b, id)| (*b, *id))
    }

    pub fn list_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .map
            .iter()
            .filter_map(|(button, id)| {
                command_name(*id).map(|name| format!("bind {} {name}", display_button(*button)))
            })
            .collect();
        lines.sort();
        lines.dedup();
        lines
    }

    fn cmd_bind(&mut self, args: &[String]) -> Result<Option<String>, String> {
        match args {
            [] => Ok(None),
            [key] => {
                let buttons =
                    parse_button_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
                let names: Vec<&str> = buttons
                    .iter()
                    .filter_map(|button| self.binding_name(*button))
                    .collect();
                if names.is_empty() {
                    Ok(Some(format!("`{key}` is unbound")))
                } else {
                    Ok(Some(format!("bind {key} {}", names[0])))
                }
            }
            [key, action @ ..] => {
                let action = action.join(" ");
                let id = command_id_lookup(&action)
                    .ok_or_else(|| format!("unknown command `{action}`"))?;
                let buttons =
                    parse_button_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
                for button in buttons {
                    self.set(button, id);
                }
                let name = command_name(id).unwrap_or(action.as_str());
                Ok(Some(format!("bind {key} {name}")))
            }
        }
    }

    fn cmd_unbind(&mut self, args: &[String]) -> Result<Option<String>, String> {
        match args {
            [key] => {
                let buttons =
                    parse_button_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
                let mut any = false;
                for button in buttons {
                    any |= self.clear_button(button);
                }
                if any {
                    Ok(Some(format!("unbind {key}")))
                } else {
                    Ok(Some(format!("`{key}` is unbound")))
                }
            }
            _ => Err("usage: unbind <key>".into()),
        }
    }
}

pub(crate) fn pulse_wheel_binding(
    binds: &KeyBinds,
    client: &mut ClientInput,
    button: BindButton,
    now_msec: i32,
    frame_msec: u32,
) {
    let Some(id) = binds.get(button) else { return };
    let key_num = host_keynum(button);
    client.keys[key_num].binding = id;
    key_event(client, key_num, true, now_msec, frame_msec);
    key_event(client, key_num, false, now_msec, frame_msec);
}
