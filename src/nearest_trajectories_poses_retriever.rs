use bevy::prelude::*;

use crate::{
    motion_data_asset::{MotionDataAsset, Pose},
    player::PlayerMarker,
    // pose_matching::match_pose,
    trajectory::Trajectory,
};

pub struct NearestTrajectoryRetrieverPlugin;

impl Plugin for NearestTrajectoryRetrieverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, match_trajectory);
    }
}

pub fn match_trajectory(
    motion_data_assets: Res<Assets<MotionDataAsset>>,
    query_motion_data: Query<&Handle<MotionDataAsset>>,
    user_input_trajectory: Query<(&Trajectory, &Transform), With<PlayerMarker>>,
) {
    for handle in query_motion_data.iter() {
        if let Some(motion_data) = motion_data_assets.get(handle) {
            for (trajectory, transform) in user_input_trajectory.iter() {
                let nearest_trajectories =
                    find_nearest_trajectories::<10>(motion_data, trajectory, transform);
                println!("10 nearest trajectory: {:?}", nearest_trajectories);
                // let _poses = get_nearest_trajectories_pose(motion_data, nearest_trajectories);
                // match_pose(&motion_data_assets, &query_motion_data, _poses);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NearestTrajectory {
    /// Error distance from this trajectory to the trajecctory that is being compared to.
    pub distance: f32,
    /// Index pointing to the chunk that holds this trajectory.
    pub chunk_index: usize,
    /// Offset index into the chunk that holds this trajectory.
    pub chunk_offset: usize,
}

/// # Panic
///
/// Panic if `N` is 0.
pub fn find_nearest_trajectories<const N: usize>(
    motion_data: &MotionDataAsset,
    player_trajectory: &Trajectory,
    player_transform: &Transform,
) -> [Option<NearestTrajectory>; N] {
    assert!(
        N > 0,
        "Unable to find closest trajectory if the number of closest trajectory needed is 0."
    );

    let player_inv_matrix = player_transform.compute_matrix().inverse();
    let mut stack_count = 0;
    let mut nearest_trajectories_stack = [None::<NearestTrajectory>; N];

    let trajectories = &motion_data.trajectories;
    for (chunk_index, chunk) in trajectories.iter_chunk().enumerate() {
        let chunk_count = chunk.len();
        if chunk_count < 7 {
            warn!("Chunk ({chunk_index}) has less than 7 trajectories.");
            continue;
        }

        for chunk_offset in 0..chunk_count - 7 {
            let trajectory = &chunk[chunk_offset..chunk_offset + 7];

            // Center point of trajectory
            let inv_matrix = trajectory[3].inverse();

            let player_local_translations = player_trajectory
                .values
                .iter()
                .map(|player_trajectory| {
                    player_inv_matrix.transform_point3(Vec3::new(
                        player_trajectory.x,
                        0.0,
                        player_trajectory.y,
                    ))
                })
                .map(|v| v.xz())
                .collect::<Vec<_>>();

            let local_translations = trajectory
                .iter()
                .map(|trajectory| {
                    inv_matrix.transform_point3(trajectory.to_scale_rotation_translation().2)
                })
                .map(|v| v.xz())
                .collect::<Vec<_>>();

            let distance =
                calculate_trajectory_distance(&player_local_translations, &local_translations);

            if stack_count < N {
                // Stack not yet full, push into it
                nearest_trajectories_stack[stack_count] = Some(NearestTrajectory {
                    distance,
                    chunk_index,
                    chunk_offset,
                });
            } else if let Some(max_trajectory) = nearest_trajectories_stack[N - 1] {
                if distance < max_trajectory.distance {
                    nearest_trajectories_stack[N - 1] = Some(NearestTrajectory {
                        distance,
                        chunk_index,
                        chunk_offset,
                    })
                }
            }

            stack_count = usize::min(stack_count + 1, N);

            // Sort so that trajectories with the largest distance
            // is placed as the final element in the stack
            nearest_trajectories_stack.sort_by(|t0, t1| match (t0, t1) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(t0), Some(t1)) => t0.distance.total_cmp(&t1.distance),
            });
        }
    }

    nearest_trajectories_stack
}

pub fn calculate_trajectory_distance(t1: &[Vec2], t2: &[Vec2]) -> f32 {
    // distance = sqrt((p1-q1)^2 + (p2-q2)^2)
    t1.iter()
        .zip(t2.iter())
        .map(|(p, traj)| (*p - *traj).length_squared())
        .sum::<f32>()
}

// pub fn get_nearest_trajectories_pose(
//     motion_data: &MotionDataAsset,
//     nearest_trajectory: Vec<(f32, f32, usize)>,
// ) -> Vec<&Pose> {
//     let mut poses = Vec::new();

//     for (_distance, time, file_index) in nearest_trajectory.iter() {
//         let pose_start_index = motion_data.pose_offsets[*file_index];
//         println!("Pose Start Index {}", pose_start_index);
//         let pose_index = ((pose_start_index as f32) + time / 0.016667) as usize;
//         println!("{pose_index}");

//         println!("Motion Data Poses Total Len: {}", motion_data.poses.len());
//         if let Some(pose) = motion_data.poses.get(pose_index) {
//             poses.push(pose);
//         }
//     }

//     // println!("Pose count: {:?}", poses.len());
//     // println!("Pose count: {:?}", poses);
//     poses
// }
