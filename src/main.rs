use bevy::prelude::*;
use bvh::*;
use motion_database::load_motion_data_onto;

mod bvh;
mod camera;
mod input_trajectory;
mod motion_database;
mod scene_loader;
mod ui;
mod pose_matching;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            bvh_library::BvhLibraryPlugin,
            scene_loader::SceneLoaderPlugin,
            bvh_player::BvhPlayerPlugin,
            input_trajectory::InputTrajectoryPlugin,
            camera::CameraPlugin,
            ui::UiPlugin,
            motion_database::MotionDatabasePlugin,
            pose_matching::PoseMatchingPlugin
        ))
        .add_systems(Startup, setup)
        .add_systems(Startup, load_motion_data_onto)
        .run();
}

pub fn setup(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle::default());
}
