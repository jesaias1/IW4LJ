use anim_iw4::{
    XANIM_LEGS_PARENT_WEIGHT_WHEN_TORSO, client_anim_blend_ms, client_anim_playback_rate,
    goal_time_from_blend_ms,
};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerAnimProperties {
    pub ladder: bool,
    pub stationary: bool,
    pub blend_ms: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerBodyBranches {
    pub legs: crate::XAnimNodeId,
    pub torso: crate::XAnimNodeId,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ClientAnimSample {
    pub origin: [f32; 3],
    pub time_ms: i32,
    pub move_speed: f32,
    pub ladder: bool,
}

pub fn apply_player_anim_rates(
    runtime: &mut crate::XAnimTreeRuntime,
    legs_sample: &mut ClientAnimSample,
    torso_sample: &mut ClientAnimSample,
    legs_index: u16,
    torso_index: u16,
    origin: [f32; 3],
    pose_time_ms: i32,
) -> Result<(), String> {
    apply_one_client_anim_rate(runtime, legs_index, origin, pose_time_ms, legs_sample)?;
    if torso_index != 0 {
        apply_one_client_anim_rate(runtime, torso_index, origin, pose_time_ms, torso_sample)?;
    }
    Ok(())
}

fn apply_one_client_anim_rate(
    runtime: &mut crate::XAnimTreeRuntime,
    index: u16,
    origin: [f32; 3],
    pose_time_ms: i32,
    sample: &mut ClientAnimSample,
) -> Result<(), String> {
    if index == 0 {
        return Ok(());
    }
    if pose_time_ms < sample.time_ms {
        sample.time_ms = 0;
    }
    let Some(rate) = client_anim_playback_rate(
        origin,
        sample.origin,
        pose_time_ms,
        sample.time_ms,
        sample.move_speed,
        sample.ladder,
    ) else {
        return Ok(());
    };
    runtime
        .set_rate(crate::XAnimNodeId(index), rate)
        .map_err(|error| error.to_string())?;
    *sample = ClientAnimSample {
        origin,
        time_ms: pose_time_ms,
        ..*sample
    };
    Ok(())
}

pub fn apply_player_anim_goals(
    runtime: &mut crate::XAnimTreeRuntime,
    branches: PlayerBodyBranches,
    old_legs: u16,
    old_torso: u16,
    legs_index: u16,
    torso_index: u16,
    legs_restart: bool,
    torso_restart: bool,
    old_legs_moving: bool,
    old_torso_moving: bool,
    new_legs_moving: bool,
    new_torso_moving: bool,
    authored_blend_ms: [i32; 2],
) -> Result<(), String> {
    let legs_time = goal_time_from_blend_ms(client_anim_blend_ms(
        legs_index,
        authored_blend_ms[0],
        old_legs != 0,
        false,
        old_legs_moving,
        new_legs_moving,
    ));
    let torso_time = goal_time_from_blend_ms(client_anim_blend_ms(
        torso_index,
        authored_blend_ms[1],
        old_torso != 0,
        true,
        old_torso_moving,
        new_torso_moving,
    ));
    if old_legs != 0 && old_legs != legs_index {
        runtime
            .set_goal_weight(crate::XAnimNodeId(old_legs), 0.0, legs_time)
            .map_err(|error| error.to_string())?;
    }
    if old_torso != 0 && old_torso != torso_index {
        runtime
            .set_goal_weight(crate::XAnimNodeId(old_torso), 0.0, torso_time)
            .map_err(|error| error.to_string())?;
    }
    if old_legs == legs_index && !legs_restart {
        runtime
            .set_goal_weight(crate::XAnimNodeId(legs_index), 1.0, legs_time)
            .map_err(|error| error.to_string())?;
    } else {
        runtime
            .set_complete_goal_weight_in(crate::XAnimNodeId(legs_index), 0.0, 1.0, legs_time)
            .map_err(|error| error.to_string())?;
    }
    if torso_index != 0 {
        if old_torso == torso_index && !torso_restart {
            runtime
                .set_goal_weight(crate::XAnimNodeId(torso_index), 1.0, torso_time)
                .map_err(|error| error.to_string())?;
        } else {
            runtime
                .set_complete_goal_weight_in(crate::XAnimNodeId(torso_index), 0.0, 1.0, torso_time)
                .map_err(|error| error.to_string())?;
        }
    }
    runtime
        .set_complete_goal_weight_in(
            branches.legs,
            0.0,
            if torso_index != 0 {
                XANIM_LEGS_PARENT_WEIGHT_WHEN_TORSO
            } else {
                1.0
            },
            torso_time,
        )
        .map_err(|error| error.to_string())?;
    runtime
        .set_complete_goal_weight_in(
            branches.torso,
            0.0,
            if torso_index != 0 { 1.0 } else { 0.0 },
            torso_time,
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn overlay_legs_clip(legs: &crate::AnimClip, torso: &crate::AnimClip) -> crate::AnimClip {
    let names: HashSet<&str> = torso
        .tracks
        .iter()
        .map(|track| track.name.as_str())
        .collect();
    let mut overlayed = legs.clone();
    overlayed
        .tracks
        .retain(|track| !names.contains(track.name.as_str()));
    overlayed
}
