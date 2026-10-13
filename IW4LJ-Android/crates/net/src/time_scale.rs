use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use frame::RuntimeRole;

use crate::client::runtime::LastAdoptedSnapshot;
use crate::{AuthorityClock, AuthorityWorld, FrameClock, GameActive};

#[derive(SystemParam)]
pub(crate) struct TimeScaleInputs<'w> {
    authority: Option<Res<'w, AuthorityWorld>>,
    authority_clock: Option<Res<'w, AuthorityClock>>,
    adopted: Option<Res<'w, LastAdoptedSnapshot>>,
    clock: Option<Res<'w, FrameClock>>,
    active: Option<Res<'w, GameActive>>,
}

pub(crate) fn apply_time_scale(
    role: Res<RuntimeRole>,
    inputs: TimeScaleInputs,
    fixed: Res<Time<Fixed>>,
    mut time: ResMut<Time<Virtual>>,
    presented: Res<crate::PresentedSnapshot>,
    local: Res<crate::LocalPresentClient>,
    mut killcam_start: Local<Option<f64>>,
) {
    let TimeScaleInputs {
        authority,
        authority_clock,
        adopted,
        clock,
        active,
    } = inputs;
    let (plan, game_ms) = if role.runs_authority() {
        (
            authority.as_ref().and_then(|world| world.0.slow_motion()),
            authority_clock
                .as_ref()
                .map_or(0.0, |clock| f64::from(clock.time_ms))
                + fixed.overstep().as_secs_f64() * 1000.0,
        )
    } else {
        let snapshot = adopted
            .as_ref()
            .and_then(|a| a.next().or(a.snap.as_deref()))
            .filter(|_| active.as_ref().is_none_or(|active| active.0));
        let game_ms = clock.as_ref().filter(|clock| clock.started()).map_or_else(
            || snapshot.map_or(0.0, |s| f64::from(s.tick.0) * f64::from(sim::MATCH_TICK_MS)),
            |clock| f64::from(clock.time()),
        );
        (
            snapshot.and_then(|s| s.meta.objectives.slow_motion),
            game_ms,
        )
    };
    let mut speed = plan
        .filter(|p| p.valid())
        .map_or(1.0, |p| p.sample(game_ms));
    // The match scripts never slow the final killcam down, so it is done here:
    // the replay runs at full speed, then eases into slow motion as the
    // winning kill approaches.
    let in_final_killcam = presented
        .snapshot()
        .and_then(|s| s.meta.for_client(local.0))
        .and_then(|meta| meta.killcam_hud)
        .is_some_and(|hud| hud.final_kill);
    if in_final_killcam {
        let started = *killcam_start.get_or_insert(game_ms);
        speed = speed.min(final_killcam_speed((game_ms - started) as f32));
    } else {
        *killcam_start = None;
    }
    time.set_relative_speed(speed);
}

/// Playback speed of the final killcam, `elapsed_ms` of game time into it.
fn final_killcam_speed(elapsed_ms: f32) -> f32 {
    const SLOW_FROM_MS: f32 = 1500.0;
    const SLOWEST_AT_MS: f32 = 5500.0;
    const SLOWEST: f32 = 0.2;
    let t = ((elapsed_ms - SLOW_FROM_MS) / (SLOWEST_AT_MS - SLOW_FROM_MS)).clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    1.0 - (1.0 - SLOWEST) * eased
}
