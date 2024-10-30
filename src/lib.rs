use bevy::prelude::*;

pub mod action;
pub mod bvh_manager;
pub mod camera;
pub mod draw_axes;
pub mod motion_data;
pub mod motion_matching;
pub mod player;
pub mod pose_matching;
pub mod record;
pub mod scene_loader;
pub mod trajectory;
pub mod transform2d;
pub mod ui;

pub const BVH_SCALE_RATIO: f32 = 0.01;

pub struct MotionMatchingAppPlugin;

impl Plugin for MotionMatchingAppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins.set(AssetPlugin {
                mode: AssetMode::Processed,
                ..default()
            }),
            draw_axes::DrawAxesPlugin,
            transform2d::Transform2dPlugin,
            action::ActionPlugin,
            motion_data::MotionDataPlugin,
            scene_loader::SceneLoaderPlugin,
            bvh_manager::BvhManagerPlugin,
            trajectory::TrajectoryPlugin,
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
