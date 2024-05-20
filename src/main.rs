use bevy::prelude::*;
use bvh::*;

mod bvh;
mod camera;
mod input_trajectory;
mod scene_loader;
mod ui;

fn main() {
    App::new()
        // Bevy plugins
        .add_plugins(DefaultPlugins)
        // Custom plugins
        .add_plugins((
            bvh_library::BvhLibraryPlugin,
            scene_loader::SceneLoaderPlugin,
            bvh_player::BvhPlayerPlugin,
            input_trajectory::InputTrajectoryPlugin,
            camera::CameraPlugin,
            ui::UiPlugin,
        ))
        .run();
}
