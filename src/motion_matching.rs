use crate::bvh_manager::bvh_player::JointMap;
use crate::motion_data::motion_data_asset::MotionDataAsset;
use crate::motion_data::motion_data_player::MotionDataPlayerPair;
use crate::motion_data::{MotionData, MotionDataHandle};
use crate::player::{MovementDirection, PlayerMarker};
use crate::pose_matching::match_pose;
use crate::scene_loader::MainScene;
use crate::trajectory::Trajectory;
use bevy::prelude::*;

pub struct MotionMatchingPlugin;

impl Plugin for MotionMatchingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MotionMatchingResult {
            nearest_trajectories: [None; 5],
            pose_matching_result: [0.0; 5],
            best_pose_result: BesePoseResult {
                chunk_index: 0,
                chunk_offset: 0,
                trajectory_distance: 0.0,
                pose_distance: 0.0,
            },
        })
        .add_systems(Startup, load_motion_data)
        .add_systems(Update, match_trajectory);
    }
}

pub fn load_motion_data(mut commands: Commands, asset_server: Res<AssetServer>) {
    let file_path = "motion_data/motion_data.json";
    let motion_data = asset_server.load::<MotionDataAsset>(file_path);

    commands.insert_resource(MotionDataHandle(motion_data));
}

pub fn match_trajectory(
    user_input_trajectory: Query<(&Trajectory, &Transform, &MovementDirection), With<PlayerMarker>>,
    mut q_transforms: Query<&mut Transform, (Without<MainScene>, Without<PlayerMarker>)>,
    mut main_character: Query<&JointMap, With<MainScene>>,
    time: Res<Time>,
    mut motion_player_pair: ResMut<MotionDataPlayerPair>,
    motion_data: MotionData,
    mut match_time: Local<f32>,
    mut interpolation_time: Local<f32>,
    mut prev_direction: Local<Vec2>,
    mut motion_matching_result: ResMut<MotionMatchingResult>,
) {
    const TRAJECTORY_INTERVAL: f32 = 0.5;
    const MATCH_INTERVAL: f32 = 0.4;
    const INTERPOLATION_DURATION: f32 = TRAJECTORY_INTERVAL - MATCH_INTERVAL;

    const MATCH_TRAJECTORY_COUNT: usize = 5;

    let Ok((trajectory, transform, movement_direction)) = user_input_trajectory.get_single() else {
        return;
    };

    // if user input not changing, match every 0.4, if user input change, match
    if Vec2::dot(**movement_direction, *prev_direction) < 0.5
        && movement_direction.length_squared() > 0.1
    {
        *match_time = 0.0;
    }
    *prev_direction = **movement_direction;

    if motion_player_pair.is_playing == false {
        return;
    }

    // MATCH_INTERVAL -> 0.0
    *match_time -= time.delta_seconds();
    // 0.0 -> INTERPOLATION_DURATION (0 to 0.1)
    *interpolation_time = f32::min(
        INTERPOLATION_DURATION,
        *interpolation_time + time.delta_seconds(),
    );

    // (0 to 1)
    let mut interpolation_factor = *interpolation_time / INTERPOLATION_DURATION;
    if motion_player_pair.pair_bool == true {
        // Reverse interpolation factor.
        interpolation_factor = 1.0 - interpolation_factor;
    }
    motion_player_pair.interpolation_factor = interpolation_factor;

    if *match_time <= 0.0 {
        // If MATCH_INTERVAL have passed, match!

        // Reset the timers.
        *match_time = MATCH_INTERVAL;
        *interpolation_time = 0.0;

        motion_player_pair.pair_bool = !motion_player_pair.pair_bool;

        if let Some(motion_asset) = motion_data.get() {
            let nearest_trajectories = find_nearest_trajectories::<MATCH_TRAJECTORY_COUNT>(
                motion_asset,
                trajectory,
                transform,
            );
            // println!(
            //     "{MATCH_TRAJECTORY_COUNT} nearest trajectories:\n{:?}",
            //     nearest_trajectories
            // );

            motion_matching_result.nearest_trajectories = nearest_trajectories;

            let mut smallest_pose_distance = f32::MAX;
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

                    motion_matching_result.pose_matching_result[i] = pose_distance;

                    println!("Pose Distance: {}", pose_distance);

                    if pose_distance < smallest_pose_distance {
                        smallest_pose_distance = pose_distance;
                        best_trajectory_index = i;
                        // println!("Chunk Index: {}", best_trajectory_index);
                    }
                }
            }
            let Some(best_trajectory) = nearest_trajectories[best_trajectory_index] else {
                return;
            };

            motion_matching_result.best_pose_result.chunk_index = best_trajectory.chunk_index;
            motion_matching_result.best_pose_result.chunk_offset = best_trajectory.chunk_offset;
            motion_matching_result.best_pose_result.trajectory_distance = best_trajectory.distance;
            motion_matching_result.best_pose_result.pose_distance = smallest_pose_distance;

            if motion_player_pair.pair_bool {
                motion_player_pair.jump_to_pose(
                    best_trajectory.chunk_index,
                    motion_asset
                        .trajectories
                        .time_from_chunk_offset(best_trajectory.chunk_offset),
                    0,
                );
            } else {
                motion_player_pair.jump_to_pose(
                    best_trajectory.chunk_index,
                    motion_asset
                        .trajectories
                        .time_from_chunk_offset(best_trajectory.chunk_offset),
                    1,
                );
            }
        }
    } else {
        // *interpolation_time += time.delta_seconds();
        // *interpolation_time = f32::min(*interpolation_time, INTERPOLATION_DURATION);

        // let interpolation_factor = *interpolation_time / INTERPOLATION_DURATION;

        // motion_player_pair.interpolation_factor = interpolation_factor;

        // *interpolation_time = 0.0;
    }
}

#[derive(Component, Default, Debug)]
pub struct BesePoseResult {
    pub chunk_index: usize,
    pub chunk_offset: usize,
    pub trajectory_distance: f32,
    pub pose_distance: f32,
}

#[derive(Default, Resource)]
pub struct MotionMatchingResult {
    pub nearest_trajectories: [Option<NearestTrajectory>; 5],
    pub pose_matching_result: [f32; 5],
    pub best_pose_result: BesePoseResult,
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

        for chunk_offset in 0..chunk_count - 6 {
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
