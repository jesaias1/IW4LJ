mod common;
mod fx_lane;
mod smodel;
mod static_lane;
mod xmodel_lane;
use common::{merge_presorted_retained, with_catalog};
pub(crate) use common::{mix_draw_membership, pack_draw, retained_draw_order_tie};
pub use fx_lane::FxDrawLane;
pub(crate) use fx_lane::rebuild_fx_draw_lane;
use smodel::{
    SmodelBucketEmitQueues, SmodelBucketEmitSource, SmodelDestinationPass, SmodelDestinationRecord,
    SmodelPretessBuilder, consume_smodel_bucket_tail, consume_smodel_buckets,
    expand_smodel_destination, packed_lighting_dword_nonzero, push_smodel_surf_bucket,
    smodel_cache_index_u16, smodel_lod_is_rigid, smodel_lod_smc_flag,
};
pub(crate) use static_lane::rebuild_static_draw_lane;
pub use static_lane::{RetainedRebuildCensus, StaticDrawLane};
pub use xmodel_lane::XModelDrawLane;
pub(crate) use xmodel_lane::rebuild_xmodel_draw_lane;

use super::tess::smodel::{LodRampArgs, SmodelGpuPlan, SmodelPlacement, smodel_camera_lod};
use super::tess::xmodel::{XMODEL_OBJECT_ID_VIEWMODEL, XModelDrawPlan};
use crate::prepare::scene::smodel_geom_cache::WorldStaticModelCache;
use crate::prepare::scene::smodel_lighting::WorldSmodelLighting;
use bevy::prelude::*;
use common::world_surface_rank;
use dpvs_iw4::{GfxDrawSurf, pack_xmodel_rigid_skinned_draw_surf};
use std::collections::HashSet;

pub use render_frame::{BspCameraLane, RetainedDrawItem, RetainedDrawKind, SmodelPretessRange};

#[derive(Resource, Clone, Debug, Default)]
pub struct SunShadowCasterPlan {
    pub generation_id: super::MaterialGenerationId,
    pub items: Vec<RetainedDrawItem>,

    pub surface_vis_sun: [Vec<u8>; 2],

    pub smodel_vis_sun: [Vec<u8>; 2],
    pub world_eligible: u32,
    pub world_missing_key: u32,
    pub smodel_eligible: u32,
    pub smodel_excluded: u32,
    pub smodel_missing_key: u32,

    pub cutout_plus23: u32,

    pub cutout_missing_key: u32,

    pub cutout_empty_ib: u32,

    pub cutout_custom0: u32,

    pub bsp_ids: Vec<u16>,
    pub smodel_ids: Vec<u16>,
    pub bsp_ids_far: Vec<u16>,
    pub smodel_ids_far: Vec<u16>,

    pub xmodel_eligible: u32,

    pub xmodel_skipped_viewmodel: u32,

    pub xmodel_missing_key: u32,

    pub xmodel_no_technique: u32,

    pub smodel_no_custom: u32,

    pub lists: render_frame::SunShadowCasterLists,

    pub sun_near_n: usize,

    pub smodel_surf_lists: lighting_iw4::SmodelSurfBucketLists,
    pub smodel_bucket_flush_n: u32,
    pub smodel_bucket_rigid_n: u32,
    pub smodel_bucket_skinned_n: u32,
    pub smodel_bucket_cached_n: u32,
    pub smodel_bucket_unread_n: u32,
    pub smodel_bucket_consume_n: u32,
    pub smodel_bucket_context_refused_n: u32,
    pub smodel_pretess_indices: Vec<u16>,
}

impl SunShadowCasterPlan {
    fn note_smodel_bucket(&mut self, bucket: Option<i32>, full: bool) {
        let Some(bucket) = bucket else {
            self.smodel_bucket_unread_n = self.smodel_bucket_unread_n.saturating_add(1);
            return;
        };
        match bucket & 3 {
            lighting_iw4::SMODEL_BUCKET_CACHED => {
                self.smodel_bucket_cached_n = self.smodel_bucket_cached_n.saturating_add(1);
            }
            lighting_iw4::SMODEL_BUCKET_SKINNED => {
                self.smodel_bucket_skinned_n = self.smodel_bucket_skinned_n.saturating_add(1);
            }
            _ => {
                self.smodel_bucket_rigid_n = self.smodel_bucket_rigid_n.saturating_add(1);
            }
        }
        if full {
            self.smodel_bucket_flush_n = self.smodel_bucket_flush_n.saturating_add(1);
        }
    }
}

#[must_use]
pub(crate) fn bmodel_world_from_local_for_surf(
    surf: u16,
    models: &[asset_world::GfxBrushModelSurfs],
    poses: &[Mat4],
) -> Mat4 {
    let s = usize::from(surf);
    for (index, model) in models.iter().enumerate().skip(1) {
        let start = usize::from(model.start_surf);
        let end = start.saturating_add(usize::from(model.surface_count));
        if s >= start && s < end {
            return poses.get(index).copied().unwrap_or(Mat4::IDENTITY);
        }
    }
    Mat4::IDENTITY
}

pub(crate) fn extra_bmodel_surfs_with_pose(
    models: &[asset_world::GfxBrushModelSurfs],
    already_bit0: &asset_world::SurfaceCastsSunShadow,
    world_from_local: &[Mat4],
) -> Vec<(usize, Mat4)> {
    let mut extra = Vec::new();
    for (index, model) in models.iter().enumerate().skip(1) {
        let pose = world_from_local
            .get(index)
            .copied()
            .unwrap_or(Mat4::IDENTITY);
        let start = usize::from(model.start_surf);
        let end = start.saturating_add(usize::from(model.surface_count));
        let last = end.min(already_bit0.len());
        for surf in start..last {
            if !already_bit0.get(surf) {
                extra.push((surf, pose));
            }
        }
    }
    extra
}

fn emit_world_sun_shadow_surf(
    surf: usize,
    cull: &crate::prepare::scene::world::WorldCull,
    world_plan: &super::WorldDrawGpuPlan,
    catalog: &super::RuntimeMaterialCatalog,
    plan: &mut SunShadowCasterPlan,
    world_from_local: Mat4,
) {
    plan.world_eligible = plan.world_eligible.saturating_add(1);
    let material = cull
        .surface_materials
        .get(surf)
        .copied()
        .flatten()
        .and_then(|id| catalog.derived(id));
    let cutout = material
        .map(|material| super::sun_shadow_cutout_name(&material.name))
        .unwrap_or(false);
    if cutout {
        plan.cutout_plus23 = plan.cutout_plus23.saturating_add(1);
    }
    let empty_ib = world_plan
        .surface_ranges()
        .get(surf)
        .map(|&(_, count)| count == 0)
        .unwrap_or(true);
    if cutout && empty_ib {
        plan.cutout_empty_ib = plan.cutout_empty_ib.saturating_add(1);
    }
    let Some(surf_u16) = u16::try_from(surf).ok() else {
        plan.world_missing_key = plan.world_missing_key.saturating_add(1);
        if cutout {
            plan.cutout_missing_key = plan.cutout_missing_key.saturating_add(1);
        }
        return;
    };
    let packed = if let Some(word) = cull
        .capture
        .packed_draw_surfs
        .get(surf)
        .copied()
        .filter(|word| word.packed != 0)
    {
        word.packed
    } else {
        let Some(material_id) = cull.surface_materials.get(surf).copied().flatten() else {
            plan.world_missing_key = plan.world_missing_key.saturating_add(1);
            if cutout {
                plan.cutout_missing_key = plan.cutout_missing_key.saturating_add(1);
            }
            return;
        };
        let Some(packed) = catalog
            .derived(material_id)
            .and_then(|material| material.baked_draw_surf)
        else {
            plan.world_missing_key = plan.world_missing_key.saturating_add(1);
            if cutout {
                plan.cutout_missing_key = plan.cutout_missing_key.saturating_add(1);
            }
            return;
        };
        let scene_light = cull.surface_primary_lights.get(surf).copied().unwrap_or(0);
        super::with_scene_light_index(packed, scene_light)
    };
    if cutout && GfxDrawSurf::from_packed(packed).custom_index() == 0 {
        plan.cutout_custom0 = plan.cutout_custom0.saturating_add(1);
    }
    let samplers = world_plan
        .surface_sampler_inputs
        .get(surf)
        .copied()
        .unwrap_or_default();

    let world_rank = world_surface_rank(Some(cull), surf, packed, catalog);
    plan.items.push(with_catalog(
        packed,
        world_rank,
        RetainedDrawKind::world_with_pose(surf_u16, world_from_local),
        samplers,
        catalog,
    ));
}

pub(crate) fn sun_shadow_bsp_range(
    dpvs: &crate::prepare::scene::world::WorldDpvs,
    surf_count: usize,
) -> (u32, u32) {
    let begin = dpvs.lit_opaque_begin;
    let end = if dpvs.emissive_surfs_end > begin {
        dpvs.emissive_surfs_end
    } else if dpvs.lit_opaque_end > begin {
        dpvs.lit_opaque_end
    } else {
        surf_count as u32
    };
    (begin, end.min(surf_count as u32))
}

fn emit_world_sun_shadow_from_vis(
    vis: &[u8],
    cull: &crate::prepare::scene::world::WorldCull,
    world_plan: &super::WorldDrawGpuPlan,
    catalog: &super::RuntimeMaterialCatalog,
    plan: &mut SunShadowCasterPlan,
    bsp_ids: &mut Vec<u16>,
) {
    let n = cull.surface_materials.len();
    let (begin, end) = sun_shadow_bsp_range(&cull.dpvs, n);
    if !cull.capture.packed_draw_surfs.is_empty() {
        let cap = end.saturating_sub(begin) as usize;
        if bsp_ids.len() < cap {
            bsp_ids.resize(cap, 0);
        }
        let got = render_frontend::add_bsp_sun_shadow_partition(
            begin,
            end,
            vis,
            cull.capture.casters.words(),
            &cull.capture.packed_draw_surfs,
            &mut bsp_ids[..cap],
        );
        for &surf in &bsp_ids[..got] {
            emit_world_sun_shadow_surf(
                usize::from(surf),
                cull,
                world_plan,
                catalog,
                plan,
                Mat4::IDENTITY,
            );
        }
    } else {
        let last = (end as usize).min(n);
        for surf in begin as usize..last {
            if vis.get(surf).copied().unwrap_or(0) == 0 {
                continue;
            }
            if !cull.capture.casters.get(surf) {
                continue;
            }
            emit_world_sun_shadow_surf(surf, cull, world_plan, catalog, plan, Mat4::IDENTITY);
        }
    }
    for (surf, pose) in extra_bmodel_surfs_with_pose(
        &cull.brush_models,
        &cull.capture.casters,
        &cull.bmodel_world_from_local,
    ) {
        if vis.get(surf).copied().unwrap_or(0) == 0 {
            continue;
        }
        emit_world_sun_shadow_surf(surf, cull, world_plan, catalog, plan, pose);
    }
}

fn emit_smodel_sun_shadow_one(
    placement_i: usize,
    placement: &SmodelPlacement,
    smodel_plan: &SmodelGpuPlan,
    catalog: &super::RuntimeMaterialCatalog,
    plan: &mut SunShadowCasterPlan,
    eye: Option<Vec3>,
    ramp: LodRampArgs,
    buckets: SmodelBucketBakeSrc<'_>,
    queues: &mut SmodelBucketEmitQueues,
    pretess: &mut SmodelPretessBuilder,
    destinations: &mut Vec<SmodelDestinationRecord>,
    custom_skip: &mut HashSet<u32>,
) -> Option<u32> {
    if !super::smodel_casts_sun_shadow(placement.flags) {
        plan.smodel_excluded = plan.smodel_excluded.saturating_add(1);
        return None;
    }
    plan.smodel_eligible = plan.smodel_eligible.saturating_add(1);
    let Some(mesh) = smodel_plan.meshes.get(placement.mesh) else {
        plan.smodel_missing_key = plan.smodel_missing_key.saturating_add(1);
        return None;
    };
    let Some(lod) = smodel_camera_lod(mesh.lod, placement.origin, placement.scale, eye, ramp)
    else {
        plan.smodel_missing_key = plan.smodel_missing_key.saturating_add(1);
        return None;
    };
    let lod_surfs = mesh
        .surfaces_by_lod
        .get(usize::from(lod))
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    if lod_surfs.is_empty() {
        plan.smodel_missing_key = plan.smodel_missing_key.saturating_add(1);
        return None;
    }
    let lod_i = usize::from(lod);
    let packed = placement.packed_lighting.or_else(|| {
        placement.lighting_slot.and_then(|slot| {
            buckets
                .lighting
                .and_then(|l| l.packed_lighting_for_slot(slot))
        })
    });
    let Some(smodel_index) = u16::try_from(placement.lighting_slot.unwrap_or(placement_i)).ok()
    else {
        plan.smodel_bucket_context_refused_n =
            plan.smodel_bucket_context_refused_n.saturating_add(1);
        return None;
    };
    let source = SmodelBucketEmitSource {
        placement: placement_i as u32,
        lod,
        payload: 0,
        world_from_local: placement.world_from_local,
        lighting_handle: 0,
        packed_lighting: packed,
        pass: SmodelDestinationPass::SunShadow,
    };

    let (bucket, full) = push_smodel_surf_bucket(
        &mut plan.smodel_surf_lists,
        queues,
        i32::from(lod),
        false,
        packed_lighting_dword_nonzero(packed),
        smodel_lod_smc_flag(mesh, lod_i),
        smodel_cache_index_u16(buckets.cache, placement.lighting_slot, lod_i),
        smodel_index,
        smodel_lod_is_rigid(mesh, lod_i),
        source,
    );
    plan.note_smodel_bucket(bucket, full);
    if bucket.is_none() {
        let stats = expand_smodel_destination(
            source,
            None,
            smodel_plan,
            catalog,
            buckets.cache,
            pretess,
            destinations,
            Some(custom_skip),
        );
        plan.smodel_no_custom = plan.smodel_no_custom.saturating_add(stats.skipped_custom);
    } else if full
        && let Some(mask) = bucket
            .and_then(|bucket| u8::try_from(bucket).ok())
            .and_then(lighting_iw4::smodel_bucket_mask)
    {
        let consumed = consume_smodel_buckets(
            &mut plan.smodel_surf_lists,
            queues,
            mask,
            smodel_plan,
            catalog,
            buckets.cache,
            pretess,
            destinations,
            Some(custom_skip),
        );
        plan.smodel_bucket_consume_n = plan
            .smodel_bucket_consume_n
            .saturating_add(consumed.buckets);
        plan.smodel_bucket_context_refused_n = plan
            .smodel_bucket_context_refused_n
            .saturating_add(consumed.context_refused);
        plan.smodel_no_custom = plan
            .smodel_no_custom
            .saturating_add(consumed.skipped_custom);
    }
    Some(source.placement)
}

#[derive(Clone, Copy, Default)]
pub struct SmodelBucketBakeSrc<'a> {
    pub pretess_enable: bool,
    pub cache: Option<&'a WorldStaticModelCache>,
    pub lighting: Option<&'a WorldSmodelLighting>,
}

pub fn bake_sun_shadow_caster_plan(
    cull: &crate::prepare::scene::world::WorldCull,
    smodel_draw_insts: &[dpvs_iw4::GfxStaticModelDrawInstShadow],
    world_plan: &super::WorldDrawGpuPlan,
    smodel_plan: &SmodelGpuPlan,
    catalog: &super::RuntimeMaterialCatalog,
    world_vis: Option<&[u8]>,
    smodel_vis: Option<&[u8]>,
    eye: Option<Vec3>,
    ramp: LodRampArgs,
    buckets: SmodelBucketBakeSrc<'_>,
    bsp_ids: &mut Vec<u16>,
    smodel_ids: &mut Vec<u16>,
) -> SunShadowCasterPlan {
    let mut plan = SunShadowCasterPlan {
        generation_id: catalog.generation_id(),
        ..Default::default()
    };
    let mut queues = SmodelBucketEmitQueues::default();
    let mut pretess = SmodelPretessBuilder::new(buckets.pretess_enable);
    let mut destinations = Vec::new();
    let mut destination_sources = Vec::new();
    let mut custom_skip = HashSet::new();
    if let Some(vis) = world_vis {
        emit_world_sun_shadow_from_vis(vis, cull, world_plan, catalog, &mut plan, bsp_ids);
    } else {
        let n = cull.capture.casters.len().max(cull.surface_materials.len());
        for surf in 0..n {
            if !cull.capture.casters.get(surf) {
                continue;
            }
            emit_world_sun_shadow_surf(surf, cull, world_plan, catalog, &mut plan, Mat4::IDENTITY);
        }
        for (surf, pose) in extra_bmodel_surfs_with_pose(
            &cull.brush_models,
            &cull.capture.casters,
            &cull.bmodel_world_from_local,
        ) {
            emit_world_sun_shadow_surf(surf, cull, world_plan, catalog, &mut plan, pose);
        }
    }
    if let Some(vis) = smodel_vis {
        let cap = vis.len().min(smodel_draw_insts.len());
        if smodel_ids.len() < cap {
            smodel_ids.resize(cap, 0);
        }
        let got =
            dpvs_iw4::add_smodel_range_sun_shadow(vis, smodel_draw_insts, &mut smodel_ids[..cap]);
        for &id in &smodel_ids[..got] {
            let authored_slot = usize::from(id);
            let Some(placement_i) = smodel_plan
                .authored_placement_indices
                .get(authored_slot)
                .copied()
                .flatten()
            else {
                plan.smodel_missing_key = plan.smodel_missing_key.saturating_add(1);
                continue;
            };
            let Some(placement) = smodel_plan.placements.get(placement_i) else {
                plan.smodel_missing_key = plan.smodel_missing_key.saturating_add(1);
                continue;
            };
            if let Some(source) = emit_smodel_sun_shadow_one(
                placement_i,
                placement,
                smodel_plan,
                catalog,
                &mut plan,
                eye,
                ramp,
                buckets,
                &mut queues,
                &mut pretess,
                &mut destinations,
                &mut custom_skip,
            ) {
                destination_sources.push(source);
            }
        }
    } else {
        for (placement_i, placement) in smodel_plan.placements.iter().enumerate() {
            if let Some(source) = emit_smodel_sun_shadow_one(
                placement_i,
                placement,
                smodel_plan,
                catalog,
                &mut plan,
                eye,
                ramp,
                buckets,
                &mut queues,
                &mut pretess,
                &mut destinations,
                &mut custom_skip,
            ) {
                destination_sources.push(source);
            }
        }
    }
    let consumed = consume_smodel_bucket_tail(
        &mut plan.smodel_surf_lists,
        &mut queues,
        smodel_plan,
        catalog,
        buckets.cache,
        &mut pretess,
        &mut destinations,
        Some(&mut custom_skip),
    );
    plan.smodel_bucket_consume_n = plan
        .smodel_bucket_consume_n
        .saturating_add(consumed.buckets);
    plan.smodel_bucket_context_refused_n = plan
        .smodel_bucket_context_refused_n
        .saturating_add(consumed.context_refused);
    plan.smodel_no_custom = plan
        .smodel_no_custom
        .saturating_add(consumed.skipped_custom);
    let destination_placements: HashSet<u32> = destinations
        .iter()
        .map(|record| record.placement())
        .collect();
    plan.smodel_missing_key = plan.smodel_missing_key.saturating_add(
        destination_sources
            .into_iter()
            .filter(|source| {
                !destination_placements.contains(source) && !custom_skip.contains(source)
            })
            .count() as u32,
    );
    plan.smodel_pretess_indices = pretess.finish();
    plan.items.extend(
        destinations
            .into_iter()
            .map(|record| record.into_item(catalog)),
    );
    plan.items
        .sort_unstable_by_key(|item| (item.host_sort_key(), retained_draw_order_tie(&item.kind)));
    plan
}

pub(crate) fn fill_smodel_draw_inst_shadow(
    out: &mut Vec<dpvs_iw4::GfxStaticModelDrawInstShadow>,
    cull_dists: &[u16],
    smodel_plan: &SmodelGpuPlan,
) {
    let n = smodel_plan.authored_placement_indices.len();
    if out.len() != n {
        out.clear();
        out.resize(
            n,
            dpvs_iw4::GfxStaticModelDrawInstShadow {
                flags: 0,
                cull_dist: 0,
                origin: [0.0; 3],
            },
        );
    }
    out.fill(dpvs_iw4::GfxStaticModelDrawInstShadow {
        flags: 0,
        cull_dist: 0,
        origin: [0.0; 3],
    });
    for (authored_slot, placement_i) in smodel_plan.authored_placement_indices.iter().enumerate() {
        let Some(placement) = placement_i.and_then(|i| smodel_plan.placements.get(i)) else {
            continue;
        };
        out[authored_slot] = dpvs_iw4::GfxStaticModelDrawInstShadow {
            flags: placement.flags,
            cull_dist: cull_dists.get(authored_slot).copied().unwrap_or(0),
            origin: placement.world_from_local.w_axis.truncate().to_array(),
        };
    }
}

/// One dynamic caster and the sphere the partition test reads. A caster with no
/// sphere is admitted to both partitions, which is what the collector did for
/// every caster before the bound existed.
struct DynamicSunCaster {
    item: RetainedDrawItem,
    bound: Option<render_scene::XModelCasterBound>,
}

/// The dynamic casters a partition keeps, in the order `merge_presorted_retained`
/// needs. Filtering a sorted slice preserves the order, so the sort happens once
/// for both partitions.
fn partition_dynamic_casters(
    dynamic: &[DynamicSunCaster],
    planes: &[[f32; 4]],
) -> Vec<RetainedDrawItem> {
    dynamic
        .iter()
        .filter(|caster| dynamic_caster_kept(caster.bound, planes))
        .map(|caster| caster.item)
        .collect()
}

/// A caster is kept unless its own sphere is wholly outside the partition.
/// No planes and no bound both mean "kept": the partition that states nothing
/// and the producer that states nothing each widen the volume rather than
/// dropping a shadow.
fn dynamic_caster_kept(
    bound: Option<render_scene::XModelCasterBound>,
    planes: &[[f32; 4]],
) -> bool {
    if planes.is_empty() {
        return true;
    }
    bound.is_none_or(|bound| !bound.outside(planes))
}

pub(crate) fn merge_sun_shadow_caster_partitions(
    near: &mut SunShadowCasterPlan,
    far: &mut SunShadowCasterPlan,
    xmodel: Option<&XModelDrawPlan>,
    catalog: &super::RuntimeMaterialCatalog,
    partition_planes: [&[[f32; 4]]; 2],
) -> (Vec<RetainedDrawItem>, Vec<RetainedDrawItem>) {
    near.xmodel_eligible = 0;
    near.xmodel_skipped_viewmodel = 0;
    near.xmodel_missing_key = 0;
    near.xmodel_no_technique = 0;
    let mut dynamic = match xmodel {
        Some(xmodel) => collect_xmodel_sun_shadow_casters(xmodel, catalog, near),
        None => Vec::new(),
    };
    far.xmodel_eligible = near.xmodel_eligible;
    far.xmodel_skipped_viewmodel = near.xmodel_skipped_viewmodel;
    far.xmodel_missing_key = near.xmodel_missing_key;
    far.xmodel_no_technique = near.xmodel_no_technique;
    if dynamic.is_empty() {
        return (
            std::mem::take(&mut near.items),
            std::mem::take(&mut far.items),
        );
    }
    dynamic.sort_unstable_by_key(|caster| {
        (
            caster.item.host_sort_key(),
            retained_draw_order_tie(&caster.item.kind),
        )
    });
    let near_dynamic = partition_dynamic_casters(&dynamic, partition_planes[0]);
    let far_dynamic = partition_dynamic_casters(&dynamic, partition_planes[1]);
    (
        merge_presorted_retained(&near.items, &near_dynamic),
        merge_presorted_retained(&far.items, &far_dynamic),
    )
}

fn collect_xmodel_sun_shadow_casters(
    xmodel: &XModelDrawPlan,
    catalog: &super::RuntimeMaterialCatalog,
    plan: &mut SunShadowCasterPlan,
) -> Vec<DynamicSunCaster> {
    let mut items = Vec::new();
    for draw in &xmodel.draws {
        if draw.object_id == XMODEL_OBJECT_ID_VIEWMODEL || draw.is_scope {
            plan.xmodel_skipped_viewmodel = plan.xmodel_skipped_viewmodel.saturating_add(1);
            continue;
        }
        plan.xmodel_eligible = plan.xmodel_eligible.saturating_add(1);
        let Some(item) = xmodel_shadow_draw(xmodel, draw, catalog) else {
            plan.xmodel_missing_key = plan.xmodel_missing_key.saturating_add(1);
            continue;
        };
        if !super::material_runtime::add_surf_has_technique(
            catalog,
            render_material::MaterialDrawKey::new(item.key, item.material_rank),
            super::TechType(super::SUN_SHADOW_CASTER_TECH),
        ) {
            plan.xmodel_no_technique = plan.xmodel_no_technique.saturating_add(1);
            continue;
        }
        items.push(DynamicSunCaster {
            item,
            bound: draw.caster_bound,
        });
    }
    items
}

pub(super) fn xmodel_shadow_materials(
    xmodel: &XModelDrawPlan,
    catalog: &super::RuntimeMaterialCatalog,
) -> Vec<RetainedDrawItem> {
    xmodel
        .draws
        .iter()
        .filter(|draw| draw.object_id != XMODEL_OBJECT_ID_VIEWMODEL && !draw.is_scope)
        .filter_map(|draw| xmodel_shadow_draw(xmodel, draw, catalog))
        .collect()
}

fn xmodel_shadow_draw(
    xmodel: &XModelDrawPlan,
    draw: &super::tess::xmodel::XModelSurfaceDraw,
    catalog: &super::RuntimeMaterialCatalog,
) -> Option<RetainedDrawItem> {
    let material = xmodel.materials.get(draw.material as usize)?;
    let ordinal = material.material_sorted_index?;
    let baked = catalog
        .material_for_sorted_ordinal(ordinal)?
        .baked_draw_surf?;
    let key =
        pack_xmodel_rigid_skinned_draw_surf(GfxDrawSurf::from_packed(baked), draw.object_id).packed;
    Some(with_catalog(
        key,
        ordinal,
        RetainedDrawKind::XModel {
            surface: draw.surface,
            material: draw.material,
            object_id: draw.object_id,
            world_from_local: draw.world_from_local,
            lighting_handle: draw.lighting_handle,
            packed_lighting: draw.packed_lighting,
            is_scope: draw.is_scope,
            scene_entnum: draw.scene_entnum,
        },
        super::SurfaceSamplerInputs {
            reflection_probe: Some(super::SurfaceReflectionProbeId(draw.reflection_probe_index)),
            ..Default::default()
        },
        catalog,
    ))
}
