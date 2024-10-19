use bevy::prelude::*;
use motion_data_asset::MotionDataAsset;
use motion_data_player::MotionDataPlayer;

pub mod motion_data_asset;
pub mod motion_data_player;

pub struct MotionDataPlugin;

impl Plugin for MotionDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            motion_data_asset::MotionDataAssetPlugin,
            motion_data_player::MotionDataPlayerPlugin,
        ));
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct MotionData<'w> {
    pub assets: Res<'w, Assets<MotionDataAsset>>,
    pub player: ResMut<'w, MotionDataPlayer>,
}

impl MotionData<'_> {
    pub fn get(&self) -> Option<&MotionDataAsset> {
        self.assets.get(&self.player.motion_data)
    }

    pub fn jump_to_pose(&mut self, chunk_index: usize, time: f32) {
        self.player.chunk_index = chunk_index;
        self.player.time = time;
    }
}
