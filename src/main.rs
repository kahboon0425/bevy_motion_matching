use bevy::prelude::*;

mod animation_player;
mod bvh_asset;
mod bvh_library;
mod camera;
mod character_loader;
mod input_trajectory;
mod ui;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            bvh_library::BvhLibraryPlugin,
            character_loader::CharacterLoaderPlugin,
            animation_player::AnimationPlayerPlugin,
            input_trajectory::InputTrajectoryPlugin,
            camera::CameraPlugin,
            ui::UiPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

pub fn setup(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle::default());
}
