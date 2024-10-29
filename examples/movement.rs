use std::collections::VecDeque;
use std::marker::PhantomData;

use bevy::prelude::*;
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
        CameraPlugin,
        Transform2dPlugin,
        ActionPlugin,
    ))
    .init_resource::<MouseInUi>();

    app.add_plugins((
        Transform2dRecordPlugin::<Transform2d>::default(),
        Transform2dRecordPlugin::<Transform2dPrediction>::default(),
    ))
    .init_resource::<ColorPalette>()
    .init_resource::<MovementConfig>()
    .insert_resource(TrajectoryConfig {
        interval_time: 0.1667,
        predict_count: 5,
        history_count: 1,
    })
    .add_systems(Startup, setup)
    .add_systems(Update, draw_debug_axis)
    .add_systems(Update, draw_trajectory);

    app.run()
}

fn draw_trajectory() {}

/// Debug XYZ axis in world space.
fn draw_debug_axis(mut gizmos: Gizmos, palette: Res<ColorPalette>) {
    gizmos.arrow(Vec3::ZERO, Vec3::X, palette.red);
    gizmos.arrow(Vec3::ZERO, Vec3::Y, palette.green);
    gizmos.arrow(Vec3::ZERO, Vec3::Z, palette.blue);
}

// TODO: Trajectory redo
// - Record stored from start to end.
// - Allows definition for the length of prediction and history.
// - Prediction trajectory depends on the recorded prediction of the primary prediction point.
//
// TODO: New tooling
// - Json file for storing motion matching settings.
//   - Trajectory interval
//   - Trajectory length
// - Inspect trajectories from existing bvh data.
//
// DONE: Figure out axis: Use gizmos to draw out the raw XYZ axis.

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Cuboid::from_size(Vec3::splat(1.0))),
            material: materials.add(Color::WHITE),
            ..default()
        },
        Transform2dRecordsBundle::<Transform2d>::new(20),
        Transform2dRecordsBundle::<Transform2dPrediction>::new(20),
    ));
}

/// The predicted [`Transform2d`] based on [`PlayerAction`].
#[derive(Component, Clone, Copy, Default, Debug, Deref, DerefMut)]
pub struct Transform2dPrediction(Transform2d);

impl Transform2dComp for Transform2dPrediction {
    fn transform2d(&self) -> Transform2d {
        **self
    }
}

#[derive(Default)]
pub struct Transform2dRecordPlugin<T: Transform2dComp>(PhantomData<T>);

impl<T: Transform2dComp> Plugin for Transform2dRecordPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (transform2d_record_size::<T>, transform2d_record::<T>).chain(),
        );
    }
}

/// Push in a new [`Transform2dComp`] to the front of a [`Transform2dRecord`] while popping out an old one.
fn transform2d_record<T: Transform2dComp>(
    mut q_transform2d_records: Query<(&T, &mut Transform2dRecords<T>)>,
    time: Res<Time>,
) {
    for (&transform2d, mut record) in q_transform2d_records.iter_mut() {
        record.pop_back();
        record.push_front(Transform2dRecord {
            transform2d,
            delta_time: time.delta_seconds(),
        });
    }
}

/// Update size of [`Transform2dRecord`] if there are changes to [`Transform2dRecordSize`].
fn transform2d_record_size<T: Transform2dComp>(
    mut q_transform2d_records: Query<
        (&Transform2dRecordLen<T>, &mut Transform2dRecords<T>),
        Changed<Transform2dRecordLen<T>>,
    >,
) {
    for (len, mut records) in q_transform2d_records.iter_mut() {
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

#[derive(Bundle)]
pub struct Transform2dRecordsBundle<T: Transform2dComp> {
    pub transform2d: T,
    pub records: Transform2dRecords<T>,
    pub len: Transform2dRecordLen<T>,
}

impl<T: Transform2dComp> Transform2dRecordsBundle<T> {
    pub fn new(len: usize) -> Self {
        Self {
            transform2d: T::default(),
            records: Transform2dRecords::default(),
            len: Transform2dRecordLen::new(len),
        }
    }
}

/// A history record of the target [`Transform2dComp`] component.
#[derive(Component, Default, Debug, Deref, DerefMut, Clone)]
pub struct Transform2dRecords<T: Transform2dComp>(VecDeque<Transform2dRecord<T>>);

#[derive(Default, Debug, Clone, Copy)]
pub struct Transform2dRecord<T: Transform2dComp> {
    /// The recorded transform data.
    pub transform2d: T,
    /// The time between the previous frame and the frame where the transform is recorded.
    pub delta_time: f32,
}

/// Determines the size of [`Transform2dRecord`].
#[derive(Component, Clone, Copy, Default, Debug, Deref, DerefMut)]
pub struct Transform2dRecordLen<T: Transform2dComp>(#[deref] usize, PhantomData<T>);

impl<T: Transform2dComp> Transform2dRecordLen<T> {
    pub fn new(len: usize) -> Self {
        Self(len, PhantomData)
    }
}

pub trait Transform2dComp: Component + Default + Copy + Clone {
    fn transform2d(&self) -> Transform2d;
}

impl Transform2dComp for Transform2d {
    fn transform2d(&self) -> Transform2d {
        *self
    }
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
