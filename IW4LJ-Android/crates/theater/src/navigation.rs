use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;

use crate::browser::{Browser, BrowserButton, BrowserRoot};
use crate::editor::{EditorAction, EditorButton, EditorRoot, TheaterState, TimelineBar};

#[derive(Default)]
struct Navigation {
    selected: usize,
    focus: Option<Entity>,
    repeat: Option<(IVec2, f32)>,
    root: Option<Entity>,
    library: Option<bool>,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TheaterNavigation;

pub(crate) fn register(app: &mut App) {
    app.add_systems(
        Update,
        navigate
            .in_set(frame::ClientSet::Ui)
            .in_set(TheaterNavigation),
    );
}

fn navigate(
    mut commands: Commands,
    roots: Query<Entity, Or<(With<BrowserRoot>, With<EditorRoot>)>>,
    children: Query<&Children>,
    buttons: Query<
        (
            Entity,
            &UiGlobalTransform,
            Option<Ref<bevy::picking::hover::Hovered>>,
            Option<&BrowserButton>,
            Option<&EditorButton>,
            Has<TimelineBar>,
        ),
        With<bevy::ui_widgets::Button>,
    >,
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<&Gamepad>,
    active: Res<frame::ActivePad>,
    devices: Res<frame::InputDevices>,
    console: Res<console::ConsoleState>,
    overlay: Res<frame::UiOverlayInput>,
    time: Res<Time>,
    mut browser: ResMut<Browser>,
    mut editor: ResMut<TheaterState>,
    mut nav: Local<Navigation>,
) {
    if !overlay.0 || !devices.focused || console.open {
        nav.repeat = None;
        return;
    }
    let pad = active.0.and_then(|entity| pads.get(entity).ok());
    if !browser.open {
        if keys.just_pressed(KeyCode::Escape)
            || pad.is_some_and(|p| p.just_pressed(GamepadButton::East))
        {
            editor.actions.push(EditorAction::Exit);
            return;
        }
        if let Some(pad) = pad {
            for (button, action) in [
                (GamepadButton::Start, EditorAction::Controls),
                (GamepadButton::Select, EditorAction::Tools),
                (GamepadButton::North, EditorAction::Hud),
                (GamepadButton::West, EditorAction::Pause),
                (GamepadButton::LeftTrigger, EditorAction::Player(false)),
                (GamepadButton::RightTrigger, EditorAction::Player(true)),
                (GamepadButton::LeftTrigger2, EditorAction::Event(false)),
                (GamepadButton::RightTrigger2, EditorAction::Event(true)),
            ] {
                if pad.just_pressed(button) {
                    editor.actions.push(action);
                }
            }
        }
    } else if pad.is_some_and(|p| {
        p.just_pressed(GamepadButton::East) || p.just_pressed(GamepadButton::Start)
    }) {
        browser.actions.push(BrowserButton::Back);
        return;
    }
    let Some(root) = roots.iter().next() else {
        return;
    };
    let mut ordered = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if buttons.contains(entity) {
            ordered.push(entity);
        }
        if let Ok(descendants) = children.get(entity) {
            stack.extend(descendants.iter().rev());
        }
    }
    if ordered.is_empty() {
        return;
    }
    if nav.library != Some(browser.open) {
        nav.library = Some(browser.open);
        nav.selected = 0;
    }
    nav.selected = nav.selected.min(ordered.len() - 1);
    if nav.root != Some(root) {
        nav.root = Some(root);
        nav.focus = None;
    }
    for (index, &entity) in ordered.iter().enumerate() {
        if let Ok((_, _, interaction, _, _, _)) = buttons.get(entity)
            && interaction.is_some_and(|hovered| hovered.is_changed() && hovered.get())
        {
            nav.selected = index;
        }
    }
    let stick = pad.map_or(Vec2::ZERO, Gamepad::left_stick);
    let held = |button| pad.is_some_and(|p| p.pressed(button));
    let direction = if held(GamepadButton::DPadUp) || stick.y > 0.6 {
        IVec2::NEG_Y
    } else if held(GamepadButton::DPadDown) || stick.y < -0.6 {
        IVec2::Y
    } else if held(GamepadButton::DPadLeft) || stick.x < -0.6 {
        IVec2::NEG_X
    } else if held(GamepadButton::DPadRight) || stick.x > 0.6 {
        IVec2::X
    } else {
        IVec2::ZERO
    };
    let now = time.elapsed_secs();
    let fire = if direction == IVec2::ZERO {
        nav.repeat = None;
        false
    } else if let Some((held, next)) = nav.repeat.filter(|(held, _)| *held == direction) {
        if now >= next {
            nav.repeat = Some((held, now + 0.12));
            true
        } else {
            false
        }
    } else {
        nav.repeat = Some((direction, now + 0.4));
        true
    };
    let mut direction = if fire {
        direction.as_vec2()
    } else {
        Vec2::ZERO
    };
    if browser.open {
        for (key, vector) in [
            (KeyCode::ArrowUp, Vec2::NEG_Y),
            (KeyCode::ArrowDown, Vec2::Y),
            (KeyCode::ArrowLeft, Vec2::NEG_X),
            (KeyCode::ArrowRight, Vec2::X),
        ] {
            if keys.just_pressed(key) {
                direction = vector;
            }
        }
    }
    let current = buttons.get(ordered[nav.selected]).unwrap();
    if current.5 && direction.x != 0.0 {
        editor
            .actions
            .push(EditorAction::Skip(if direction.x < 0.0 {
                -100
            } else {
                100
            }));
    } else if direction != Vec2::ZERO {
        let origin = current.1.translation;
        if let Some((index, _)) = ordered
            .iter()
            .enumerate()
            .filter_map(|(index, &entity)| {
                let position = buttons.get(entity).ok()?.1.translation;
                let delta = position - origin;
                let forward = delta.dot(direction);
                (forward > 1.0).then_some((index, forward + delta.perp_dot(direction).abs() * 3.0))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
        {
            nav.selected = index;
        }
    }
    if keys.just_pressed(KeyCode::Tab) {
        nav.selected = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            (nav.selected + ordered.len() - 1) % ordered.len()
        } else {
            (nav.selected + 1) % ordered.len()
        };
    }
    let entity = ordered[nav.selected];
    if nav.focus != Some(entity) {
        if let Some(previous) = nav.focus
            && let Ok(mut previous) = commands.get_entity(previous)
        {
            previous.remove::<Outline>();
        }
        commands.entity(entity).insert(Outline {
            width: Val::Px(3.0),
            offset: Val::Px(1.0),
            color: Color::srgb(0.8, 0.85, 0.35),
        });
        nav.focus = Some(entity);
    }
    if keys.just_pressed(KeyCode::Enter)
        || pad.is_some_and(|p| p.just_pressed(GamepadButton::South))
    {
        let (_, _, _, browser_button, editor_button, timeline) = buttons.get(entity).unwrap();
        if let Some(button) = browser_button {
            browser.actions.push(button.clone());
        } else if let Some(button) = editor_button {
            editor.actions.push(button.0.clone());
        } else if timeline {
            editor.actions.push(EditorAction::Pause);
        }
    }
}
