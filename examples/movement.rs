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
        DrawAxesPlugin,
        TrajectoryPlugin,
        PlayerPlugin,
    ))
    .init_resource::<MouseInUi>();

    app.add_plugins((
        RecordPlugin::<Transform2d>::default(),
        RecordPlugin::<Velocity>::default(),
    ))
    .insert_resource(MovementConfig {
        walk_speed: 1.0,
        run_speed: 2.0,
        lerp_factor: 10.0,
    })
    .insert_resource(TrajectoryConfig {
        interval_time: 0.1667,
        predict_count: 10,
        history_count: 14,
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
// - Inspect trajectories from existing bvh data. (done)
//
// DONE: Figure out axis: Use gizmos to draw out the raw XYZ axis.

// TODO: Remove this
fn movement_test(
    mut q_movements: Query<(&mut Transform2d, &MovementDirection)>,
    movement_config: Res<MovementConfig>,
    time: Res<Time>,
) {
    for (mut transform2d, direction) in q_movements.iter_mut() {
        transform2d.translation += **direction * movement_config.walk_speed * time.delta_secs();
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.1)))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        TrajectoryBundle::new(100),
    ));
}

/// Debug XYZ axis in world space.
fn draw_debug_axis(mut axes: ResMut<DrawAxes>) {
    axes.draw(Mat4::IDENTITY, 1.0);
}
