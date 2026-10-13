//! The letterbox and title of the final killcam.
//!
//! In Modern Warfare 2 the winning kill is replayed between two translucent
//! black bars, under a title naming what is being shown. The bars and the
//! title are drawn here from the same signal the replay itself uses: the
//! presented snapshot says the local player is watching the final killcam.

use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::{Display, GlobalZIndex};

const BAR_HEIGHT: f32 = 13.0;
const BAR_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.62);

#[derive(Component)]
pub(crate) struct KillcamBar;

#[derive(Component)]
pub(crate) struct KillcamTitle;

pub(crate) fn spawn(mut commands: Commands) {
    for top in [true, false] {
        let mut node = Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(BAR_HEIGHT),
            ..default()
        };
        if top {
            node.top = Val::Px(0.0);
        } else {
            node.bottom = Val::Px(0.0);
        }
        commands.spawn((
            node,
            BackgroundColor(BAR_FILL),
            GlobalZIndex(400),
            Pickable::IGNORE,
            KillcamBar,
        ));
    }
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            top: Val::Percent(2.5),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Text::new("ROUND WINNING KILL"),
        TextFont {
            font_size: bevy::text::FontSize::Vh(4.2),
            ..default()
        },
        TextColor(Color::srgb(0.96, 0.96, 0.96)),
        TextLayout::default().with_justify(Justify::Center),
        GlobalZIndex(401),
        Pickable::IGNORE,
        KillcamTitle,
    ));
}

pub(crate) fn update(
    presented: Res<net::PresentedSnapshot>,
    local: Res<net::LocalPresentClient>,
    setup: Res<frame::MatchSetup>,
    mut bars: Query<&mut Node, (With<KillcamBar>, Without<KillcamTitle>)>,
    mut titles: Query<(&mut Node, &mut Text), (With<KillcamTitle>, Without<KillcamBar>)>,
) {
    let snapshot = presented.snapshot();
    let final_killcam = snapshot
        .and_then(|s| s.meta.for_client(local.0))
        .and_then(|meta| meta.killcam_hud)
        .is_some_and(|hud| hud.final_kill);
    // Round-based modes win a round; every other mode wins the game.
    let gametype = snapshot
        .and_then(|s| s.meta.script_dvars(local.0).string("g_gametype"))
        .map(str::to_owned)
        .or_else(|| setup.gametype.clone());
    let display = if final_killcam {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut bars {
        if node.display != display {
            node.display = display;
        }
    }
    for (mut node, mut text) in &mut titles {
        if node.display != display {
            node.display = display;
        }
        let want = if gametype.as_deref().is_some_and(|g| matches!(g, "sd" | "sab" | "dd")) {
            "ROUND WINNING KILL"
        } else {
            "GAME WINNING KILL"
        };
        if final_killcam && text.0 != want {
            text.0 = want.to_owned();
        }
    }
}
