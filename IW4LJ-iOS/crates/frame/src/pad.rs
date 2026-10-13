use bevy::prelude::{Entity, Message, Resource};

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivePad(pub Option<Entity>);

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputDevices {
    pub pad_prompts: bool,
    pub focused: bool,
    pub aiming_with_pad: bool,
}

#[derive(Message)]
pub struct TestControllerRumble;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerIcon {
    A,
    B,
    X,
    Y,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

impl ControllerIcon {
    pub const ALL: [Self; 8] = [
        Self::A,
        Self::B,
        Self::X,
        Self::Y,
        Self::DpadUp,
        Self::DpadDown,
        Self::DpadLeft,
        Self::DpadRight,
    ];

    pub const fn prompt(self) -> &'static str {
        match self {
            Self::A => "\u{e000}",
            Self::B => "\u{e001}",
            Self::X => "\u{e002}",
            Self::Y => "\u{e003}",
            Self::DpadUp => "\u{e004}",
            Self::DpadDown => "\u{e005}",
            Self::DpadLeft => "\u{e006}",
            Self::DpadRight => "\u{e007}",
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        Self::ALL
            .get((c as u32).checked_sub(0xe000)? as usize)
            .copied()
    }

    pub const fn material(self) -> &'static str {
        match self {
            Self::A => "iw4l_controller_a",
            Self::B => "iw4l_controller_b",
            Self::X => "iw4l_controller_x",
            Self::Y => "iw4l_controller_y",
            Self::DpadUp => "iw4l_controller_dpad_up",
            Self::DpadDown => "iw4l_controller_dpad_down",
            Self::DpadLeft => "iw4l_controller_dpad_left",
            Self::DpadRight => "iw4l_controller_dpad_right",
        }
    }
}
