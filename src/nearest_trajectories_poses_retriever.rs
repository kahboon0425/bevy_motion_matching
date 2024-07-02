use bevy::prelude::*;

use crate::{
    motion_database::{MotionDataAsset, Pose},
    player::PlayerMarker,
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
                let nearest_trajectory =
                    find_closest_trajectory(motion_data, trajectory, transform);
                println!("10 nearest trajectory: {:?}", nearest_trajectory);
                let _poses = get_nearest_trajectories_pose(motion_data, nearest_trajectory);
            }
        }
    }
}

pub fn find_closest_trajectory(
    motion_data: &MotionDataAsset,
    user_trajectory: &Trajectory,
    transform: &Transform,
) -> Vec<(f32, f32, usize)> {
    let mut nearest_trajectories = Vec::new();

    let user_inverse_matrix = transform.compute_matrix().inverse();

    for (i, &end_offset) in motion_data.trajectory_offsets.iter().enumerate().skip(1) {
        let file_index = i - 1;
        let start_offset = motion_data.trajectory_offsets[file_index];

        let available_len = end_offset - start_offset;
        println!("Available Length: {}", available_len);

        if available_len < 7 {
            let mut extended_trajectories = Vec::new();
            let available_trajectories = &motion_data.trajectories[start_offset..end_offset];
            let mut index = 0;
            while extended_trajectories.len() < 7 {
                extended_trajectories.push(available_trajectories[index % available_len]);
                index += 1;
            }

            let inv_matrix = extended_trajectories[3].transform_matrix.inverse();

            let user_local_translations = user_trajectory
                .values
                .iter()
                .map(|user_trajectory| {
                    user_inverse_matrix.transform_point3(Vec3::new(
                        user_trajectory.x,
                        0.0,
                        user_trajectory.y,
                    ))
                })
                .map(|v| v.xz())
                .collect::<Vec<_>>();

            let local_translations = extended_trajectories
                .iter()
                .map(|trajectory| {
                    inv_matrix.transform_point3(
                        trajectory
                            .transform_matrix
                            .to_scale_rotation_translation()
                            .2,
                    )
                })
                .map(|v| v.xz())
                .collect::<Vec<_>>();

            let distance =
                calculate_trajectory_distance(&user_local_translations, &local_translations);

            nearest_trajectories.push((distance, extended_trajectories[0].time, file_index));
        } else {
            for traj_index in start_offset..(end_offset - 7) {
                let trajectories = &motion_data.trajectories[traj_index..traj_index + 7];

                // Center point of trajectory
                let inv_matrix = trajectories[3].transform_matrix.inverse();

                let user_local_translations = user_trajectory
                    .values
                    .iter()
                    .map(|user_trajectory| {
                        user_inverse_matrix.transform_point3(Vec3::new(
                            user_trajectory.x,
                            0.0,
                            user_trajectory.y,
                        ))
                    })
                    .map(|v| v.xz())
                    .collect::<Vec<_>>();

                let local_translations = trajectories
                    .iter()
                    .map(|trajectory| {
                        inv_matrix.transform_point3(
                            trajectory
                                .transform_matrix
                                .to_scale_rotation_translation()
                                .2,
                        )
                    })
                    .map(|v| v.xz())
                    .collect::<Vec<_>>();

                let distance =
                    calculate_trajectory_distance(&user_local_translations, &local_translations);

                nearest_trajectories.push((
                    distance,
                    motion_data.trajectories[traj_index].time,
                    file_index,
                ));
            }
        }
    }

    nearest_trajectories.sort_by(|a, b| a.0.total_cmp(&b.0));

    if nearest_trajectories.len() > 10 {
        nearest_trajectories.truncate(10)
    }

    nearest_trajectories
}

pub fn calculate_trajectory_distance(t1: &[Vec2], t2: &[Vec2]) -> f32 {
    // distance = sqrt((p1-q1)^2 + (p2-q2)^2)
    t1.iter()
        .zip(t2.iter())
        .map(|(p, traj)| (*p - *traj).length_squared())
        .sum::<f32>()
}

pub fn get_nearest_trajectories_pose(
    motion_data: &MotionDataAsset,
    nearest_trajectory: Vec<(f32, f32, usize)>,
) -> Vec<&Pose> {
    let mut poses = Vec::new();

    for (_distance, time, file_index) in nearest_trajectory.iter() {
        let pose_start_index = motion_data.pose_offsets[*file_index];
        println!("Pose Start Index {}", pose_start_index);
        let pose_index = ((pose_start_index as f32) + time / 0.016667) as usize;
        println!("{pose_index}");

        println!("Motion Data Poses Total Len: {}", motion_data.poses.len());
        if let Some(pose) = motion_data.poses.get(pose_index) {
            poses.push(pose);
        }
    }

    // println!("Pose count: {:?}", poses.len());
    // println!("Pose count: {:?}", poses);
    poses
}
