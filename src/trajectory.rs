use bevy::prelude::*;

use crate::draw_axes::{ColorPalette, DrawAxes};
use crate::player::MovementConfig;
use crate::record::{Records, RecordsBundle};
use crate::transform2d::Transform2d;
use crate::MainSet;

pub struct TrajectoryPlugin;

impl Plugin for TrajectoryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrajectoryConfig {
            interval_time: 0.1667,
            predict_count: 5,
            history_count: 1,
        })
        .init_resource::<TrajectoryPlot>()
        .add_systems(
            Update,
            (
                resize_trajectory.run_if(resource_changed::<TrajectoryConfig>),
                (predict_trajectory, current_trajectory, history_trajectory),
            )
                .chain()
                .in_set(MainSet::Trajectory),
        )
        .add_systems(Last, (update_velocities, update_prev_transform2ds).chain())
        .add_systems(Update, (draw_trajectory_axes, draw_trajectory_plot));

        app.register_type::<Trajectory>()
            .register_type::<PrevTransform2d>()
            .register_type::<Velocity>()
            .register_type::<MovementDirection>();
    }
}

fn predict_trajectory(
    mut q_trajectories: Query<(&mut Trajectory, &Transform2d, &Velocity, &MovementDirection)>,
    trajectory_config: Res<TrajectoryConfig>,
    movement_config: Res<MovementConfig>,
) {
    const DAMPING: f32 = 0.9;

    for (mut trajectory, transform2d, velocity, direction) in q_trajectories.iter_mut() {
        // Predict trajectory.
        let mut translation = transform2d.translation;
        let mut velocity = **velocity;

        let velocity_addition = **direction * movement_config.walk_speed;

        for i in 0..trajectory_config.predict_count {
            velocity += velocity_addition * trajectory_config.interval_time;
            translation += velocity * trajectory_config.interval_time;
            // Accelerate to walk speed max.
            velocity = Vec2::clamp_length(velocity, 0.0, movement_config.walk_speed);
            velocity *= DAMPING;

            trajectory[i + trajectory_config.history_count + 1] = TrajectoryPoint {
                translation,
                velocity,
            };
        }
    }
}

fn current_trajectory(
    mut q_trajectories: Query<(&mut Trajectory, &Transform2d, &Velocity)>,
    trajectory_config: Res<TrajectoryConfig>,
) {
    for (mut trajectory, transform2d, velocity) in q_trajectories.iter_mut() {
        trajectory[trajectory_config.history_count] = TrajectoryPoint {
            translation: transform2d.translation,
            velocity: **velocity,
        };
    }
}

fn history_trajectory(
    mut q_trajectories: Query<(
        &mut Trajectory,
        &Transform2d,
        &Velocity,
        &Records<Transform2d>,
        &Records<Velocity>,
    )>,
    trajectory_config: Res<TrajectoryConfig>,
    time: Res<Time>,
) {
    for (mut trajectory, transform2d, velocity, transform_record, velocity_record) in
        q_trajectories.iter_mut()
    {
        assert!(
            transform_record.len() == velocity_record.len(),
            "Records<Transform2d> must have the same length as Records<Velocity>."
        );
        let record_len = transform_record.len();

        // Start and end point to interpolate from.
        let mut trans_start = transform2d.translation;
        let mut vel_start = **velocity;

        let mut trans_end = transform_record[0].value.translation;
        let mut vel_end = *velocity_record[0].value;

        // Accumulate the record time.
        let mut record_time = time.delta_seconds();
        // Keep track of our last used record index
        let mut record_index = 0;
        let mut curr_delta_time = time.delta_seconds();

        for i in 1..=trajectory_config.history_count {
            let target_time = i as f32 * trajectory_config.interval_time;

            let range = record_index..record_len - 1;
            for _ in range {
                trans_end = transform_record[record_index].value.translation;
                vel_end = *velocity_record[record_index].value;

                // Accumulated record time has exceed the target time.
                // Break of before we update the start point.
                if record_time > target_time {
                    break;
                }

                curr_delta_time = transform_record[record_index].delta_time;
                // Accumulate time and index so that we don't loop through
                // previously looped records.
                record_time += curr_delta_time;
                record_index += 1;

                trans_start = trans_end;
                vel_start = vel_end;
            }

            // Lerp between start and end point.
            let factor = 1.0 - (record_time - target_time) / curr_delta_time;
            trajectory[trajectory_config.history_count - i] = TrajectoryPoint {
                translation: Vec2::lerp(trans_start, trans_end, factor),
                velocity: Vec2::lerp(vel_start, vel_end, factor),
            };
        }
    }
}

fn resize_trajectory(
    mut q_trajectories: Query<&mut Trajectory>,
    trajectory_config: Res<TrajectoryConfig>,
) {
    let num_points = trajectory_config.num_points();

    for mut trajectory in q_trajectories.iter_mut() {
        if trajectory.len() != num_points {
            trajectory.resize(num_points, TrajectoryPoint::default());
        }
    }
}

fn draw_trajectory_axes(
    q_trajectories: Query<&Trajectory>,
    mut axes: ResMut<DrawAxes>,
    movement_config: Res<MovementConfig>,
    palette: Res<ColorPalette>,
) {
    for trajectory in q_trajectories.iter() {
        for point in trajectory.iter() {
            let angle = f32::atan2(point.velocity.x, point.velocity.y);
            let translation = Vec3::new(point.translation.x, 0.0, point.translation.y);

            let velocity_magnitude = point.velocity.length();
            axes.draw_forward(
                Mat4::from_rotation_translation(Quat::from_rotation_y(angle), translation),
                velocity_magnitude * 0.1,
                palette
                    .green
                    .mix(&palette.red, velocity_magnitude / movement_config.run_speed),
            );
        }
    }
}

fn update_velocities(
    mut q_velocities: Query<(&mut Velocity, &PrevTransform2d, &Transform2d)>,
    time: Res<Time>,
) {
    // Prevent division by 0
    if time.delta_seconds() < f32::EPSILON {
        return;
    }

    for (mut velocity, prev_transform2d, transform2d) in q_velocities.iter_mut() {
        **velocity =
            (transform2d.translation - prev_transform2d.translation) / time.delta_seconds();
    }
}

fn update_prev_transform2ds(mut q_transform2ds: Query<(&mut PrevTransform2d, &Transform2d)>) {
    for (mut prev_transform2d, transform2d) in q_transform2ds.iter_mut() {
        **prev_transform2d = *transform2d;
    }
}

#[derive(Bundle)]
pub struct TrajectoryBundle {
    pub trajectory: Trajectory,
    pub transform2d: Transform2d,
    pub prev_transform2d: PrevTransform2d,
    pub velocity: Velocity,
    pub movement_direction: MovementDirection,
    pub transform2d_records: RecordsBundle<Transform2d>,
    pub velocity_records: RecordsBundle<Velocity>,
}

impl TrajectoryBundle {
    pub fn new(record_len: usize) -> Self {
        Self {
            trajectory: Trajectory::default(),
            transform2d: Transform2d::default(),
            prev_transform2d: PrevTransform2d::default(),
            velocity: Velocity::default(),
            movement_direction: MovementDirection::default(),
            transform2d_records: RecordsBundle::new(record_len),
            velocity_records: RecordsBundle::new(record_len),
        }
    }
}

/// Trajectory containing prediction and history based on [`TrajectoryConfig`].
#[derive(Component, Reflect, Default, Debug, Deref, DerefMut, Clone)]
#[reflect(Component)]
pub struct Trajectory(Vec<TrajectoryPoint>);

#[derive(Component, Reflect, Default, Debug, Deref, DerefMut, Clone, Copy)]
#[reflect(Component)]
pub struct PrevTransform2d(Transform2d);

#[derive(Component, Reflect, Default, Debug, Deref, DerefMut, Clone, Copy)]
#[reflect(Component)]
pub struct Velocity(Vec2);

#[derive(Component, Reflect, Default, Debug, Deref, DerefMut, Clone, Copy)]
#[reflect(Component)]
pub struct MovementDirection(Vec2);

/// A single point in the [`Trajectory`].
#[derive(Reflect, Default, Debug, Clone, Copy)]
pub struct TrajectoryPoint {
    pub translation: Vec2,
    pub velocity: Vec2,
}

impl TrajectoryPoint {
    pub fn new(translation: Vec2, velocity: Vec2) -> Self {
        Self {
            translation,
            velocity,
        }
    }
}

/// Configuration for all trajectories.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct TrajectoryConfig {
    /// Time between each trajectory point.
    pub interval_time: f32,
    /// Number of prediction points.
    pub predict_count: usize,
    /// Number of history points.
    pub history_count: usize,
}

pub trait TrajectoryDistance {
    fn distance(&self, rhs: &Self) -> f32;
}

impl TrajectoryDistance for [TrajectoryPoint] {
    fn distance(&self, rhs: &Self) -> f32 {
        let len = self.len();
        assert_eq!(len, rhs.len());

        let mut offset_distance = 0.0;

        for i in 1..len {
            let offset0 = self[i].translation - self[i - 1].translation;
            let offset1 = rhs[i].translation - rhs[i - 1].translation;

            offset_distance += Vec2::distance(offset1, offset0);
        }

        let mut velocity_distance = 0.0;

        for i in 0..len {
            velocity_distance += Vec2::distance(self[i].velocity, rhs[i].velocity);
        }

        // Averaging the distances.
        offset_distance /= len.saturating_sub(1) as f32;
        velocity_distance /= len as f32;

        offset_distance + velocity_distance
    }
}

impl TrajectoryConfig {
    /// Duration of the prediction part of the trajectory.
    #[inline]
    pub fn predict_time(&self) -> f32 {
        self.interval_time * self.predict_count as f32
    }

    /// Duration of the history part of the trajectory.
    #[inline]
    pub fn history_time(&self) -> f32 {
        self.interval_time * self.history_count as f32
    }

    /// Number of trajectory segments in a trajectory.
    #[inline]
    pub fn num_segments(&self) -> usize {
        self.predict_count + self.history_count
    }

    /// Number of trajectory points in a trajectory.
    #[inline]
    pub fn num_points(&self) -> usize {
        self.num_segments() + 1
    }

    /// Total duration of the entire trajectory.
    #[inline]
    pub fn total_time(&self) -> f32 {
        self.interval_time * self.num_segments() as f32
    }
}

#[derive(Resource, Debug, Default)]
pub struct TrajectoryPlot {
    pub trajectories_points: Vec<[f64; 2]>,
}

pub fn draw_trajectory_plot(
    mut trajectories_point: ResMut<TrajectoryPlot>,
    user_input_trajectory: Query<(&Trajectory, &Transform)>,
) {
    for (trajectory, transform) in user_input_trajectory.iter() {
        let player_inv_matrix = transform.compute_matrix().inverse();

        let player_local_translations: Vec<_> = trajectory
            .iter()
            .map(|point| {
                player_inv_matrix.transform_point3(Vec3::new(
                    point.translation.x,
                    0.0,
                    point.translation.y,
                ))
            })
            .map(|v| v.xz())
            .collect();

        if let Some(mut start) = player_local_translations.first() {
            trajectories_point.trajectories_points.clear();
            for next in &player_local_translations[1..] {
                trajectories_point
                    .trajectories_points
                    .push([start.x as f64, start.y as f64]);
                start = next;
            }

            trajectories_point
                .trajectories_points
                .push([start.x as f64, start.y as f64]);
        }
    }
}
