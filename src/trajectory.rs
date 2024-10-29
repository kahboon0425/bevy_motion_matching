use std::collections::VecDeque;

use bevy::{color::palettes::css, prelude::*};

use crate::{
    player::{DesiredDirection, MovementSpeed, PlayerMarker},
    transform2d::Transform2d,
    ui::config::DrawTrajectory,
};

pub struct TrajectoryPlugin;

impl Plugin for TrajectoryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrajectoryConfig {
            interval_time: 0.1667,
            predict_count: 5,
            history_count: 1,
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

fn change_trajectory_history_len(
    mut q_history: Query<&mut TrajectoryHistory, With<PlayerMarker>>,
    config: Res<TrajectoryConfig>,
    history_config: Res<TrajectoryHistoryConfig>,
) {
    let target_len = f32::ceil(config.total_history_time() / history_config.interval) as usize;

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
            &DesiredDirection,
            &MovementSpeed,
        ),
        With<PlayerMarker>,
    >,
    config: Res<TrajectoryConfig>,
    history_config: Res<TrajectoryHistoryConfig>,
) {
    for (mut trajectory, trajectory_history, direction, speed) in q_trajectory.iter_mut() {
        let traj_len = config.predict_count + config.history_count + 1;
        if trajectory.len() != traj_len {
            **trajectory = vec![default(); traj_len];
        }

        // Populate current & history
        for c in 0..=config.history_count {
            // Percentage factor to the history
            let offset_factor = c as f32 / config.predict_count as f32;
            // Time offset into the history
            let time_offset = offset_factor * config.interval_time;
            // Starting index offset
            let index_offset_f = time_offset / history_config.interval;
            let start_index_offset = index_offset_f as usize;
            // Subtract 1 because we are going backwards (at least be 0)
            let end_index_offset = usize::max(start_index_offset, 1) - 1;
            let factor = index_offset_f - start_index_offset as f32;

            let start_translation = trajectory_history.histories[start_index_offset];
            let end_translation = trajectory_history.histories[end_index_offset];
            let translation = Vec2::lerp(start_translation, end_translation, factor);

            trajectory[config.predict_count - c].translation = translation;
        }

        // Populate prediction
        let current_translation = trajectory[config.predict_count].translation;
        for c in 1..=config.predict_count {
            let prediction = direction.get() * speed.get() * c as f32 * config.interval_time;
            trajectory[c + config.history_count].translation = current_translation + prediction;
        }
    }
}

fn draw_trajectory(
    q_trajectory: Query<&Trajectory>,
    mut gizmos: Gizmos,
    show_arrow: Res<DrawTrajectory>,
) {
    if **show_arrow {
        for trajectory in q_trajectory.iter() {
            // Draw arrow gizmos of the smoothed out trajectory
            let mut trajectory_iter = trajectory.iter();
            let next = trajectory_iter.next();

            if let Some(next) = next {
                let mut start = next.translation;

                for next in trajectory_iter {
                    let end = next.translation;

                    let arrow_start = Vec3::new(start.x, 0.0, start.y);
                    let arrow_end = Vec3::new(end.x, 0.0, end.y);
                    gizmos.arrow(arrow_start, arrow_end, css::RED);
                    start = end;
                }
            }
        }
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
    pub interval_time: f32,
    pub predict_count: usize,
    pub history_count: usize,
}

impl TrajectoryConfig {
    pub fn total_prediction_time(&self) -> f32 {
        self.interval_time * self.predict_count as f32
    }

    pub fn total_history_time(&self) -> f32 {
        self.interval_time * self.history_count as f32
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
#[derive(Component, Default, Clone, Deref, DerefMut)]
pub struct Trajectory(Vec<Transform2d>);
