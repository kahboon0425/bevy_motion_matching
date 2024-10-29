use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::action::PlayerAction;
use crate::motion_data::motion_data_player::{MotionDataPlayer, MotionDataPlayerPair};
use crate::transform2d::Transform2d;

pub(super) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MovementConfig>()
            .add_systems(PreUpdate, input_direction);
    }
}

fn input_direction(
    mut q_player: Query<
        (&mut DesiredDirection, &mut MovementSpeed, &Transform2d),
        With<PlayerMarker>,
    >,
    q_camera: Query<&Transform, With<Camera>>,
    time: Res<Time>,
    motion_player: Res<MotionDataPlayerPair>,
    movement_config: Res<MovementConfig>,
    action_state: Res<ActionState<PlayerAction>>,
) {
    if motion_player.is_playing == false {
        return;
    }

    let Ok(camera_transform) = q_camera.get_single() else {
        return;
    };

    let Ok((mut movement_direction, mut movement_speed, transform2d)) = q_player.get_single_mut()
    else {
        return;
    };

    let lerp_factor = f32::min(1.0, time.delta_seconds() * movement_config.lerp_factor);

    let is_walking = action_state.pressed(&PlayerAction::Walk);
    let is_running = action_state.pressed(&PlayerAction::Run);

    let mut target_speed = 0.0;
    if is_walking {
        target_speed = match is_running {
            true => movement_config.run_speed,
            false => movement_config.walk_speed,
        };
    }

    **movement_speed = f32::lerp(**movement_speed, target_speed, lerp_factor);

    let input_dir = action_state
        .clamped_axis_pair(&PlayerAction::Walk)
        .map(|axis| axis.xy().normalize_or_zero())
        .unwrap_or_default();

    let direction = input_dir.x * -transform2d.right() + input_dir.y * transform2d.forward();
    *movement_direction = DesiredDirection(direction);
    // println!(
    //     "{}",
    //     movement_direction.get_vec3() * movement_speed.get() * time.delta_seconds()
    // );

    // Create a copy tranfsorm.
    let mut camera_transform = *camera_transform;
    camera_transform.rotate(Quat::from_rotation_y(f32::atan2(input_dir.y, input_dir.x)));
    let camera_forward = camera_transform.forward().zx();
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub marker: PlayerMarker,
    pub transform2d: Transform2d,
    pub movement_speed: MovementSpeed,
    pub direction: DesiredDirection,
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

#[derive(Component, Default, Deref, Debug)]
pub struct DesiredDirection(Vec2);

impl DesiredDirection {
    pub fn get(&self) -> Vec2 {
        self.0
    }

    pub fn get_vec3(&self) -> Vec3 {
        Vec3::new(self.0.x, 0.0, self.0.y)
    }
}

#[derive(Resource, Debug)]
pub struct MovementConfig {
    pub walk_speed: f32,
    pub run_speed: f32,
    pub rotation_speed: f32,
    pub lerp_factor: f32,
}

impl Default for MovementConfig {
    fn default() -> Self {
        Self {
            walk_speed: 2.0,
            run_speed: 4.0,
            rotation_speed: 2.0,
            lerp_factor: 10.0,
        }
    }
}
