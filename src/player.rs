use bevy::prelude::*;
use bevy_bvh_anim::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::PlayerAction;
use crate::bvh_manager::bvh_library::BvhLibrary;
use crate::bvh_manager::bvh_player::{FrameData, JointMap};
use crate::draw_axes::{ColorPalette, DrawAxes};
use crate::motion::motion_player::MotionPlayerBundle;
use crate::scene_loader::MainScene;
use crate::trajectory::MovementDirection;
use crate::transform2d::Transform2d;
use crate::ui::play_mode::RunPresetDirection;
use crate::MainSet;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ResetPlayer>()
            .insert_resource(MovementConfig {
                walk_speed: 2.0,
                run_speed: 2.5,
                lerp_factor: 10.0,
            })
            .add_systems(
                Update,
                (
                    preset_movement_direction,
                    movement_direction,
                    draw_player_direction,
                )
                    .chain()
                    .in_set(MainSet::Action),
            )
            .add_systems(Last, reset_player);
    }
}

fn preset_movement_direction(
    mut q_movement_directions: Query<&mut MovementDirection>,
    time: Res<Time>,
    movement_config: Res<MovementConfig>,
    mut state: Local<(usize, f32)>,
    run_preset_direction: Res<RunPresetDirection>,
) {
    let (current_direction, elapsed_time) = *state;

    if **run_preset_direction == false {
        return;
    }

    const DIRECTIONS: [Vec2; 4] = [
        // Up
        Vec2::new(0.0, 1.0),
        // Right
        Vec2::new(1.0, 0.0),
        // Down
        Vec2::new(0.0, -1.0),
        // Left
        Vec2::new(-1.0, 0.0),
    ];

    const DIRECTION_DURATIONS: [f32; 4] = [6.0, 5.0, 5.0, 5.0];

    let new_elapsed_time = elapsed_time + time.delta_seconds();

    let current_direction_duration = DIRECTION_DURATIONS[current_direction];

    let mut new_direction = current_direction;
    let mut reset_time = new_elapsed_time;

    if new_elapsed_time >= current_direction_duration {
        new_direction = (current_direction + 1) % DIRECTIONS.len();
        reset_time = 0.0;
    }

    *state = (new_direction, reset_time);

    let direction = DIRECTIONS[new_direction];
    for mut movement_direction in q_movement_directions.iter_mut() {
        // **movement_direction = direction;

        **movement_direction = Vec2::lerp(
            **movement_direction,
            direction,
            f32::min(1.0, movement_config.lerp_factor * time.delta_seconds()),
        );
    }
}

fn movement_direction(
    mut q_movement_directions: Query<&mut MovementDirection>,
    movement_config: Res<MovementConfig>,
    action: Res<ActionState<PlayerAction>>,
    time: Res<Time>,
    q_camera: Query<&Transform, With<Camera>>,
    run_preset_direction: Res<RunPresetDirection>,
) {
    if **run_preset_direction {
        return;
    }
    let camera_transform = q_camera.single();
    let mut action_axis = action
        .clamped_axis_pair(&PlayerAction::Walk)
        .map(|axis| axis.xy().normalize_or_zero())
        .unwrap_or_default();
    action_axis.x = -action_axis.x;

    for mut movement_direction in q_movement_directions.iter_mut() {
        let mut target_direction = Vec2::ZERO;
        target_direction += camera_transform.forward().xz().normalize_or_zero() * action_axis.y;
        target_direction += camera_transform.left().xz().normalize_or_zero() * action_axis.x;

        **movement_direction = Vec2::lerp(
            **movement_direction,
            target_direction,
            f32::min(1.0, movement_config.lerp_factor * time.delta_seconds()),
        );
    }
}

fn draw_player_direction(
    q_transform2ds: Query<&Transform2d, With<PlayerMarker>>,
    mut draw_axes: ResMut<DrawAxes>,
    palette: Res<ColorPalette>,
) {
    for transform2d in q_transform2ds.iter() {
        draw_axes.draw_forward(
            Mat4::from_rotation_translation(
                Quat::from_rotation_y(transform2d.angle),
                transform2d.translation3d(),
            ),
            0.3,
            palette.green.with_alpha(0.5),
        );
    }
}

fn reset_player(
    mut commands: Commands,
    bvh_library: Res<BvhLibrary>,
    bvh_assets: Res<Assets<BvhAsset>>,
    mut evr_reset_player: EventReader<ResetPlayer>,
    mut q_transforms: Query<&mut Transform>,
    q_scene: Query<(&JointMap, Entity), With<MainScene>>,
) {
    let Some(map) = bvh_library.get_map().and_then(|bvh| bvh_assets.get(bvh)) else {
        return;
    };

    for _ in evr_reset_player.read() {
        let frame = FrameData(
            map.frames()
                .next()
                .expect("There should be at least one frame in the bvh map."),
        );

        for (joint_map, entity) in q_scene.iter() {
            commands.entity(entity).insert((
                PlayerBundle::default(),
                Transform2d::default(),
                MotionPlayerBundle::default(),
            ));

            for joint in map.joints() {
                let joint_data = joint.data();
                let bone_name = joint_data.name().to_str().unwrap();

                let Some(&bone_entity) = joint_map.get(bone_name) else {
                    continue;
                };
                // Get bone transform
                let Ok(mut transform) = q_transforms.get_mut(bone_entity) else {
                    continue;
                };

                let channels = joint_data.channels();
                let o = joint_data.offset();
                let offset = Vec3::new(o.x, o.y, o.z);

                (transform.translation, transform.rotation) = frame.get_pos_rot(channels);

                if channels.len() == 3 {
                    transform.translation += offset;
                }
            }
        }
    }
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub marker: PlayerMarker,
    pub movement_speed: MovementSpeed,
}

#[derive(Component, Default)]
pub struct PlayerMarker;

#[derive(Component, Default, Deref, DerefMut, Clone, Copy)]
pub struct MovementSpeed(f32);

impl MovementSpeed {
    pub fn get(&self) -> f32 {
        self.0
    }
}

#[derive(Resource, Debug)]
pub struct MovementConfig {
    pub walk_speed: f32,
    pub run_speed: f32,
    pub lerp_factor: f32,
}

#[derive(Event, Default, Clone, Copy)]
pub struct ResetPlayer;
