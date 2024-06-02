use bevy::prelude::*;

use crate::input_trajectory::TrajectoryBundle;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player)
            .add_systems(PreUpdate, input_direction);
    }
}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub marker: PlayerMarker,
    pub transform: Transform,
    pub speed: Speed,
    pub direction: MovementDirection,
}

#[derive(Component, Default)]
pub struct PlayerMarker;

#[derive(Component)]
pub struct Speed(f32);

impl Speed {
    pub fn get(&self) -> f32 {
        self.0
    }
}

impl Default for Speed {
    fn default() -> Self {
        Self(2.0)
    }
}

#[derive(Component, Default)]
pub struct MovementDirection(Vec2);

impl MovementDirection {
    pub fn get(&self) -> Vec2 {
        self.0
    }

    pub fn get_vec3(&self) -> Vec3 {
        Vec3::new(self.0.x, 0.0, self.0.y)
    }
}

fn setup_player(mut commands: Commands) {
    commands.spawn((PlayerBundle::default(), TrajectoryBundle::default()));
}

fn input_direction(
    mut q_player: Query<(&mut MovementDirection, &mut Transform, &Speed), With<PlayerMarker>>,
    key_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
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

    input_dir = Vec2::normalize_or_zero(input_dir);
    for (mut direction, mut transform, speed) in q_player.iter_mut() {
        *direction = MovementDirection(input_dir);

        transform.translation += direction.get_vec3() * speed.get() * time.delta_seconds();
    }
}
