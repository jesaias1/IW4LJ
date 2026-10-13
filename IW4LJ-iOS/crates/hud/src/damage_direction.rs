use bevy::prelude::*;
use frame::{LifeStarted, UiDraw, ViewSubject};
use net::{FrameClock, LocalPresentClient, PresentedSnapshot};

use crate::draw2d::{Draw2dProvenance, Draw2dQuad};
use crate::gpu_list::{HudTessPass, TessJob};

#[derive(Component)]
pub(crate) struct DamageDirectionRaster;

#[derive(Clone, Copy, Default)]
struct Hit {
    time: i32,
    duration: i32,
    yaw: f32,
    stun: bool,
}

#[derive(Resource, Default)]
pub(crate) struct DamageDirections {
    hits: [Hit; 12],
    event: Option<u32>,
    subject: Option<ViewSubject>,
    time: Option<i32>,
    weapon_frac: f32,
    ads_in: bool,
    random: u32,
}

pub(crate) fn spawn(root: &mut ChildSpawnerCommands) {
    crate::font_overlay::spawn_overlay(root, DamageDirectionRaster);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update(
    surface: Res<crate::surface::Hud2dSurface>,
    presented: Res<PresentedSnapshot>,
    local: Res<LocalPresentClient>,
    view: Res<ViewSubject>,
    clock: Res<FrameClock>,
    ui_draw: Res<UiDraw>,
    aim: Res<net::ViewweaponAim>,
    weapons: Option<Res<assets::PreparedWeapons>>,
    mut started: MessageReader<LifeStarted>,
    mut directions: ResMut<DamageDirections>,
    mut pass: ResMut<HudTessPass>,
) {
    pass.damage_direction = TessJob::Hide;
    let now = clock.time();
    let reset = started.read().filter(|ev| ev.client == local.0.0).count() != 0
        || directions.subject != Some(*view)
        || directions.time.is_some_and(|time| now < time);
    if reset {
        *directions = DamageDirections::default();
    }
    directions.subject = Some(*view);
    directions.time = Some(now);
    let Some(ps) = presented.player(local.0) else {
        *directions = DamageDirections::default();
        return;
    };
    let dvars = presented
        .snapshot()
        .map(|snapshot| snapshot.meta.script_dvars(local.0));
    if directions
        .event
        .is_some_and(|event| event != ps.damage_event)
        && ps.damage_count > 0
        && !(ps.damage_yaw == 255 && ps.damage_pitch == 255)
    {
        let duration = dvars
            .and_then(|d| d.int("cg_hudDamageIconTime"))
            .unwrap_or(6000)
            .max(0);
        let slot = directions
            .hits
            .iter()
            .enumerate()
            .min_by_key(|(_, hit)| hit.time)
            .map(|(index, _)| index)
            .unwrap_or(0);
        directions.random = directions.random.wrapping_mul(214013).wrapping_add(2531011);
        let jitter = (((directions.random >> 16) & 0x7fff) as f32 / 32768.0 - 0.5) * 20.0;
        directions.hits[slot] = Hit {
            time: now,
            duration,
            yaw: ps.damage_yaw as f32 * (360.0 / 256.0) + jitter,
            stun: ps.damage_flags & 1 != 0,
        };
    }
    directions.event = Some(ps.damage_event);
    if ps.f_weapon_pos_frac != directions.weapon_frac {
        directions.ads_in = ps.f_weapon_pos_frac > directions.weapon_frac;
    }
    directions.weapon_frac = ps.f_weapon_pos_frac;
    let scoped = weapons.as_ref().is_some_and(|weapons| {
        let index = playerstate_iw4::get_viewmodel_weapon_index(ps);
        weapons
            .snapshot_weapon(presented.weapon_epoch(), index)
            .ok()
            .and_then(|weapon| weapon.hud_facts())
            .is_some_and(|facts| {
                hud_iw4::get_weap_reticle_zoom(
                    ps.f_weapon_pos_frac,
                    directions.ads_in,
                    &hud_iw4::WeaponAdsOverlayFacts {
                        ads_zoom_in_frac: facts.ads_zoom_in_frac,
                        ads_zoom_out_frac: facts.ads_zoom_out_frac,
                        overlay_material: u32::from(
                            weapons.registry().overlay_material_of(index).is_some(),
                        ),
                        overlay_reticle: facts.overlay_reticle,
                        ..default()
                    },
                )
                .active
            })
    });
    let flashed = hud_iw4::is_flashbanged(
        now,
        ps.shellshock_time,
        ps.shellshock_duration,
        presented
            .shellshock(local.0)
            .map_or(hud_iw4::SCREEN_BLEND_BLURRED, |shock| shock.screen_type),
    ) != 0;
    if !surface.is_ready()
        || !ui_draw.0
        || flashed
        || ps.pm_flags & playerstate_iw4::pm_flags::MANTLE != 0
        || (scoped
            && dvars
                .and_then(|d| d.int("cg_hudDamageIconInScope"))
                .unwrap_or(0)
                == 0)
        || dvars
            .and_then(|d| d.int("cg_drawDamageDirection"))
            .unwrap_or(1)
            == 0
        || (view.in_killcam() && ps.kill_cam_entity != playerstate_iw4::ENTITYNUM_NONE)
    {
        return;
    }
    let value = |name, default| {
        dvars
            .and_then(|d| d.float(name))
            .filter(|value| value.is_finite())
            .unwrap_or(default)
            .clamp(0.0, 512.0)
    };
    let scale = surface.scale_virtual_to_real();
    let half_width = value("cg_hudDamageIconWidth", 128.0) * scale[0] * 0.5;
    let offset = value("cg_hudDamageIconOffset", 128.0) * scale[1];
    let outer = offset + value("cg_hudDamageIconHeight", 64.0) * scale[1];
    let mut center = [surface.width() * 0.5, surface.height() * 0.5];
    if scoped && aim.live {
        let projection = crate::presentation_scale::PresentationScale::from_window(
            surface.width(),
            surface.height(),
        );
        let factor = projection.factor(crate::presentation_scale::ScaleClass::ProjectionBound);
        center[0] += aim.xhair_x * factor;
        center[1] += aim.xhair_y * factor;
    }
    let mut quads = Vec::new();
    for hit in &directions.hits {
        let age = now.saturating_sub(hit.time);
        if age <= 0 || age >= hit.duration {
            continue;
        }
        let (sin, cos) = (ps.viewangles[1] - hit.yaw).to_radians().sin_cos();
        let xy = [
            [half_width, outer],
            [-half_width, outer],
            [-half_width, offset],
            [half_width, offset],
        ]
        .map(|[x, y]| [center[0] + cos * x - sin * y, center[1] + sin * x + cos * y]);
        quads.push(Draw2dQuad {
            xy,
            st: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            color: [
                1.0,
                1.0,
                1.0,
                (2.0 - 2.0 * age as f32 / hit.duration as f32).min(1.0),
            ],
            material: if hit.stun {
                "hit_direction_stun"
            } else {
                "hit_direction"
            }
            .into(),
            material_namespace: crate::images::HUD_CHROME_NAMESPACE,
            provenance: Draw2dProvenance::CgDraw {
                site: "damage_direction",
            },
            layer: 0,
            clip: None,
        });
    }
    pass.damage_direction = TessJob::Quads(quads);
}
