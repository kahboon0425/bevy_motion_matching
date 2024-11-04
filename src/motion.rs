use bevy::prelude::*;
use motion_asset::MotionAsset;

pub mod chunk;
pub mod joint_info;
pub mod motion_asset;
pub mod motion_player;
pub mod pose_data;
pub mod trajectory_data;

pub struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            motion_asset::MotionAssetPlugin,
            motion_player::MotionPlayerPlugin,
        ));
    }
}

#[derive(Resource, Debug, Deref, DerefMut)]
pub struct MotionHandle(pub Handle<MotionAsset>);

#[derive(bevy::ecs::system::SystemParam)]
pub struct MotionData<'w> {
    pub assets: Res<'w, Assets<MotionAsset>>,
    pub handle: Res<'w, MotionHandle>,
}

impl MotionData<'_> {
    pub fn get(&self) -> Option<&MotionAsset> {
        self.assets.get(&**self.handle)
    }
}
