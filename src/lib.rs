use bevy::prelude::*;
use bvh::*;

pub mod bvh;
pub mod camera;
pub mod motion_data_asset;
// pub mod motion_database;
pub mod nearest_trajectories_poses_retriever;
pub mod player;
pub mod pose_matching;
pub mod scene_loader;
pub mod trajectory;
pub mod ui;

pub struct MotionMatchingAppPlugin;

impl Plugin for MotionMatchingAppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins,
            bvh_library::BvhLibraryPlugin,
            motion_data_asset::MotionDataAssetPlugin,
            scene_loader::SceneLoaderPlugin,
            bvh_player::BvhPlayerPlugin,
            trajectory::InputTrajectory,
            camera::CameraPlugin,
            ui::UiPlugin,
            player::PlayerPlugin,
            nearest_trajectories_poses_retriever::NearestTrajectoryRetrieverPlugin,
            // motion_database::MotionDatabasePlugin,
            // pose_matching::PoseMatchingPlugin,
        ));
    }
}
