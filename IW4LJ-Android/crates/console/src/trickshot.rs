//! Trickshot commands for a host.
//!
//! The game's own scripts decide when a match ends, from `scr_*` dvars they
//! re-read every second. These commands set those dvars, so a trickshot lobby
//! is the real game mode with its rules bent, not a mode of its own:
//!
//! - `lastkill` makes the next kill the last: every score limit drops to one
//!   point and Search and Destroy needs one round, so the script ends the match
//!   on it and plays the final killcam.
//! - `trickshot` restarts the map as Search and Destroy with no round timer,
//!   respawns on, and the first round won taking the match — the classic
//!   trickshot lobby. The setup is kept for the maps loaded after it, until
//!   `trickshot off`.

use bevy::prelude::*;

use crate::{ConsoleCommand, ConsoleLine, ConsoleQueue, ConsoleRegistry, ConsoleSettings, ConsoleState};

/// The gametypes whose score limit the scripts watch.
const SCORED_MODES: [&str; 8] = ["dm", "war", "dom", "koth", "sab", "ctf", "dd", "sd"];

/// The rules of a trickshot Search and Destroy match.
const TRICKSHOT_RULES: [(&str, &str); 7] = [
    // No round clock: line the shot up for as long as it takes.
    ("scr_sd_timelimit", "0"),
    // The first round won is the match, so its last kill is the final killcam.
    ("scr_sd_winlimit", "1"),
    ("scr_sd_roundlimit", "0"),
    ("scr_sd_roundswitch", "0"),
    // Respawn after a miss instead of sitting out the round.
    ("scr_sd_numlives", "0"),
    ("scr_game_playerwaittime", "0"),
    ("scr_game_matchstarttime", "0"),
];

pub(crate) fn register_trickshot_commands(registry: &mut ConsoleRegistry) {
    for (name, usage) in [
        (
            "lastkill",
            "lastkill — the next kill ends the match with the final killcam (needs cheats)",
        ),
        (
            "stillbot",
            "stillbot — add a bot that never moves or shoots, standing where you are now (needs cheats)",
        ),
        (
            "trickshot",
            "trickshot [off] — restart the map as Search and Destroy with no round timer; the first round won takes the match",
        ),
    ] {
        if registry.resolve(name).is_none() {
            registry.register(crate::CommandSpec::new(name).usage(usage));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn route_trickshot_commands(
    mut events: MessageReader<ConsoleCommand>,
    mut console: ResMut<ConsoleState>,
    settings: Res<ConsoleSettings>,
    mut line: ResMut<ConsoleLine>,
    mut setup: ResMut<frame::MatchSetup>,
    mut queue: ResMut<ConsoleQueue>,
    has_world: Res<State<frame::MatchScope>>,
    mut authority: Option<ResMut<net::AuthorityWorld>>,
    presented: Res<net::PresentedSnapshot>,
    local: Res<net::LocalPresentClient>,
    mut bot_add: ResMut<bots::BotAddQueue>,
    mut looping: ResMut<frame::TrickshotLoop>,
    mut lives: MessageReader<frame::LifeStarted>,
) {
    let capacity = settings.log_capacity;
    let mut echo = |msg: String| {
        diag::info!(Console, "{msg}");
        line.0 = msg.clone();
        console.echo(msg, capacity);
    };
    let live = *has_world.get() == frame::MatchScope::Live;
    for cmd in events.read() {
        match cmd.name.as_str() {
            "stillbot" => {
                if authority.as_deref().is_some_and(|a| !a.0.cheats_enabled()) {
                    echo("stillbot: cheats are off".into());
                    continue;
                }
                let Some(ps) = presented.alive_player(local.0) else {
                    echo("stillbot: not alive — spawn a class first".into());
                    continue;
                };
                // Beside the player, not on them: two bodies in one place trap
                // the player. About two body widths ahead, facing back.
                let yaw = ps.viewangles[1].to_radians();
                let origin = [
                    ps.origin[0] + yaw.cos() * 70.0,
                    ps.origin[1] + yaw.sin() * 70.0,
                    ps.origin[2],
                ];
                let angles = [0.0, (ps.viewangles[1] + 180.0) % 360.0, 0.0];
                looping.bot = Some((origin, angles));
                bot_add.push_dummy_at(origin, angles);
                echo("stillbot: a frozen bot is joining at your position".into());
            }
            "lastkill" => {
                let Some(authority) = authority.as_deref_mut().filter(|_| live) else {
                    echo("lastkill: host a match first".into());
                    continue;
                };
                if !authority.0.cheats_enabled() {
                    echo("lastkill: cheats are off".into());
                    continue;
                }
                end_on_next_kill(authority);
                // Every attempt is the last kill of a match: when the scripts
                // end it, the same map loads again with the bot where it was.
                looping.active = true;
                echo("lastkill: the next kill wins the match, then the round starts over".into());
            }
            "trickshot" => match cmd.args.first().map(String::as_str) {
                Some("off" | "0") => {
                    *setup = frame::MatchSetup::default();
                    *looping = frame::TrickshotLoop::default();
                    echo("trickshot off: the next map loads with the normal rules".into());
                }
                None | Some("on" | "1") => {
                    setup.gametype = Some("sd".to_owned());
                    setup.rules = TRICKSHOT_RULES
                        .iter()
                        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                        .collect();
                    if live {
                        queue.push_line("map_restart");
                        echo("trickshot: restarting as Search and Destroy — add a still bot, then land the shot".into());
                    } else {
                        echo("trickshot on: the next map loads as Search and Destroy".into());
                    }
                }
                Some(_) => echo("usage: trickshot [off]".into()),
            },
            _ => {}
        }
    }

    // A looped round has begun again: put the limits and the bot back.
    let respawned = lives.read().any(|life| life.client == local.0.0);
    if respawned && looping.rearm {
        if let Some(authority) = authority.as_deref_mut()
            && authority.0.cheats_enabled()
        {
            end_on_next_kill(authority);
        }
        if let Some((origin, angles)) = looping.bot {
            bot_add.push_dummy_at(origin, angles);
        }
        looping.rearm = false;
        echo("trickshot: round restarted — the bot is back where it stood".into());
    }
}

/// Make the next kill the last of the match, whatever the gametype.
fn end_on_next_kill(authority: &mut net::AuthorityWorld) {
    for mode in SCORED_MODES {
        authority
            .0
            .set_gsc_dvar(&format!("scr_{mode}_scorelimit"), "1");
    }
    authority.0.set_gsc_dvar("scr_sd_winlimit", "1");
    authority.0.set_gsc_dvar("scr_sab_winlimit", "1");
}
