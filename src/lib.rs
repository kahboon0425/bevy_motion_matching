use bevy::prelude::*;
use trajectory::Velocity;
use transform2d::Transform2d;

pub mod action;
pub mod bvh_manager;
pub mod camera;
pub mod draw_axes;
pub mod motion;
pub mod motion_matching;
pub mod player;
pub mod record;
pub mod scene_loader;
pub mod trajectory;
pub mod transform2d;
pub mod ui;

pub const BVH_SCALE_RATIO: f32 = 0.01;
pub const LARGE_EPSILON: f32 = 0.0001;

pub struct MotionMatchingAppPlugin;

impl Plugin for MotionMatchingAppPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                MainSet::Action,
                MainSet::Record,
                MainSet::Trajectory,
                MainSet::MotionMatching,
                MainSet::Animation,
            )
                .chain(),
        );

        app.add_plugins((
            DefaultPlugins.set(AssetPlugin {
                mode: AssetMode::Processed,
                ..default()
            }),
            transform2d::Transform2dPlugin,
            record::RecordPlugin::<Transform2d>::default(),
            record::RecordPlugin::<Velocity>::default(),
            trajectory::TrajectoryPlugin,
            action::ActionPlugin,
            motion::MotionPlugin,
            scene_loader::SceneLoaderPlugin,
            bvh_manager::BvhManagerPlugin,
            camera::CameraPlugin,
            ui::UiPlugin,
            player::PlayerPlugin,
            motion_matching::MotionMatchingPlugin,
            draw_axes::DrawAxesPlugin,
        ));

        app.init_state::<GameMode>();
    }
}

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameMode {
    #[default]
    None,
    Config,
    Play,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MainSet {
    Action,
    Record,
    Trajectory,
    MotionMatching,
    Animation,
}
