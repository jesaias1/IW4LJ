use bevy::prelude::*;

use crate::match_load::register_match_load_systems;

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        register_match_load_systems(app);
    }
}
