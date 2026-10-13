use bevy::prelude::*;

#[derive(Component, Clone, Debug)]
pub struct WorldScriptModelInstance {
    pub id: asset_world::ScriptModelId,

    pub authority_owner: Option<sim::AuthorityModelOwner>,
    pub current_model: asset_world::MapXModelAssetKey,
    pub transform: Transform,
    pub lighting_origin: [f32; 3],
    pub dobj_state: xmodel_runtime::DObjSemanticState,
    pub metadata: asset_world::ScriptModelMetadata,

    pub gentity_number: Option<u16>,
}

#[derive(Component, Clone, Debug)]
pub struct WorldDynEntInstance {
    pub index: u16,
    pub ty: asset_world::DynEntType,
    pub current_model: asset_world::MapXModelAssetKey,
    pub transform: Transform,
    pub lighting_origin: [f32; 3],

    pub phys_preset: Option<asset_world::OwnedPhysPreset>,

    pub health: i32,

    pub destroy_fx: Option<String>,

    pub dead: bool,
}
