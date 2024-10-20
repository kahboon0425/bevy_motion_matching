use bevy::prelude::*;

pub mod bvh_manager;
pub mod camera;
pub mod motion_data;
pub mod motion_matching;
pub mod player;
pub mod pose_matching;
pub mod scene_loader;
pub mod trajectory;
pub mod ui;

pub struct MotionMatchingAppPlugin;

impl Plugin for MotionMatchingAppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins.set(AssetPlugin {
                mode: AssetMode::Processed,
                ..default()
            }),
            motion_data::MotionDataPlugin,
            scene_loader::SceneLoaderPlugin,
            bvh_manager::BvhManagerPlugin,
            trajectory::InputTrajectory,
            camera::CameraPlugin,
            ui::UiPlugin,
            player::PlayerPlugin,
            motion_matching::MotionMatchingPlugin,
            // TODO: Merge this into motion matching plugin.
            pose_matching::PoseMatchingPlugin,
        ));

        app.init_state::<GameMode>();
    }
}

#[derive(States, Default, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum GameMode {
    #[default]
    None,
    Config,
    Play,
}
