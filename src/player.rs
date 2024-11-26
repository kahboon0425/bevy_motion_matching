use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::PlayerAction;
use crate::draw_axes::{ColorPalette, DrawAxes};
use crate::trajectory::MovementDirection;
use crate::transform2d::Transform2d;
use crate::ui::play_mode::RunPresetDirection;
use crate::MainSet;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MovementConfig {
            walk_speed: 2.0,
            run_speed: 4.0,
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
        );
    }
}

fn preset_movement_direction(
    mut q_movement_directions: Query<&mut MovementDirection>,
    time: Res<Time>,
    movement_config: Res<MovementConfig>,
    mut state: Local<(usize, f32)>,
    run_preset_direction: Res<RunPresetDirection>,
) {
    if **run_preset_direction == false {
        return;
    }

    let directions = [
        // Up
        Vec2::new(0.0, 1.0),
        // Right
        Vec2::new(1.0, 0.0),
        // Down
        Vec2::new(0.0, -1.0),
        // Left
        Vec2::new(-1.0, 0.0),
    ];

    let direction_durations = [6.0, 5.0, 5.0, 5.0];

    let (current_direction, elapsed_time) = *state;

    let new_elapsed_time = elapsed_time + time.delta_seconds();

    let current_direction_duration = direction_durations[current_direction];

    let mut new_direction = current_direction;
    let mut reset_time = new_elapsed_time;

    if new_elapsed_time >= current_direction_duration {
        new_direction = (current_direction + 1) % directions.len();
        reset_time = 0.0;
    }

    *state = (new_direction, reset_time);

    let direction = directions[new_direction];
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
