use bevy::prelude::*;

use crate::motion_data::motion_data_player::{MotionDataPlayer, MotionDataPlayerPair};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, input_direction);
    }
}

fn input_direction(
    mut q_player: Query<
        (
            &mut MovementDirection,
            &mut Transform,
            &MovementSpeed,
            &RotationSpeed,
        ),
        With<PlayerMarker>,
    >,
    q_camera: Query<&GlobalTransform, With<Camera>>,
    key_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    motion_player: Res<MotionDataPlayerPair>,
) {
    if motion_player.is_playing == false {
        return;
    }

    let Ok(camera_transform) = q_camera.get_single() else {
        return;
    };

    let mut input_dir = Vec2::ZERO;

    if key_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        input_dir.y -= 1.0;
    }
    if key_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        input_dir.y += 1.0;
    }
    if key_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        input_dir.x += 1.0;
    }
    if key_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        input_dir.x -= 1.0;
    }

    input_dir = input_dir.normalize_or_zero();
    for (mut movement_direction, mut transform, movement_speed, rotation_speed) in
        q_player.iter_mut()
    {
        let direction =
            input_dir.x * transform.left().xz() + input_dir.y * transform.forward().xz();

        *movement_direction = MovementDirection(direction);
        // transform.translation +=
        //     movement_direction.get_vec3() * movement_speed.get() * time.delta_seconds();

        let desired_direction = camera_transform.forward().zx();
        let desired_rotation = Quat::from_rotation_y(desired_direction.to_angle());
        // transform.rotation = Quat::slerp(
        //     transform.rotation,
        //     desired_rotation,
        //     time.delta_seconds() * rotation_speed.get(),
        // );
    }
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub marker: PlayerMarker,
    pub transform: Transform,
    pub movement_speed: MovementSpeed,
    pub rotation_speed: RotationSpeed,
    pub direction: MovementDirection,
}

#[derive(Component, Default)]
pub struct PlayerMarker;

#[derive(Component, Deref)]
pub struct MovementSpeed(f32);

impl MovementSpeed {
    pub fn get(&self) -> f32 {
        self.0
    }
}

impl Default for MovementSpeed {
    fn default() -> Self {
        Self(2.0)
    }
}

#[derive(Component, Deref)]
pub struct RotationSpeed(f32);

impl RotationSpeed {
    pub fn get(&self) -> f32 {
        self.0
    }
}

impl Default for RotationSpeed {
    fn default() -> Self {
        Self(2.0)
    }
}

#[derive(Component, Default, Deref, Debug)]
pub struct MovementDirection(Vec2);

impl MovementDirection {
    pub fn get(&self) -> Vec2 {
        self.0
    }

    pub fn get_vec3(&self) -> Vec3 {
        Vec3::new(self.0.x, 0.0, self.0.y)
    }
}
