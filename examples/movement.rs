use bevy::color::palettes::css;
use bevy::prelude::*;

fn main() -> AppExit {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);

    app.add_systems(Update, draw_trajectory);

    app.run()
}

#[derive(Clone, Copy)]
pub struct Transform2d {
    pub translation: Vec2,
    /// Rotation angle in radians.
    pub angle: f32,
}

#[derive(Component, Deref, DerefMut, Clone)]
pub struct Trajectory(Vec<Transform2d>);

fn draw_trajectory(q_trajectory: Query<&Trajectory>, mut gizmos: Gizmos) {
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
// TODO: Figure out axis: Use gizmos to draw out the raw XYZ axis.
