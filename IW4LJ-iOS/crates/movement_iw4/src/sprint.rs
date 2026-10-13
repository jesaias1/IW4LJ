use playerstate_iw4::{PlayerState, UserCmd, buttons, pm_flags};

#[must_use]
pub fn get_max_sprint_time(sprint_duration_scale: f32, player_sprint_time_seconds: f32) -> i32 {
    let scale = if sprint_duration_scale > 0.0 {
        sprint_duration_scale
    } else {
        1.0
    };
    let mut max_ms = ftol_round(scale * (player_sprint_time_seconds * 1000.0));
    if max_ms > 0x3fff {
        max_ms = 0x3fff;
    }
    max_ms
}

#[must_use]
pub fn sprint_recharge_penalty_ms(recharge_pause_seconds: f32) -> i32 {
    ftol_round(recharge_pause_seconds * 1000.0)
}

fn ftol_round(x: f32) -> i32 {
    if x >= 0.0 {
        (x + 0.5) as i32
    } else {
        (x - 0.5) as i32
    }
}

#[must_use]
pub fn sprint_forward_below_minimum(forwardmove: i8, forward_minimum: i32) -> bool {
    !((forwardmove as i32) > forward_minimum)
}

const BUTTON_SPRINT: u32 = 0x2;

fn sprint_interfering_buttons(ps: &PlayerState, pressed: u32, mask: u32) -> bool {
    let mask = if ps.last_weapon_hand == 1
        && !(8..=12).contains(&ps.weaponstate_primary)
        && pressed & buttons::USE_RELOAD != 0
    {
        mask & !(buttons::USE_RELOAD | buttons::RELOAD)
    } else {
        mask
    };
    pressed & mask != 0
}

pub const PERK_MARATHON: u32 = 0x0200_0000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SprintContext {
    pub weapon_max_sprint_time: i32,

    pub sprint_forever: bool,

    pub min_sprint_time_seconds: f32,

    pub sprint_delay_seconds: f32,

    pub sprint_forward_minimum: i32,

    pub stand_up_clear: bool,

    pub sprint_recharge_pause_seconds: f32,
}

#[must_use]
pub fn sprint_start_interfering_buttons(
    ps: &PlayerState,
    forwardmove: i8,
    buttons: u32,
    forward_minimum: i32,
) -> bool {
    let flags = ps.pm_flags;
    if (flags & pm_flags::LADDER) != 0
        || sprint_forward_below_minimum(forwardmove, forward_minimum)
        || sprint_interfering_buttons(ps, buttons, 0xcc35)
    {
        return true;
    }
    if ps.leanf != 0.0 || (flags & 0x801c) != 0 {
        return true;
    }
    !weapon_state_admits_sprint(ps, flags, false)
}

#[must_use]
pub fn sprint_ending_buttons(
    ps: &PlayerState,
    forwardmove: i8,
    buttons: u32,
    forward_minimum: i32,
) -> bool {
    let flags = ps.pm_flags;
    if (flags & 0x8018) != 0
        || sprint_forward_below_minimum(forwardmove, forward_minimum)
        || sprint_interfering_buttons(ps, buttons, 0xcf35)
    {
        return true;
    }
    if ps.leanf != 0.0 {
        return true;
    }
    !weapon_state_admits_sprint(ps, flags, true)
}

fn weapon_state_admits_sprint(ps: &PlayerState, flags: u32, ending: bool) -> bool {
    if !ending && (flags & pm_flags::JUMPING) != 0 && ps.pm_time == 0 {
        return true;
    }
    let state = ps.weaponstate_primary;
    if state == 0xd || state == 0xe || state == 0xf {
        return false;
    }
    if (0x10..=0x15).contains(&state) {
        return false;
    }
    if ending && (state == 0x1d || state == 0x1e) {
        return false;
    }
    true
}

pub fn end_sprint(ps: &mut PlayerState, cmd: &UserCmd) {
    if (ps.pm_flags & pm_flags::SPRINTING) == 0 {
        return;
    }
    ps.sprint_delay = 0;
    ps.last_sprint_end = cmd.server_time;
    ps.pm_flags &= !pm_flags::SPRINTING;
    if (cmd.buttons & BUTTON_SPRINT) != 0 {
        ps.sprint_button_up_required = 1;
    }
}

#[must_use]
pub fn sprint_time_remaining(ps: &PlayerState, server_time: i32, context: SprintContext) -> i32 {
    let maximum = context.weapon_max_sprint_time;
    let mut remaining = maximum;
    if (ps.perks[0] & PERK_MARATHON) != 0 {
        return remaining;
    }

    let start = ps.last_sprint_start;
    if !context.sprint_forever && start != 0 {
        if ps.last_sprint_end < start {
            remaining = ps
                .sprint_start_max_length
                .wrapping_sub(server_time)
                .wrapping_add(start);
        } else {
            let base = if ps.sprint_delay == 0 {
                ps.sprint_start_max_length
                    .wrapping_add(ps.last_sprint_end.wrapping_mul(-2))
            } else {
                ps.last_sprint_end
                    .wrapping_mul(-2)
                    .wrapping_sub(sprint_recharge_penalty_ms(
                        context.sprint_recharge_pause_seconds,
                    ))
                    .wrapping_add(ps.sprint_start_max_length)
            };
            remaining = base.wrapping_add(start).wrapping_add(server_time);
        }
    }

    if remaining < 0 {
        remaining = 0;
    }
    if maximum < remaining {
        remaining = maximum;
    }
    remaining
}

#[must_use]
fn sprint_has_room(ps: &PlayerState, stand_up_clear: bool) -> bool {
    if (ps.pm_flags & 3) == 0 {
        return true;
    }
    stand_up_clear
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SprintResult {
    Unchanged,

    Started,

    Ended,
}

pub fn update_sprint(
    ps: &mut PlayerState,
    cmd: &UserCmd,
    old_buttons: u32,
    context: SprintContext,
) -> SprintResult {
    if ps.sprint_button_up_required != 0 && (cmd.buttons & BUTTON_SPRINT) == 0 {
        ps.sprint_button_up_required = 0;
    }

    if ps.pm_type != 0 && ps.pm_type != 1 {
        return end_for_dead_movement_type(ps, cmd);
    }

    if context.weapon_max_sprint_time <= 0 {
        return end_for_dead_movement_type(ps, cmd);
    }

    if (ps.pm_flags & pm_flags::SPRINTING) != 0 {
        let unlimited = (ps.perks[0] & PERK_MARATHON) != 0 || context.sprint_forever;
        if !unlimited
            && ps.sprint_start_max_length <= cmd.server_time.wrapping_sub(ps.last_sprint_start)
        {
            end_sprint(ps, cmd);
            ps.sprint_delay = 1;
            return SprintResult::Ended;
        }
        if sprint_ending_buttons(
            ps,
            cmd.forwardmove,
            cmd.buttons,
            context.sprint_forward_minimum,
        ) {
            end_sprint(ps, cmd);
            return SprintResult::Ended;
        }
        if (old_buttons & BUTTON_SPRINT) != 0 || (cmd.buttons & BUTTON_SPRINT) == 0 {
            return SprintResult::Unchanged;
        }

        end_sprint(ps, cmd);
        ps.sprint_button_up_required = 1;
        return SprintResult::Ended;
    }

    if ps.sprint_delay != 0 {
        let since_end = cmd.server_time.wrapping_sub(ps.last_sprint_end) as f32;
        if since_end < context.sprint_delay_seconds * 1000.0 {
            return SprintResult::Unchanged;
        }
    }
    if (cmd.buttons & BUTTON_SPRINT) == 0
        || (ps.pm_flags & pm_flags::SPRINT_BLOCKED) != 0
        || ps.sprint_button_up_required != 0
        || sprint_start_interfering_buttons(
            ps,
            cmd.forwardmove,
            cmd.buttons,
            context.sprint_forward_minimum,
        )
        || !sprint_has_room(ps, context.stand_up_clear)
    {
        return SprintResult::Unchanged;
    }

    let budget = sprint_time_remaining(ps, cmd.server_time, context);
    if (budget as f32) <= context.min_sprint_time_seconds * 1000.0 {
        return SprintResult::Unchanged;
    }

    ps.sprint_start_max_length = budget;
    ps.last_sprint_start = cmd.server_time;
    ps.pm_flags |= pm_flags::SPRINTING;
    SprintResult::Started
}

fn end_for_dead_movement_type(ps: &mut PlayerState, cmd: &UserCmd) -> SprintResult {
    if (ps.pm_flags & pm_flags::SPRINTING) == 0 {
        return SprintResult::Unchanged;
    }
    ps.sprint_delay = 0;
    ps.last_sprint_end = cmd.server_time;
    ps.pm_flags &= !pm_flags::SPRINTING;
    if (cmd.buttons & BUTTON_SPRINT) != 0 {
        ps.sprint_button_up_required = 1;
    }
    SprintResult::Ended
}
