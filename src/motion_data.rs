use bevy::prelude::*;
use motion_data_asset::MotionDataAsset;

pub mod motion_data_asset;
pub mod motion_data_player;

pub struct MotionDataPlugin;

impl Plugin for MotionDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(motion_data_asset::MotionDataAssetPlugin);
    }
}

#[derive(Resource, Debug, Deref, DerefMut)]
pub struct MotionDataHandle(pub Handle<MotionDataAsset>);

#[derive(bevy::ecs::system::SystemParam)]
pub struct MotionData<'w> {
    pub assets: Res<'w, Assets<MotionDataAsset>>,
    pub handle: Res<'w, MotionDataHandle>,
}

impl MotionData<'_> {
    pub fn get(&self) -> Option<&MotionDataAsset> {
        self.assets.get(&**self.handle)
    }
}
