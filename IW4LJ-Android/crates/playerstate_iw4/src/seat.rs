use crate::{ENTITYNUM_NONE, PlayerState, eflags, other_flags};

pub const HITSCAN_KILL_CAM_ENTITY: i32 = ENTITYNUM_NONE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeatFocus {
    pub kill_cam_entity: i32,

    pub kill_cam_look_at_entity: i32,

    pub kill_cam_client_num: i32,
}

impl SeatFocus {
    pub const fn hitscan(attacker_client_num: i32) -> Self {
        Self {
            kill_cam_entity: HITSCAN_KILL_CAM_ENTITY,
            kill_cam_look_at_entity: ENTITYNUM_NONE,
            kill_cam_client_num: attacker_client_num,
        }
    }
}

pub fn rebase_archived_timers(ps: &mut PlayerState, delta_ms: i32) {
    for timer in [
        &mut ps.command_time,
        &mut ps.pm_time,
        &mut ps.jump_time,
        &mut ps.view_height_lerp_time,
        &mut ps.shellshock_time,
        &mut ps.stun_time,
    ] {
        if *timer != 0 {
            *timer = timer.wrapping_add(delta_ms);
        }
    }
    ps.delta_time = ps.delta_time.wrapping_add(delta_ms);
}

pub fn apply_killcam_seat(
    viewer: &PlayerState,
    archived: &PlayerState,
    focus: SeatFocus,
) -> PlayerState {
    let mut seat = *archived;

    seat.e_flags =
        ((archived.e_flags ^ viewer.e_flags) & eflags::KILLCAM_PRESERVED) ^ archived.e_flags;

    seat.other_flags =
        (archived.other_flags & !other_flags::PLAYER) | other_flags::DEAD_KILLCAM_TPV;

    seat.kill_cam_entity = focus.kill_cam_entity;
    seat.kill_cam_look_at_entity = focus.kill_cam_look_at_entity;
    seat.kill_cam_client_num = focus.kill_cam_client_num;

    seat
}

pub fn seat_matches_archived_except_exceptions(
    seat: &PlayerState,
    archived: &PlayerState,
    viewer: &PlayerState,
    focus: SeatFocus,
) -> bool {
    let expected_e_flags =
        ((archived.e_flags ^ viewer.e_flags) & eflags::KILLCAM_PRESERVED) ^ archived.e_flags;
    let expected_other =
        (archived.other_flags & !other_flags::PLAYER) | other_flags::DEAD_KILLCAM_TPV;

    if seat.e_flags != expected_e_flags {
        return false;
    }
    if seat.other_flags != expected_other {
        return false;
    }
    if seat.kill_cam_entity != focus.kill_cam_entity
        || seat.kill_cam_look_at_entity != focus.kill_cam_look_at_entity
        || seat.kill_cam_client_num != focus.kill_cam_client_num
    {
        return false;
    }

    let mut want = *archived;
    want.e_flags = expected_e_flags;
    want.other_flags = expected_other;
    want.kill_cam_entity = focus.kill_cam_entity;
    want.kill_cam_look_at_entity = focus.kill_cam_look_at_entity;
    want.kill_cam_client_num = focus.kill_cam_client_num;
    seat == &want
}
