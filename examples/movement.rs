use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_motion_matching::action::*;
use bevy_motion_matching::camera::CameraPlugin;
use bevy_motion_matching::draw_axes::*;
use bevy_motion_matching::player::*;
use bevy_motion_matching::record::*;
use bevy_motion_matching::trajectory::*;
use bevy_motion_matching::transform2d::*;
use bevy_motion_matching::ui::MouseInUi;
use leafwing_input_manager::prelude::*;

fn main() -> AppExit {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        WorldInspectorPlugin::new(),
        CameraPlugin,
        Transform2dPlugin,
        ActionPlugin,
    ))
    .init_resource::<MouseInUi>();

    app.add_plugins((
        DrawAxesPlugin,
        RecordPlugin::<Transform2d>::default(),
        RecordPlugin::<Velocity>::default(),
    ))
    .init_resource::<ColorPalette>()
    .insert_resource(MovementConfig {
        walk_speed: 2.0,
        run_speed: 4.0,
        lerp_factor: 10.0,
    })
    .insert_resource(TrajectoryConfig {
        interval_time: 0.1667,
        predict_count: 6,
        history_count: 5,
    })
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            trajectory_len,
            update_movement_direction,
            (predict_trajectory, trajectory_history),
        )
            .chain(),
    )
    .add_systems(
        Update,
        (movement_test, draw_trajectory_axes, draw_debug_axis),
    )
    .add_systems(Last, (update_velocities, update_prev_transform2ds).chain());

    app.register_type::<Trajectory>()
        .register_type::<PrevTransform2d>()
        .register_type::<Velocity>()
        .register_type::<MovementDirection>();

    app.run()
}

// TODO: Trajectory redo
// - Record stored from start to end.
// - Allows definition for the length of prediction and history. (done)
// - Prediction trajectory depends on the recorded prediction of the primary prediction point.
//
// TODO: New tooling
// - Json file for storing motion matching settings.
//   - Trajectory interval
//   - Trajectory length
// - Inspect trajectories from existing bvh data.
//
// DONE: Figure out axis: Use gizmos to draw out the raw XYZ axis.

// TODO: Remove this
fn movement_test(
    mut q_movements: Query<(&mut Transform2d, &MovementDirection)>,
    movement_config: Res<MovementConfig>,
    time: Res<Time>,
) {
    for (mut transform2d, direction) in q_movements.iter_mut() {
        transform2d.translation += **direction * movement_config.walk_speed * time.delta_seconds();
    }
}

fn predict_trajectory(
    mut q_trajectories: Query<(&mut Trajectory, &Transform2d, &Velocity, &MovementDirection)>,
    trajectory_config: Res<TrajectoryConfig>,
    movement_config: Res<MovementConfig>,
) {
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

            trajectory[i + trajectory_config.history_count] =
                TrajectoryPoint::new(translation, velocity);
        }
    }
}

fn trajectory_history(
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
                record_time += curr_delta_time;
                record_index += 1;

                trans_start = trans_end;
                vel_start = vel_end;
            }

            // Lerp between start and end point.
            let factor = 1.0 - (record_time - target_time) / curr_delta_time;
            trajectory[trajectory_config.history_count - i] = TrajectoryPoint::new(
                Vec2::lerp(trans_start, trans_end, factor),
                Vec2::lerp(vel_start, vel_end, factor),
            );
        }
    }
}

fn update_movement_direction(
    mut q_movement_directions: Query<&mut MovementDirection>,
    movement_config: Res<MovementConfig>,
    action: Res<ActionState<PlayerAction>>,
    time: Res<Time>,
) {
    let mut action_axis = action
        .clamped_axis_pair(&PlayerAction::Walk)
        .map(|axis| axis.xy().normalize_or_zero())
        .unwrap_or_default();
    action_axis.x = -action_axis.x;

    for mut movement_direction in q_movement_directions.iter_mut() {
        **movement_direction = Vec2::lerp(
            **movement_direction,
            action_axis,
            f32::min(1.0, movement_config.lerp_factor * time.delta_seconds()),
        );
    }
}

fn trajectory_len(
    mut q_trajectories: Query<&mut Trajectory>,
    trajectory_config: Res<TrajectoryConfig>,
) {
    // Add one for the current transform
    let target_len = 1 + trajectory_config.history_count + trajectory_config.predict_count;

    for mut trajectory in q_trajectories.iter_mut() {
        if trajectory.len() != target_len {
            **trajectory = vec![TrajectoryPoint::default(); target_len];
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
                palette.purple.mix(
                    &palette.orange,
                    velocity_magnitude / movement_config.run_speed,
                ),
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Cuboid::from_size(Vec3::splat(0.1))),
            material: materials.add(Color::WHITE),
            ..default()
        },
        RecordsBundle::<Transform2d>::new(100),
        RecordsBundle::<Velocity>::new(100),
        TrajectoryBundle::default(),
    ));
}

/// Debug XYZ axis in world space.
fn draw_debug_axis(mut axes: ResMut<DrawAxes>) {
    axes.draw(Mat4::IDENTITY, 1.0);
}

#[derive(Bundle, Default)]
pub struct TrajectoryBundle {
    pub trajectory: Trajectory,
    pub transform2d: Transform2d,
    pub prev_transform2d: PrevTransform2d,
    pub velocity: Velocity,
    pub movement_direction: MovementDirection,
}

/// Trajectory containing prediction and history based on [`TrajectoryConfig`].
#[derive(Component, Reflect, Default, Debug, Deref, DerefMut)]
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
    translation: Vec2,
    velocity: Vec2,
}

impl TrajectoryPoint {
    pub fn new(translation: Vec2, velocity: Vec2) -> Self {
        Self {
            translation,
            velocity,
        }
    }
}
