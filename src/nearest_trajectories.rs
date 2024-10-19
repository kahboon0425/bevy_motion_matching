use bevy::prelude::*;

use crate::bvh_manager::bvh_player::JointMap;
use crate::motion_data::motion_data_asset::MotionDataAsset;
use crate::motion_data::motion_data_player::MotionDataPlayer;
use crate::motion_data::MotionData;
use crate::player::PlayerMarker;
use crate::pose_matching::{apply_pose, match_pose};
use crate::scene_loader::MainScene;
use crate::trajectory::Trajectory;

pub struct NearestTrajectoryRetrieverPlugin;

impl Plugin for NearestTrajectoryRetrieverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_motion_data)
            .add_systems(Update, match_trajectory);
    }
}

pub fn load_motion_data(mut commands: Commands, asset_server: Res<AssetServer>) {
    let file_path = "motion_data/motion_data.json";
    let motion_data = asset_server.load::<MotionDataAsset>(file_path);

    commands.insert_resource(MotionDataPlayer {
        motion_data,
        is_playing: true,
        ..default()
    });
}

pub fn match_trajectory(
    mut motion_data: MotionData,
    user_input_trajectory: Query<(&Trajectory, &Transform), With<PlayerMarker>>,
    mut q_transforms: Query<&mut Transform, (Without<MainScene>, Without<PlayerMarker>)>,
    mut main_character: Query<
        (&mut Transform, &JointMap),
        (With<MainScene>, Without<PlayerMarker>),
    >,
    time: Res<Time>,
    mut time_passed: Local<f32>,
) {
    *time_passed += time.delta_seconds();

    if *time_passed >= 1.0 {
        let Ok((trajectory, transform)) = user_input_trajectory.get_single() else {
            return;
        };

        if let Some(motion_asset) = motion_data.get() {
            let nearest_trajectories =
                find_nearest_trajectories::<1>(motion_asset, trajectory, transform);
            println!("10 nearest trajectory: {:?}", nearest_trajectories);

            let mut smallest_pose_distance = f32::MAX;
            let mut best_pose: Vec<f32> = vec![];
            let mut best_trajectory_index = 0;

            // println!("Nearest Trajectory length: {}", nearest_trajectories.len());
            for (i, nearest_trajectory) in nearest_trajectories.iter().enumerate() {
                if let Some(nearest_trajectory) = nearest_trajectory {
                    let (pose_distance, pose) = match_pose(
                        nearest_trajectory,
                        motion_asset,
                        &mut q_transforms,
                        &mut main_character,
                    );

                    if pose_distance < smallest_pose_distance {
                        smallest_pose_distance = pose_distance;
                        best_pose = pose;
                        // println!("Best Pose: {:?}", best_pose);
                        best_trajectory_index = i;
                        // println!("Chunk Index: {}", best_trajectory_index);
                    }
                }
            }
            let Some(best_trajectory) = nearest_trajectories[best_trajectory_index] else {
                return;
            };

            println!("Best Pose Trajectory: {:?}", best_trajectory);

            motion_data.jump_to_pose(
                best_trajectory.chunk_index,
                motion_asset
                    .trajectories
                    .time_from_chunk_offset(best_trajectory.chunk_offset),
            );

            // apply_pose(
            //     motion_data,
            //     &mut q_transforms,
            //     &mut main_character,
            //     best_pose,
            // );
        }

        // Reset the timer
        *time_passed = 0.0
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
            // warn!("Chunk ({chunk_index}) has less than 7 trajectories.");
            continue;
        }

        // println!("Chunk Counttttttt: {}", chunk_count);
        // println!("Chunk Indexxxxxxx: {}", chunk_index);

        println!("Chunk count: {}", chunk_count);
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

            let data_local_translations = trajectory
                .iter()
                .map(|trajectory| {
                    inv_matrix.transform_point3(trajectory.to_scale_rotation_translation().2)
                })
                // Rescale?
                .map(|v| v.xz() * 0.01)
                .collect::<Vec<_>>();

            // println!("{:?}", player_local_translations);
            // println!("{:?}", data_local_translations);

            let distance =
                calculate_trajectory_distance(&player_local_translations, &data_local_translations);

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
