use bevy::prelude::*;

use super::view_parms::PreparedSceneView;
use super::world::{MapDirPrimaryLight, WorldScene, cell_caster_bits};
use crate::assemble::drawsurf::SunShadowUnmatchedLights;

const SUN_DIR_MATCH_DOT: f32 = 0.9999;

pub fn update_active_sun_stage(
    scene: Option<ResMut<WorldScene>>,
    prepared: Res<PreparedSceneView>,
    dir_light: Option<ResMut<MapDirPrimaryLight>>,
    mut unmatched: ResMut<SunShadowUnmatchedLights>,
) {
    let Some(mut scene) = scene.filter(|scene| !scene.sun_stages.is_empty()) else {
        if *unmatched != SunShadowUnmatchedLights::default() {
            *unmatched = SunShadowUnmatchedLights::default();
        }
        return;
    };
    let Some(mut dir_light) = dir_light else {
        return;
    };
    if !prepared.ready {
        return;
    }
    let eye = prepared.eye.to_array();
    let stage = (1..scene.sun_stages.len())
        .find(|&stage| scene.sun_stages[stage].contains(eye))
        .unwrap_or(0);
    let index = scene.sun_stages[stage].sun_primary_light;
    if scene.active_sun_light == Some(index) {
        return;
    }
    let Some(active) = scene
        .sun_lights
        .iter()
        .find(|(light, _)| *light == index)
        .map(|&(_, light)| light)
    else {
        return;
    };
    let dir = active.direction;
    let mut mask = SunShadowUnmatchedLights::default();
    for (light, other) in &scene.sun_lights {
        let d = other.direction;
        if d[0] * dir[0] + d[1] * dir[1] + d[2] * dir[2] <= SUN_DIR_MATCH_DOT {
            mask.insert(*light);
        }
    }
    *unmatched = mask;
    *dir_light = active;
    scene.active_sun_light = Some(index);
    if let Some(cull) = scene.cull.as_mut() {
        cull.dpvs.cell_caster_bits = cell_caster_bits(&cull.dpvs, dir);
    }
}
