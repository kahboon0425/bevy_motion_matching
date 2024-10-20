use std::collections::VecDeque;

use bevy::{color::palettes::css, prelude::*};

use crate::{
    player::{MovementDirection, MovementSpeed, PlayerMarker},
    ui::config::DrawTrajectory,
};

pub struct InputTrajectory;

impl Plugin for InputTrajectory {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrajectoryConfig {
            time_length: 0.5,
            count: 3,
        })
        .insert_resource(TrajectoryHistoryConfig { interval: 0.01667 })
        .add_systems(
            Update,
            ((
                change_trajectory_history_len.run_if(resource_changed::<TrajectoryHistoryConfig>),
                store_trajectory_history,
                compute_trajectory,
                draw_trajectory,
            )
                .chain(),),
        );
    }
}

#[derive(Bundle, Default)]
pub struct TrajectoryBundle {
    pub trajectory: Trajectory,
    pub history: TrajectoryHistory,
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
    histories: VecDeque<Vec2>,
}

/// Final trajectory following the [`TrajectoryConfig`] used for matching.
#[derive(Component, Default, Clone)]
pub struct Trajectory {
    pub values: Vec<Vec2>,
}

fn change_trajectory_history_len(
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
fn store_trajectory_history(
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

fn compute_trajectory(
    mut q_trajectory: Query<
        (
            &mut Trajectory,
            &TrajectoryHistory,
            &MovementDirection,
            &MovementSpeed,
        ),
        With<PlayerMarker>,
    >,
    config: Res<TrajectoryConfig>,
    history_config: Res<TrajectoryHistoryConfig>,
) {
    for (mut trajectory, trajectory_history, direction, speed) in q_trajectory.iter_mut() {
        if trajectory.values.len() != config.count {
            trajectory.values = vec![default(); config.count * 2 + 1];
        }

        // Populate current & history
        for c in 0..=config.count {
            // Percentage factor to the history
            let offset_factor = c as f32 / config.count as f32;
            // Time offset into the history
            let time_offset = offset_factor * config.time_length;
            // Starting index offset
            let index_offset_f = time_offset / history_config.interval;
            let start_index_offset = index_offset_f as usize;
            // Subtract 1 because we are going backwards (at least be 0)
            let end_index_offset = usize::max(start_index_offset, 1) - 1;
            let factor = index_offset_f - start_index_offset as f32;

            let start_translation = trajectory_history.histories[start_index_offset];
            let end_translation = trajectory_history.histories[end_index_offset];
            let translation = Vec2::lerp(start_translation, end_translation, factor);

            trajectory.values[config.count - c] = translation;
        }

        let current_translation = trajectory.values[config.count];
        for c in 1..=config.count {
            let prediction = direction.get() * speed.get() * c as f32 * config.interval();
            trajectory.values[c + config.count] = current_translation + prediction;
        }
    }
}

fn draw_trajectory(
    q_trajectory: Query<&Trajectory>,
    mut gizmos: Gizmos,
    show_arrow: Res<DrawTrajectory>,
) {
    if show_arrow.get() {
        for trajectory in q_trajectory.iter() {
            // Draw arrow gizmos of the smoothed out trajectory
            let mut trajectory_iter = trajectory.values.iter();
            let next = trajectory_iter.next();

            if let Some(next) = next {
                let mut start = *next;

                for next in trajectory_iter {
                    let end = *next;

                    let arrow_start = Vec3::new(start.x, 0.0, start.y);
                    let arrow_end = Vec3::new(end.x, 0.0, end.y);
                    gizmos.arrow(arrow_start, arrow_end, css::RED);
                    start = end;
                }
            }
        }
    }
}
