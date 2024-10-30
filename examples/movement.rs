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

fn main() -> AppExit {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        WorldInspectorPlugin::new(),
        CameraPlugin,
        Transform2dPlugin,
        ActionPlugin,
        TrajectoryPlugin,
        DrawAxesPlugin,
    ))
    .init_resource::<MouseInUi>();

    app.add_plugins((
        DrawAxesPlugin,
        RecordPlugin::<Transform2d>::default(),
        RecordPlugin::<Velocity>::default(),
    ))
    .insert_resource(MovementConfig {
        walk_speed: 2.0,
        run_speed: 4.0,
        lerp_factor: 10.0,
    })
    .add_systems(Startup, setup)
    .add_systems(Update, (movement_test, draw_debug_axis));

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
