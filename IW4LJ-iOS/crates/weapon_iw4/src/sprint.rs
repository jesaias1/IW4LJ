use crate::tick::{WeaponCmd, WeaponCombatFacts, WeaponHandState};
use crate::weaponstate::WeaponState;
use playerstate_iw4::pm_flags;

pub fn weapon_check_for_sprint(
    hand: &mut WeaponHandState,
    facts: &WeaponCombatFacts,
    pm_flags: u32,
) {
    let cmd = WeaponCmd {
        cmd_weapon: hand.weapon as u16,
        pm_flags,
        ..Default::default()
    };
    weapon_check_hands_for_sprint(core::slice::from_mut(hand), facts, &cmd);
}

pub(crate) fn weapon_check_hands_for_sprint(
    hands: &mut [WeaponHandState],
    facts: &WeaponCombatFacts,
    cmd: &WeaponCmd,
) {
    let Some(primary) = hands.first() else {
        return;
    };
    if cmd.cmd_weapon == 0 {
        return;
    }
    let Ok(ws) = WeaponState::from_i32(primary.weaponstate) else {
        return;
    };
    if !check_for_sprint_allowed(ws)
        || hands.iter().skip(1).any(|hand| {
            matches!(
                WeaponState::from_i32(hand.weaponstate),
                Ok(WeaponState::Firing
                    | WeaponState::Rechambering
                    | WeaponState::MeleeInit
                    | WeaponState::MeleeFire
                    | WeaponState::MeleeEnd)
            )
        })
    {
        return;
    }
    let sprinting = cmd.pm_flags & pm_flags::SPRINTING != 0;
    if sprinting && !ws.is_sprint() {
        for hand in hands {
            begin_sprint(hand, facts);
        }
    } else if !sprinting && matches!(ws, WeaponState::SprintIn | WeaponState::SprintLoop) {
        for hand in hands {
            begin_sprint_out(hand, facts, cmd.perks1);
        }
    }
}

fn check_for_sprint_allowed(ws: WeaponState) -> bool {
    !matches!(
        ws,
        WeaponState::Raising
            | WeaponState::RaisingAltswitch
            | WeaponState::Dropping
            | WeaponState::DroppingQuick
            | WeaponState::DroppingAltswitch
            | WeaponState::Firing
            | WeaponState::Rechambering
            | WeaponState::MeleeInit
            | WeaponState::MeleeFire
            | WeaponState::MeleeEnd
            | WeaponState::OffhandInit
            | WeaponState::OffhandPrepare
            | WeaponState::OffhandHold
            | WeaponState::OffhandStart
            | WeaponState::Offhand
            | WeaponState::OffhandEnd
            | WeaponState::NightVisionWear
            | WeaponState::NightVisionRemove
    )
}

fn begin_sprint(hand: &mut WeaponHandState, facts: &WeaponCombatFacts) {
    let time = if facts.sprint_raise_time_ms > 0 {
        facts.sprint_raise_time_ms
    } else {
        1
    };
    hand.weaponstate = WeaponState::SprintIn as i32;
    hand.weapon_time = time;
    hand.weapon_delay = 0;
    hand.shot_count = 0;
    hand.burst_latch = false;
    crate::weap_anim::start_weapon_anim(
        &mut hand.weap_anim,
        crate::weap_anim::weap_anim_event::SPRINT_IN,
    );
}

fn sprint_loop(hand: &mut WeaponHandState) {
    hand.weaponstate = WeaponState::SprintLoop as i32;
    hand.weapon_time = 0;
    hand.weapon_delay = 0;
    crate::weap_anim::start_weapon_anim(
        &mut hand.weap_anim,
        crate::weap_anim::weap_anim_event::SPRINT_LOOP,
    );
}

fn begin_sprint_out(hand: &mut WeaponHandState, facts: &WeaponCombatFacts, perks1: u32) {
    let time = if facts.sprint_drop_time_ms > 0 {
        facts.sprint_drop_time_ms
    } else {
        1
    };
    let time = if perks1 & playerstate_iw4::PERK1_FASTSPRINTRECOVERY != 0 {
        ((time as f32) * playerstate_iw4::SPRINT_RECOVERY_MULTIPLIER) as i32
    } else {
        time
    };
    hand.weaponstate = WeaponState::SprintOut as i32;
    hand.weapon_time = time;
    hand.weapon_delay = 0;
    crate::weap_anim::start_weapon_anim(
        &mut hand.weap_anim,
        crate::weap_anim::weap_anim_event::SPRINT_OUT,
    );
}

pub fn weapon_advance_sprint(
    hand: &mut WeaponHandState,
    weap_flags: &mut u32,
    pm_flags_word: &mut u32,
    pm_type: i32,
) {
    match WeaponState::from_i32(hand.weaponstate) {
        Ok(WeaponState::SprintIn) if hand.weapon_time <= 0 => sprint_loop(hand),
        Ok(WeaponState::SprintOut) if hand.weapon_time <= 0 => {
            crate::melee::weapon_settle_ready(hand, weap_flags, pm_flags_word, pm_type);
        }
        _ => {}
    }
}
