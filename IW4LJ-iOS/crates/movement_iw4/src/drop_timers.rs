use crate::Pml;
use crate::jump;
use playerstate_iw4::{PlayerState, pm_flags};

const PM_TIME_EXPIRY_CLEAR: u32 = 0x2180;

pub fn drop_timers(ps: &mut PlayerState, pml: &Pml) {
    if ps.pm_time != 0 {
        if pml.msec < ps.pm_time {
            ps.pm_time -= pml.msec;
        } else {
            if (ps.pm_flags & pm_flags::JUMPING) != 0 {
                jump::clear_state(ps);
            }
            ps.pm_flags &= !PM_TIME_EXPIRY_CLEAR;
            ps.pm_time = 0;
        }
    }
    if ps.legs_timer > 0 {
        ps.legs_timer -= pml.msec;
        if ps.legs_timer < 0 {
            ps.legs_timer = 0;
        }
    }
    if ps.torso_timer > 0 {
        ps.torso_timer -= pml.msec;
        if ps.torso_timer < 0 {
            ps.torso_timer = 0;
        }
    }
}
