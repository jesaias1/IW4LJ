use super::BindButton;

use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::MouseButton;

macro_rules! key_catalog {
    (
        keys {
            $(
                $k_button:ident {
                    code: $k_code:literal,
                    display: $k_display:literal,
                    names: [$($k_name:literal),+ $(,)?],
                    parse: [$($k_parse:ident),+ $(,)?],
                    hint: [$($k_hint:literal)?] $(,)?
                },
            )*
        }
        mouse {
            $(
                $m_button:ident {
                    code: $m_code:literal,
                    display: $m_display:literal,
                    names: [$($m_name:literal),+ $(,)?],
                    parse: [$($m_parse:ident),+ $(,)?],
                    hint: [$($m_hint:literal)?] $(,)?
                },
            )*
        }
    ) => {
        pub const BINDABLE_KEYS: &[&str] = &[
            $( $($k_hint,)? )*
            $( $($m_hint,)? )*
            "mwheelup",
            "mwheeldown",
            "BUTTON_A",
            "BUTTON_B",
            "BUTTON_X",
            "BUTTON_Y",
            "BUTTON_LSHLDR",
            "BUTTON_RSHLDR",
            "BUTTON_LTRIG",
            "BUTTON_RTRIG",
            "BUTTON_LSTICK",
            "BUTTON_RSTICK",
            "DPAD_UP",
            "DPAD_DOWN",
            "DPAD_LEFT",
            "DPAD_RIGHT",
            "BUTTON_BACK",
        ];

        pub fn host_keynum(button: BindButton) -> usize {
            match button {
                BindButton::Mouse(mouse) => match mouse {
                    $(MouseButton::$m_button => $m_code,)*
                    _ => 185,
                },
                BindButton::WheelUp => 186,
                BindButton::WheelDown => 187,
                BindButton::Pad(pad) => pad.keynum(),
                BindButton::Key(key) => match key {
                    $(KeyCode::$k_button => $k_code,)*
                    _ => 62,
                },
            }
        }

        pub fn parse_button_name(name: &str) -> Option<Vec<BindButton>> {
            let name = name.trim().to_ascii_lowercase();
            match name.as_str() {
                $(
                    $( $k_name )|+ => {
                        Some(vec![$(BindButton::Key(KeyCode::$k_parse),)+])
                    }
                )*
                $(
                    $( $m_name )|+ => {
                        Some(vec![$(BindButton::Mouse(MouseButton::$m_parse),)+])
                    }
                )*
                "mwheelup" | "wheelup" => Some(vec![BindButton::WheelUp]),
                "mwheeldown" | "wheeldown" => Some(vec![BindButton::WheelDown]),
                other => {
                    let upper = other.to_ascii_uppercase();
                    super::PadButton::ALL
                        .into_iter()
                        .find(|pad| pad.console_name() == upper || pad.label() == upper
                            || format!("BUTTON_{}", pad.generic_label()) == upper)
                        .map(|pad| vec![BindButton::Pad(pad)])
                }
            }
        }

        pub fn parse_key_name(name: &str) -> Option<Vec<BindButton>> {
            parse_button_name(name)
        }

        pub fn display_button(button: BindButton) -> String {
            match button {
                BindButton::Key(key) => display_key(key),
                BindButton::Mouse(mouse) => match mouse {
                    $(MouseButton::$m_button => $m_display.into(),)*
                    other => format!("{other:?}"),
                },
                BindButton::WheelUp => "MWHEELUP".into(),
                BindButton::WheelDown => "MWHEELDOWN".into(),
                BindButton::Pad(pad) => pad.console_name().into(),
            }
        }

        fn display_key(key: KeyCode) -> String {
            match key {
                $(KeyCode::$k_button => $k_display,)*
                other => return format!("{other:?}"),
            }
            .to_owned()
        }
    };
}

key_catalog! {
    keys {
        KeyA { code: 1, display: "a", names: ["a"], parse: [KeyA], hint: ["a"] },
        KeyB { code: 2, display: "b", names: ["b"], parse: [KeyB], hint: ["b"] },
        KeyC { code: 3, display: "c", names: ["c"], parse: [KeyC], hint: ["c"] },
        KeyD { code: 4, display: "d", names: ["d"], parse: [KeyD], hint: ["d"] },
        KeyE { code: 5, display: "e", names: ["e"], parse: [KeyE], hint: ["e"] },
        KeyF { code: 6, display: "f", names: ["f"], parse: [KeyF], hint: ["f"] },
        KeyG { code: 7, display: "g", names: ["g"], parse: [KeyG], hint: ["g"] },
        KeyH { code: 8, display: "h", names: ["h"], parse: [KeyH], hint: ["h"] },
        KeyI { code: 9, display: "i", names: ["i"], parse: [KeyI], hint: ["i"] },
        KeyJ { code: 10, display: "j", names: ["j"], parse: [KeyJ], hint: ["j"] },
        KeyK { code: 11, display: "k", names: ["k"], parse: [KeyK], hint: ["k"] },
        KeyL { code: 12, display: "l", names: ["l"], parse: [KeyL], hint: ["l"] },
        KeyM { code: 13, display: "m", names: ["m"], parse: [KeyM], hint: ["m"] },
        KeyN { code: 14, display: "n", names: ["n"], parse: [KeyN], hint: ["n"] },
        KeyO { code: 15, display: "o", names: ["o"], parse: [KeyO], hint: ["o"] },
        KeyP { code: 16, display: "p", names: ["p"], parse: [KeyP], hint: ["p"] },
        KeyQ { code: 17, display: "q", names: ["q"], parse: [KeyQ], hint: ["q"] },
        KeyR { code: 18, display: "r", names: ["r"], parse: [KeyR], hint: ["r"] },
        KeyS { code: 19, display: "s", names: ["s"], parse: [KeyS], hint: ["s"] },
        KeyT { code: 20, display: "t", names: ["t"], parse: [KeyT], hint: ["t"] },
        KeyU { code: 21, display: "u", names: ["u"], parse: [KeyU], hint: ["u"] },
        KeyV { code: 22, display: "v", names: ["v"], parse: [KeyV], hint: ["v"] },
        KeyW { code: 23, display: "w", names: ["w"], parse: [KeyW], hint: ["w"] },
        KeyX { code: 24, display: "x", names: ["x"], parse: [KeyX], hint: ["x"] },
        KeyY { code: 25, display: "y", names: ["y"], parse: [KeyY], hint: ["y"] },
        KeyZ { code: 26, display: "z", names: ["z"], parse: [KeyZ], hint: ["z"] },
        Digit0 { code: 27, display: "0", names: ["0"], parse: [Digit0], hint: ["0"] },
        Digit1 { code: 28, display: "1", names: ["1"], parse: [Digit1], hint: ["1"] },
        Digit2 { code: 29, display: "2", names: ["2"], parse: [Digit2], hint: ["2"] },
        Digit3 { code: 30, display: "3", names: ["3"], parse: [Digit3], hint: ["3"] },
        Digit4 { code: 31, display: "4", names: ["4"], parse: [Digit4], hint: ["4"] },
        Digit5 { code: 32, display: "5", names: ["5"], parse: [Digit5], hint: ["5"] },
        Digit6 { code: 33, display: "6", names: ["6"], parse: [Digit6], hint: ["6"] },
        Digit7 { code: 34, display: "7", names: ["7"], parse: [Digit7], hint: ["7"] },
        Digit8 { code: 35, display: "8", names: ["8"], parse: [Digit8], hint: ["8"] },
        Digit9 { code: 36, display: "9", names: ["9"], parse: [Digit9], hint: ["9"] },
        Space { code: 37, display: "SPACE", names: ["space"], parse: [Space], hint: ["space"] },
        Tab { code: 38, display: "TAB", names: ["tab"], parse: [Tab], hint: ["tab"] },
        ShiftLeft {
            code: 39,
            display: "SHIFT",
            names: ["shift", "shiftleft", "lshift"],
            parse: [ShiftLeft, ShiftRight],
            hint: ["shift"],
        },
        ShiftRight {
            code: 40,
            display: "SHIFT",
            names: ["shiftright", "rshift"],
            parse: [ShiftRight],
            hint: [],
        },
        ControlLeft {
            code: 41,
            display: "CTRL",
            names: ["ctrl", "control", "ctrlleft", "lctrl"],
            parse: [ControlLeft, ControlRight],
            hint: ["ctrl"],
        },
        ControlRight {
            code: 42,
            display: "CTRL",
            names: ["ctrlright", "rctrl"],
            parse: [ControlRight],
            hint: [],
        },
        AltLeft {
            code: 43,
            display: "ALT",
            names: ["alt", "altleft", "lalt"],
            parse: [AltLeft, AltRight],
            hint: ["alt"],
        },
        AltRight {
            code: 44,
            display: "ALT",
            names: ["altright", "ralt"],
            parse: [AltRight],
            hint: [],
        },
        Enter { code: 45, display: "ENTER", names: ["enter", "return"], parse: [Enter], hint: ["enter"] },
        Backspace { code: 46, display: "BACKSPACE", names: ["backspace"], parse: [Backspace], hint: ["backspace"] },
        Escape { code: 47, display: "ESCAPE", names: ["escape", "esc"], parse: [Escape], hint: ["escape"] },
        ArrowUp { code: 48, display: "UPARROW", names: ["uparrow", "up"], parse: [ArrowUp], hint: ["uparrow"] },
        ArrowDown { code: 49, display: "DOWNARROW", names: ["downarrow", "down"], parse: [ArrowDown], hint: ["downarrow"] },
        ArrowLeft { code: 50, display: "LEFTARROW", names: ["leftarrow", "left"], parse: [ArrowLeft], hint: ["leftarrow"] },
        ArrowRight { code: 51, display: "RIGHTARROW", names: ["rightarrow", "right"], parse: [ArrowRight], hint: ["rightarrow"] },
        Semicolon { code: 52, display: "SEMICOLON", names: ["semicolon"], parse: [Semicolon], hint: ["semicolon"] },
        Quote { code: 53, display: "QUOTE", names: ["quote"], parse: [Quote], hint: ["quote"] },
        Comma { code: 54, display: "COMMA", names: ["comma"], parse: [Comma], hint: ["comma"] },
        Period { code: 55, display: "PERIOD", names: ["period"], parse: [Period], hint: ["period"] },
        Slash { code: 56, display: "SLASH", names: ["slash"], parse: [Slash], hint: ["slash"] },
        Minus { code: 57, display: "MINUS", names: ["minus"], parse: [Minus], hint: ["minus"] },
        Equal { code: 58, display: "EQUAL", names: ["equal", "equals"], parse: [Equal], hint: ["equal"] },
        BracketLeft { code: 59, display: "BRACKETLEFT", names: ["bracketleft", "["], parse: [BracketLeft], hint: ["bracketleft"] },
        BracketRight { code: 60, display: "BRACKETRIGHT", names: ["bracketright", "]"], parse: [BracketRight], hint: ["bracketright"] },
        Backslash { code: 61, display: "BACKSLASH", names: ["backslash"], parse: [Backslash], hint: ["backslash"] },
    }
    mouse {
        Left { code: 180, display: "MOUSE1", names: ["mouse1", "mouseleft", "lmb"], parse: [Left], hint: ["mouse1"] },
        Right { code: 181, display: "MOUSE2", names: ["mouse2", "mouseright", "rmb"], parse: [Right], hint: ["mouse2"] },
        Middle { code: 182, display: "MOUSE3", names: ["mouse3", "mousemiddle", "mmb"], parse: [Middle], hint: ["mouse3"] },
        Back { code: 183, display: "MOUSE4", names: ["mouse4"], parse: [Back], hint: ["mouse4"] },
        Forward { code: 184, display: "MOUSE5", names: ["mouse5"], parse: [Forward], hint: ["mouse5"] },
    }
}
