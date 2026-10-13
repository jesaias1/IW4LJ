use render_anim::geometry::{append_mesh, install_retained_packed};
use render_scene::SmodelPassMaterial;

use crate::model_draw::{FxModelAssetDraw, FxModelDrawPlan};

pub const XMODEL_PACKED_UNAVAILABLE: &str =
    "xmodel merge GfxPackedVertex missing or count-mismatched; decoded float is not packed VB";
pub const XMODEL_PACKED_EMPTY_PLAN: &str = "xmodel plan has no vertices";

pub fn fx_model_packed_mismatch(
    surfaces: &[render_anim::fpv_pose::PosedModelSurface],
) -> Option<(usize, usize, usize)> {
    surfaces.iter().find_map(|surface| {
        let decoded = surface
            .mesh
            .attribute(bevy::prelude::Mesh::ATTRIBUTE_POSITION)
            .map_or(0, |positions| positions.len());
        let packed = surface.packed_vertices.len();
        (packed != decoded).then_some((surface.surface_index, packed, decoded))
    })
}

pub fn append_fx_model_asset(
    plan: &mut FxModelDrawPlan,
    model_index: usize,
    lod: u8,
    surfaces: &[render_anim::fpv_pose::PosedModelSurface],
    materials: &[Option<SmodelPassMaterial>],
) -> Vec<(u32, u32)> {
    let geometry = std::sync::Arc::make_mut(&mut plan.geometry);
    let mut asset_surfaces = Vec::new();
    let vertices_empty = geometry.vertices.is_empty();
    let mut packed_ok = vertices_empty
        || matches!(
            geometry.packed_vertices,
            asset_world::PackedVertexPayload::Iw4(_)
        );
    let mut packed = match std::mem::replace(
        &mut geometry.packed_vertices,
        asset_world::PackedVertexPayload::Unavailable {
            source_layout: XMODEL_PACKED_UNAVAILABLE,
        },
    ) {
        asset_world::PackedVertexPayload::Iw4(rows) => rows,
        asset_world::PackedVertexPayload::Unavailable { .. } => Vec::new(),
    };
    for (surface, material) in surfaces.iter().zip(materials) {
        let Some(material) = material else { continue };
        let before = geometry.vertices.len();
        let Some((start, count)) =
            append_mesh(&surface.mesh, &mut geometry.vertices, &mut geometry.indices)
        else {
            continue;
        };
        let decoded = geometry.vertices.len() - before;
        if packed_ok && surface.packed_vertices.len() == decoded {
            packed.extend_from_slice(&surface.packed_vertices);
        } else {
            packed_ok = false;
            packed.clear();
        }
        let surface_index = geometry.surface_ranges.len() as u32;
        geometry.surface_ranges.push((start, count));
        let material_index = geometry.materials.len() as u32;
        geometry.materials.push(material.clone());
        asset_surfaces.push((surface_index, material_index));
    }
    geometry.packed_vertices = install_retained_packed(
        packed_ok,
        packed,
        geometry.vertices.len(),
        XMODEL_PACKED_EMPTY_PLAN,
        XMODEL_PACKED_UNAVAILABLE,
    );
    geometry.assets.push(FxModelAssetDraw {
        model_index,
        lod,
        surfaces: asset_surfaces.clone(),
    });
    let topology = {
        let mut revisions = render_frame::SourceRevisions::default();
        revisions.set_topology_from(
            &geometry.indices,
            &geometry.surface_ranges,
            geometry.vertices.len(),
        );
        revisions.topology
    };
    let mut rev = plan.revision;
    plan.revisions.topology = topology;
    plan.revisions.bump_packed_write(&mut rev);
    plan.revision = rev;
    asset_surfaces
}
