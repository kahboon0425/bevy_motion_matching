use bevy::prelude::*;
use bvh::*;

mod bvh;
mod camera;
mod input_trajectory;
mod motion_database;
mod player;
mod scene_loader;
mod ui;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            bvh_library::BvhLibraryPlugin,
            scene_loader::SceneLoaderPlugin,
            bvh_player::BvhPlayerPlugin,
            input_trajectory::InputTrajectory,
            camera::CameraPlugin,
            ui::UiPlugin,
            motion_database::MotionDatabasePlugin,
            player::PlayerPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

pub fn setup(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle::default());
}
