use std::collections::VecDeque;

use bevy::prelude::*;

use crate::ui::ShowDrawArrow;

pub struct InputTrajectoryPlugin;

impl Plugin for InputTrajectoryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrajectoryConfig {
            time_length: 1.0,
            count: 3,
        })
        .insert_resource(TrajectoryHistoryConfig { interval: 0.033 })
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
    time_length: f32,
    count: usize,
}

impl TrajectoryConfig {
    pub fn interval(&self) -> f32 {
        self.time_length / self.count as f32
    }
}

#[derive(Resource)]
pub struct TrajectoryHistoryConfig {
    interval: f32,
}

/// Translations that stores the trajectory history.
#[derive(Component, Default, Clone)]
pub struct TrajectoryHistory {
    current: Vec2,
    histories: VecDeque<Vec2>,
}

/// Final trajectory following the [`TrajectoryConfig`] used for matching.
#[derive(Component, Default, Clone)]
pub struct Trajectory {
    pub values: Vec<Vec2>,
}

#[derive(Component)]
pub struct PlayerMarker;

fn setup_input_trajectory(mut commands: Commands) {
    commands
        .spawn((TrajectoryHistory::default(), SpatialBundle::default()))
        .insert(PlayerMarker);
}

fn update_trajectory_data_len(
    mut q_history: Query<&mut TrajectoryHistory, With<PlayerMarker>>,
    config: Res<TrajectoryConfig>,
    history_config: Res<TrajectoryHistoryConfig>,
) {
    let target_len = f32::ceil(config.time_length / history_config.interval) as usize;

    for mut trajectory in q_history.iter_mut() {
        match trajectory.histories.len().cmp(&target_len) {
            std::cmp::Ordering::Less => {
                let push_count = target_len - trajectory.histories.len();
                let back_trajectory = trajectory.histories.back().copied().unwrap_or_default();

                for _ in 0..push_count {
                    trajectory.histories.push_back(back_trajectory);
                }
            }
            std::cmp::Ordering::Greater => {
                let pop_count = trajectory.histories.len() - target_len;
                for _ in 0..pop_count {
                    trajectory.histories.pop_back();
                }
            }
            std::cmp::Ordering::Equal => {}
        }
    }
}

/// Update the trajectory every interval.
fn update_trajectory(
    mut q_history: Query<(&mut TrajectoryHistory, &Transform), With<PlayerMarker>>,
    history_config: Res<TrajectoryHistoryConfig>,
    time: Res<Time>,
    mut time_passed: Local<f32>,
) {
    *time_passed += time.delta_seconds();

    if *time_passed >= history_config.interval {
        // Updates the histories and current trajectory
        for (mut trajectory, transform) in q_history.iter_mut() {
            trajectory.histories.pop_back();
            trajectory.histories.push_front(transform.translation.xz());
        }

        // Resets timer
        *time_passed = 0.0;
    }
}

fn draw_trajectory(
    q_history: Query<&TrajectoryHistory>,
    mut gizmos: Gizmos,
    show_arrow: Res<ShowDrawArrow>,
) {
    if show_arrow.show {
        for trajectory in q_history.iter() {
            // Draw arrow gizmos of the smoothed out trajectory
            let mut trajectory_iter = trajectory.histories.iter();
            let next = trajectory_iter.next();

            if let Some(next) = next {
                let mut end = *next;

                for next in trajectory_iter {
                    let start = *next;

                    let arrow_start = Vec3::new(start.x, 0.0, start.y);
                    let arrow_end = Vec3::new(end.x, 0.0, end.y);
                    gizmos.arrow(arrow_start, arrow_end, Color::RED);
                    end = start;
                }
            }
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

    if key_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        direction.y += 1.0;
    }
    if key_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        direction.y -= 1.0;
    }
    if key_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        direction.x += 1.0;
    }
    if key_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        direction.x -= 1.0;
    }

    direction = Vec2::normalize_or_zero(direction);
    direction *= time.delta_seconds() * SPEED;
    for mut transform in q_player.iter_mut() {
        transform.translation.x += direction.x;
        transform.translation.z -= direction.y;
    }
}
