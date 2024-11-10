use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::PlayerAction;
use crate::draw_axes::{ColorPalette, DrawAxes};
use crate::trajectory::MovementDirection;
use crate::transform2d::Transform2d;
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
            (movement_direction, draw_player_direction)
                .chain()
                .in_set(MainSet::Action),
        );
    }
}

fn movement_direction(
    mut q_movement_directions: Query<(&mut MovementDirection, &Transform2d)>,
    movement_config: Res<MovementConfig>,
    action: Res<ActionState<PlayerAction>>,
    time: Res<Time>,
) {
    let mut action_axis = action
        .clamped_axis_pair(&PlayerAction::Walk)
        .map(|axis| axis.xy().normalize_or_zero())
        .unwrap_or_default();
    action_axis.x = -action_axis.x;

    for (mut movement_direction, transform2d) in q_movement_directions.iter_mut() {
        let mut target_direction = Vec2::ZERO;
        target_direction += transform2d.forward() * action_axis.y;
        target_direction += transform2d.right() * action_axis.x;

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
            0.5,
            palette.green,
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
