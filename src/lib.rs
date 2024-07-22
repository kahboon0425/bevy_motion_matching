use bevy::prelude::*;

pub mod bvh_manager;
pub mod camera;
pub mod motion_data_asset;
pub mod nearest_trajectories;
pub mod player;
pub mod pose_matching;
pub mod scene_loader;
pub mod trajectory;
pub mod ui;
// pub mod motion_database;

pub struct MotionMatchingAppPlugin;

impl Plugin for MotionMatchingAppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins,
            motion_data_asset::MotionDataAssetPlugin,
            scene_loader::SceneLoaderPlugin,
            bvh_manager::BvhManagerPlugin,
            trajectory::InputTrajectory,
            camera::CameraPlugin,
            ui::UiPlugin,
            player::PlayerPlugin,
            nearest_trajectories::NearestTrajectoryRetrieverPlugin,
            // motion_database::MotionDatabasePlugin,
            // pose_matching::PoseMatchingPlugin,
        ));
    }
}
