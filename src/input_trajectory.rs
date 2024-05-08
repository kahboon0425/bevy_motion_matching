use bevy::prelude::*;

pub struct InputTrajectoryPlugin;

impl Plugin for InputTrajectoryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrajectoryConfig::new(20, 0.033))
            .add_systems(Startup, setup_input_trajectory)
            .add_systems(
                Update,
                (
                    (
                        update_trajectory_data_len.run_if(resource_changed::<TrajectoryConfig>),
                        update_trajectory,
                        draw_trajectory,
                    )
                        .chain(),
                    update_player_translation,
                ),
            );
    }
}

/// Configuration for all trajectories.
#[derive(Resource)]
pub struct TrajectoryConfig {
    count: usize,
    interval: f32,
}

impl TrajectoryConfig {
    pub fn new(count: usize, interval: f32) -> Self {
        Self { count, interval }
    }
}

/// Translations that defines the trajectory.
#[derive(Component, Default, Clone)]
pub struct Trajectory {
    current: Vec2,
    histories: Vec<Vec2>,
    predictions: Vec<Vec2>,
}

#[derive(Component)]
pub struct PlayerMarker;

fn setup_input_trajectory(mut commands: Commands) {
    commands
        .spawn((Trajectory::default(), SpatialBundle::default()))
        .insert(PlayerMarker);
}

fn update_trajectory_data_len(
    mut q_trajectory: Query<&mut Trajectory, With<PlayerMarker>>,
    trajectory_config: Res<TrajectoryConfig>,
) {
    for mut trajectory in q_trajectory.iter_mut() {
        if trajectory.histories.len() != trajectory_config.count {
            trajectory.histories = vec![trajectory.current; trajectory_config.count];
        }
        if trajectory.predictions.len() != trajectory_config.count {
            trajectory.predictions = vec![trajectory.current; trajectory_config.count];
        }
    }
}

/// Update the trajectory every interval.
fn update_trajectory(
    mut q_trajectory: Query<(&mut Trajectory, &Transform), With<PlayerMarker>>,
    trajectory_config: Res<TrajectoryConfig>,
    time: Res<Time>,
    mut time_passed: Local<f32>,
) {
    *time_passed += time.delta_seconds();

    if *time_passed >= trajectory_config.interval {
        // Updates the histories and current trajectory
        for (mut trajectory, transform) in q_trajectory.iter_mut() {
            let mut translation = transform.translation.xz();

            std::mem::swap(&mut trajectory.current, &mut translation);
            for history in trajectory.histories.iter_mut() {
                std::mem::swap(history, &mut translation)
            }
        }

        // Resets timer
        *time_passed = 0.0;
    }
}

fn draw_trajectory(q_trajectory: Query<&Trajectory>, mut gizmos: Gizmos) {
    for trajectory in q_trajectory.iter() {
        // Draw arrow gizmos of the smoothed out trajectory
        let mut end = trajectory.current;

        for history in trajectory.histories.iter() {
            let start = *history;

            let arrow_start = Vec3::new(start.x, 0.0, start.y);
            let arrow_end = Vec3::new(end.x, 0.0, end.y);
            gizmos.arrow(arrow_start, arrow_end, Color::YELLOW);
            end = start;
        }
    }
}

fn update_player_translation(
    mut q_player: Query<&mut Transform, With<PlayerMarker>>,
    key_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    const SPEED: f32 = 2.0;
    let mut direction = Vec2::ZERO;

    if key_input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if key_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if key_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    if key_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }

    direction = Vec2::normalize_or_zero(direction);
    direction *= time.delta_seconds() * SPEED;
    for mut transform in q_player.iter_mut() {
        transform.translation.x += direction.x;
        transform.translation.z += direction.y;
    }
}
