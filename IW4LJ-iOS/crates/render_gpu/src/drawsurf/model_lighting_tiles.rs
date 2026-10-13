use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    Extent3d, Origin3d, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
};
use bevy::render::renderer::RenderQueue;
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp, RenderSystems};
use lighting_iw4::{
    MODEL_LIGHTING_ATLAS_DEPTH, MODEL_LIGHTING_TILE_BYTES, MODEL_LIGHTING_TILE_DIM,
};

#[derive(Resource, Default)]
pub struct ModelLightingTileUploads {
    pub tiles: Vec<ModelLightingTileUpload>,
}

pub struct ModelLightingTileUpload {
    pub image: AssetId<Image>,
    pub origin: (u32, u32),
    pub texels: [u8; MODEL_LIGHTING_TILE_BYTES],
}

pub(super) fn register(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_resource::<ModelLightingTileUploads>()
        .add_systems(
            Render,
            upload_model_lighting_tiles.in_set(RenderSystems::PrepareResources),
        );
}

/// Must run after `PrepareAssets`: a tile for an atlas with no texture yet is
/// dropped, relying on the texture being created from data that holds it.
fn upload_model_lighting_tiles(
    mut uploads: ResMut<ModelLightingTileUploads>,
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
) {
    const ROW_BYTES: u32 = (MODEL_LIGHTING_TILE_DIM * 4) as u32;
    const DIM: u32 = MODEL_LIGHTING_TILE_DIM as u32;
    for tile in uploads.tiles.drain(..) {
        let Some(gpu) = images.get(tile.image) else {
            continue;
        };
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &gpu.texture,
                mip_level: 0,
                origin: Origin3d {
                    x: tile.origin.0,
                    y: tile.origin.1,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            &tile.texels,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ROW_BYTES),
                rows_per_image: Some(DIM),
            },
            Extent3d {
                width: DIM,
                height: DIM,
                depth_or_array_layers: MODEL_LIGHTING_ATLAS_DEPTH,
            },
        );
    }
}
