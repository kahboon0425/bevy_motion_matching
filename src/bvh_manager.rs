use bevy::prelude::*;
use bevy_bvh_anim::prelude::*;

pub mod bvh_gizmos;
pub mod bvh_library;
pub mod bvh_player;

pub struct BvhManagerPlugin;

impl Plugin for BvhManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BvhAssetPlugin,
            bvh_library::BvhLibraryPlugin,
            bvh_player::BvhPlayerPlugin,
            bvh_gizmos::BvhGizmosPlugin,
        ));
    }
}
