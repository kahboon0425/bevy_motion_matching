use std::collections::VecDeque;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_motion_matching::action::*;
use bevy_motion_matching::camera::CameraPlugin;
use bevy_motion_matching::player::*;
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
        history_count: 1,
    })
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            trajectory_len,
            update_movement_direction,
            predict_trajectory,
        )
            .chain(),
    )
    .add_systems(Update, (draw_trajectory_axes, draw_debug_axis))
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

fn predict_trajectory(
    mut q_trajectories: Query<(&mut Trajectory, &Transform2d, &Velocity, &MovementDirection)>,
    trajectory_config: Res<TrajectoryConfig>,
    movement_config: Res<MovementConfig>,
) {
    for (mut trajectory, transform2d, velocity, direction) in q_trajectories.iter_mut() {
        // Predict trajectory.
        let mut translation = transform2d.translation;
        let mut velocity = **velocity;

        let velocity_addition =
            **direction * movement_config.walk_speed * trajectory_config.interval_time;

        for i in 0..trajectory_config.predict_count {
            velocity += velocity_addition;
            translation += velocity * trajectory_config.interval_time;
            // Accelerate to walk speed max.
            velocity = Vec2::clamp_length(velocity, 0.0, movement_config.walk_speed);

            trajectory[i + trajectory_config.history_count] =
                TrajectoryPoint::new(translation, velocity);
        }
    }
}

fn trajectory_history(
    mut q_trajectories: Query<(&mut Trajectory, &Records<Transform2d>, &Records<Velocity>)>,
    trajectory_config: Res<TrajectoryConfig>,
) {
    for (mut trajectory, transform_record, velocity_record) in q_trajectories.iter_mut() {}
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
    action_axis.y = -action_axis.y;

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

fn draw_trajectory_axes(q_trajectories: Query<&Trajectory>, mut axes: ResMut<DrawAxes>) {
    for trajectory in q_trajectories.iter() {
        for point in trajectory.iter() {
            let angle = f32::atan2(point.velocity.x, point.velocity.y);
            let translation = Vec3::new(point.translation.x, 0.0, point.translation.y);

            axes.draw(
                Mat4::from_rotation_translation(Quat::from_rotation_y(angle), translation),
                0.1,
            );
        }
    }
}

fn update_velocities(
    mut q_velocities: Query<(&mut Velocity, &PrevTransform2d, &Transform2d)>,
    time: Res<Time>,
) {
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
        RecordsBundle::<Transform2d>::new(20),
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

// ==================================================================================================================

#[derive(Default)]
pub struct RecordPlugin<T: Recordable>(PhantomData<T>);

impl<T: Recordable> Plugin for RecordPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                record_len::<T>,
                record::<T>,
                // draw_transform2d_record_axes,
            )
                .chain(),
        );
    }
}

/// Push in a new [`Transform2dComp`] to the front of a [`Transform2dRecord`] while popping out an old one.
fn record<T: Recordable>(mut q_records: Query<(&T, &mut Records<T>)>, time: Res<Time>) {
    for (&value, mut record) in q_records.iter_mut() {
        record.pop_back();
        record.push_front(Record {
            value,
            delta_time: time.delta_seconds(),
        });
    }
}

/// Update size of [`Record`] if there are changes to [`RecordLen`].
fn record_len<T: Recordable>(
    mut q_records: Query<(&RecordLen<T>, &mut Records<T>), Changed<RecordLen<T>>>,
) {
    for (len, mut records) in q_records.iter_mut() {
        let target_len = **len;
        match records.len().cmp(&target_len) {
            std::cmp::Ordering::Less => {
                let push_count = target_len - records.len();
                let back_trajectory = records.back().copied().unwrap_or_default();

                for _ in 0..push_count {
                    records.push_back(back_trajectory);
                }
            }
            std::cmp::Ordering::Greater => {
                let pop_count = records.len() - target_len;
                for _ in 0..pop_count {
                    records.pop_back();
                }
            }
            std::cmp::Ordering::Equal => {}
        }
    }
}

// fn draw_transform2d_record_axes(
//     q_transform2d_records: Query<&Records<Transform2d>>,
//     mut axes: ResMut<DrawAxes>,
//     palette: Res<ColorPalette>,
// ) {
//     for records in q_transform2d_records.iter() {
//         for record in records.iter() {
//             let transform2d = record.value;
//             let angle = f32::atan2(transform2d.direction.x, transform2d.direction.y);
//             let translation = Vec3::new(transform2d.translation.x, 0.0, transform2d.translation.y);

//             axes.draw_with_color(
//                 Mat4::from_rotation_translation(Quat::from_rotation_y(angle), translation),
//                 0.1,
//                 palette.orange,
//             );
//         }
//     }
// }

#[derive(Bundle)]
pub struct RecordsBundle<T: Recordable> {
    pub records: Records<T>,
    pub len: RecordLen<T>,
}

impl<T: Recordable> RecordsBundle<T> {
    pub fn new(len: usize) -> Self {
        Self {
            records: Records::default(),
            len: RecordLen::new(len),
        }
    }
}

/// A history record of the target [`Transform2dComp`] component.
#[derive(Component, Default, Debug, Deref, DerefMut, Clone)]
pub struct Records<T: Recordable>(VecDeque<Record<T>>);

#[derive(Default, Debug, Clone, Copy)]
pub struct Record<T: Recordable> {
    /// The recorded value.
    pub value: T,
    /// The time between the previous frame and the frame where the transform is recorded.
    pub delta_time: f32,
}

/// Determines the size of [`Transform2dRecord`].
#[derive(Component, Default, Debug, Deref, DerefMut, Clone, Copy)]
pub struct RecordLen<T: Recordable>(#[deref] usize, PhantomData<T>);

impl<T: Recordable> RecordLen<T> {
    fn new(len: usize) -> Self {
        Self(len, PhantomData)
    }
}

pub trait Recordable: Component + Default + Debug + Clone + Copy {}

impl<T> Recordable for T where T: Component + Default + Debug + Clone + Copy {}

// ==================================================================================================================

pub struct DrawAxesPlugin;

impl Plugin for DrawAxesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawAxes>()
            .add_systems(First, clear_axes)
            .add_systems(Last, draw_axes.run_if(resource_changed::<DrawAxes>));
    }
}

fn clear_axes(mut axes: ResMut<DrawAxes>) {
    if axes.is_empty() == false {
        axes.clear();
    }
}

fn draw_axes(mut gizmos: Gizmos, axes: Res<DrawAxes>, palette: Res<ColorPalette>) {
    for axis in axes.iter() {
        let start = axis.mat.transform_point3(Vec3::ZERO);
        gizmos.arrow(
            start,
            start + axis.mat.transform_vector3(Vec3::X) * axis.size,
            axis.color.unwrap_or(palette.red),
        );
        gizmos.arrow(
            start,
            start + axis.mat.transform_vector3(Vec3::Y) * axis.size,
            axis.color.unwrap_or(palette.green),
        );
        gizmos.arrow(
            start,
            start + axis.mat.transform_vector3(Vec3::Z) * axis.size,
            axis.color.unwrap_or(palette.blue),
        );
    }
}

/// Axes to draw.
///
/// Will be cleaned up every frame.
#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub struct DrawAxes(Vec<DrawAxis>);

impl DrawAxes {
    pub fn draw(&mut self, mat: Mat4, size: f32) {
        self.push(DrawAxis {
            mat,
            size,
            color: None,
        });
    }

    pub fn draw_with_color(&mut self, mat: Mat4, size: f32, color: Color) {
        self.push(DrawAxis {
            mat,
            size,
            color: Some(color),
        });
    }
}

/// Matrix and size of the axis to be drawn.
#[derive(Default, Debug, Clone, Copy)]
pub struct DrawAxis {
    pub mat: Mat4,
    pub size: f32,
    pub color: Option<Color>,
}

#[derive(Resource)]
pub struct ColorPalette {
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub blue: Color,
    pub purple: Color,
    pub base0: Color,
    pub base1: Color,
    pub base2: Color,
    pub base3: Color,
    pub base4: Color,
    pub base5: Color,
    pub base6: Color,
    pub base7: Color,
    pub base8: Color,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            red: Color::Srgba(Srgba::hex("#FF6188").unwrap()),
            orange: Color::Srgba(Srgba::hex("#FC9867").unwrap()),
            yellow: Color::Srgba(Srgba::hex("#FFD866").unwrap()),
            green: Color::Srgba(Srgba::hex("#A9DC76").unwrap()),
            blue: Color::Srgba(Srgba::hex("#78DCE8").unwrap()),
            purple: Color::Srgba(Srgba::hex("#AB9DF2").unwrap()),
            base0: Color::Srgba(Srgba::hex("#19181A").unwrap()),
            base1: Color::Srgba(Srgba::hex("#221F22").unwrap()),
            base2: Color::Srgba(Srgba::hex("#2D2A2E").unwrap()),
            base3: Color::Srgba(Srgba::hex("#403E41").unwrap()),
            base4: Color::Srgba(Srgba::hex("#5B595C").unwrap()),
            base5: Color::Srgba(Srgba::hex("#727072").unwrap()),
            base6: Color::Srgba(Srgba::hex("#939293").unwrap()),
            base7: Color::Srgba(Srgba::hex("#C1C0C0").unwrap()),
            base8: Color::Srgba(Srgba::hex("#FCFCFA").unwrap()),
        }
    }
}
