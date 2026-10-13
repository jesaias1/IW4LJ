use crate::frame::FrameWorld;
use crate::{ClientId, ClientLifecycle, PendingPrint};

pub const CHAT_BYTES: usize = 150;

pub fn clean(text: &str) -> String {
    let mut chars = text.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if ch == '^' && chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
        } else if !ch.is_control() {
            if out.len() + ch.len_utf8() >= CHAT_BYTES {
                break;
            }
            out.push(ch);
        }
    }
    out
}

pub fn payload(text: &str) -> [u8; CHAT_BYTES] {
    let text = clean(text);
    let mut bytes = [0; CHAT_BYTES];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
}

pub(crate) fn send(world: &mut FrameWorld, sender: ClientId, team_only: bool, text: &str) -> bool {
    send_text(world, sender, team_only, text, false)
}

pub(crate) fn send_localized(
    world: &mut FrameWorld,
    sender: ClientId,
    team_only: bool,
    key: &str,
) -> bool {
    send_text(world, sender, team_only, key, true)
}

fn send_text(
    world: &mut FrameWorld,
    sender: ClientId,
    team_only: bool,
    text: &str,
    localized: bool,
) -> bool {
    let Some(meta) = world.client_meta(sender) else {
        return false;
    };
    if meta.lifecycle == ClientLifecycle::Connecting {
        return false;
    }
    let playing = meta.lifecycle == ClientLifecycle::Alive;
    let team = meta.client_state_team;
    let team_only = team_only && matches!(team, 1 | 2);
    let end = meta
        .name
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(meta.name.len());
    let name = clean(&String::from_utf8_lossy(&meta.name[..end]));
    let text = clean(text);
    if text.trim().is_empty() {
        return false;
    }
    let line = format!("{name}{}: {text}", crate::HUD_PRINT_ARG_SEPARATOR);
    let dead_chat = world
        .ecs()
        .get_resource::<crate::script::MatchScript>()
        .and_then(|script| script.dvars.get("g_deadChat"))
        .is_some_and(|value| value.parse::<i32>().is_ok_and(|value| value != 0));
    for recipient in world.client_ids_sorted() {
        if world.client_meta(recipient).is_none_or(|meta| {
            meta.lifecycle == ClientLifecycle::Connecting
                || (team_only && meta.client_state_team != team)
                || (!dead_chat && !playing && meta.lifecycle == ClientLifecycle::Alive)
        }) {
            continue;
        }
        world.push_print(PendingPrint {
            recipient: Some(recipient),
            bold: false,
            chat: true,
            template: if localized {
                format!("{team}:localized")
            } else {
                team.to_string()
            },
            arg: line.clone(),
        });
    }
    true
}
