use bevy::prelude::*;
use bvh::*;

mod bvh;
mod camera;
mod motion_database;
mod nearest_trajectories_poses_retriever;
mod player;
mod scene_loader;
mod trajectory;
mod ui;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            bvh_library::BvhLibraryPlugin,
            scene_loader::SceneLoaderPlugin,
            bvh_player::BvhPlayerPlugin,
            trajectory::InputTrajectory,
            camera::CameraPlugin,
            ui::UiPlugin,
            motion_database::MotionDatabasePlugin,
            player::PlayerPlugin,
            nearest_trajectories_poses_retriever::NearestTrajectoryRetrieverPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

pub fn setup(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle::default());
}
