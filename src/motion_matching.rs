use bevy::math::NormedVectorSpace;
use bevy::prelude::*;

use crate::motion::chunk::ChunkIterator;
use crate::motion::motion_asset::MotionAsset;
use crate::motion::motion_player::MotionPose;
use crate::motion::{MotionData, MotionHandle};
use crate::trajectory::{Trajectory, TrajectoryConfig, TrajectoryPoint};
use crate::{GameMode, BVH_SCALE_RATIO};

pub struct MotionMatchingPlugin;

impl Plugin for MotionMatchingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MotionMatchingResult>()
            .add_event::<TrajectoryMatch>()
            .add_event::<PredictionMatch>()
            .add_systems(Startup, load_motion_data)
            // .add_systems(Update, match_trajectory)
            .add_systems(Update, trajectory_match.run_if(in_state(GameMode::Play)));
    }
}

pub fn load_motion_data(mut commands: Commands, asset_server: Res<AssetServer>) {
    let file_path = "motion_data/motion_data.json";
    let motion_data = asset_server.load::<MotionAsset>(file_path);

    commands.insert_resource(MotionHandle(motion_data));
}

/// Search for the best match trajectory from [`MotionData`].
///
/// Performs a match every [`TrajectoryMatch`] event.
fn trajectory_match(
    motion_data: MotionData,
    q_trajectory: Query<(&Trajectory, &Transform)>,
    mut match_evr: EventReader<TrajectoryMatch>,
    trajectory_config: Res<TrajectoryConfig>,
    mut motion_matching_result: ResMut<MotionMatchingResult>,
    // mut nearest_trajectories_writer: EventWriter<NearestTrajectories>,
) {
    // if match_evr.is_empty() {
    //     return;
    // }
    match_evr.clear();

    let Some(motion_data) = motion_data.get() else {
        return;
    };

    const N: usize = 5;
    let mut nearest_trajectories_stack = [None::<NearestTrajectory>; N];
    let num_segments = trajectory_config.num_segments();
    let num_points = trajectory_config.num_points();

    for (trajectory, transform) in q_trajectory.iter() {
        let entity_inv_matrix = transform.compute_matrix().inverse();
        let entity_trajectory = trajectory
            .iter()
            .map(|&(mut point)| {
                point.translation = entity_inv_matrix
                    .transform_point3(Vec3::new(point.translation.x, 0.0, point.translation.y))
                    .xz();
                point
            })
            .collect::<Vec<_>>();

        // println!("{:#?}", entity_trajectory);

        // let mut best_match_distance = f32::MAX;
        // let mut best_match_trajectory = None;

        let mut stack_count = 0;
        for (chunk_index, chunk) in motion_data.trajectory_data.iter_chunk().enumerate() {
            // Number of trajectory in this chunk.
            let num_trajectories = chunk.len() - num_segments;

            for chunk_offset in 0..num_trajectories {
                let trajectory_data = &chunk[chunk_offset..chunk_offset + num_points];

                // Center point of trajectory
                let data_inv_matrix = trajectory_data[trajectory_config.history_count]
                    .matrix
                    .inverse();
                let data_trajectory = trajectory_data
                    .iter()
                    .map(|point| {
                        let (.., translation) = point.matrix.to_scale_rotation_translation();
                        TrajectoryPoint {
                            translation: data_inv_matrix.transform_point3(translation).xz()
                                * BVH_SCALE_RATIO,
                            velocity: point.velocity * BVH_SCALE_RATIO,
                        }
                    })
                    .collect::<Vec<_>>();
                // println!("{:#?}", data_trajectory);

                let distance = calc_trajectory_distance(&entity_trajectory, &data_trajectory);
                // println!("Distance {}: {}", chunk_offset, distance);

                // motion_matching_result.nearest_trajectories = distance;

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

        motion_matching_result.nearest_trajectories = nearest_trajectories_stack;
    }
}

/// Match only the prediction trajectory on the current playing trajectory.
///
/// Performs a match [`PredictionMatch`] event.
fn prediction_match(mut match_evr: EventReader<PredictionMatch>) {
    // if match_evr.is_empty() {
    //     return;
    // }
    // match_evr.clear();
}

// TODO: IMPLEMENT
fn calc_trajectory_distance(traj0: &[TrajectoryPoint], traj1: &[TrajectoryPoint]) -> f32 {
    assert_eq!(traj0.len(), traj1.len());
    // 0.0

    let mut total_distance = 0.0;
    for (point0, point1) in traj0.iter().zip(traj1.iter()) {
        let translation_distance = Vec2::distance(point0.translation, point1.translation);
        let velocity_distance = (point0.velocity - point1.velocity).length() * 0.5;
        total_distance += translation_distance.powi(2) + velocity_distance.powi(2);
    }

    // Return the root mean squared distance
    (total_distance / traj0.len() as f32).sqrt()
}

#[derive(Event, Debug)]
pub struct TrajectoryMatch;

// TODO: Prediction match must loop back for loopable animations.
#[derive(Event, Debug, Deref)]
pub struct PredictionMatch(MotionPose);

// pub fn match_trajectory(
//     user_input_trajectory: Query<(&Trajectory, &Transform, &MovementDirection), With<PlayerMarker>>,
//     mut q_transforms: Query<&mut Transform, (Without<MainScene>, Without<PlayerMarker>)>,
//     mut main_character: Query<&JointMap, With<MainScene>>,
//     time: Res<Time>,
//     mut motion_player_pair: ResMut<MotionDataPlayerPair>,
//     motion_data: MotionData,
//     mut match_time: Local<f32>,
//     mut interpolation_time: Local<f32>,
//     mut prev_direction: Local<Vec2>,
//     mut motion_matching_result: ResMut<MotionMatchingResult>,
// ) {
//     const TRAJECTORY_INTERVAL: f32 = 0.5;
//     const MATCH_INTERVAL: f32 = 0.4;
//     const INTERPOLATION_DURATION: f32 = TRAJECTORY_INTERVAL - MATCH_INTERVAL;

//     const MATCH_TRAJECTORY_COUNT: usize = 5;

//     let Ok((trajectory, transform, direction)) = user_input_trajectory.get_single() else {
//         return;
//     };

//     // if user input not changing, match every 0.4, if user input change, match
//     if Vec2::dot(**direction, *prev_direction) < 0.5 && direction.length_squared() > 0.1 {
//         *match_time = 0.0;
//     }
//     *prev_direction = **direction;

//     if motion_player_pair.is_playing == false {
//         return;
//     }

//     // MATCH_INTERVAL -> 0.0
//     *match_time -= time.delta_seconds();
//     // 0.0 -> INTERPOLATION_DURATION (0 to 0.1)
//     *interpolation_time = f32::min(
//         INTERPOLATION_DURATION,
//         *interpolation_time + time.delta_seconds(),
//     );

//     // (0 to 1)
//     let mut interpolation_factor = *interpolation_time / INTERPOLATION_DURATION;
//     if motion_player_pair.pair_bool == true {
//         // Reverse interpolation factor.
//         interpolation_factor = 1.0 - interpolation_factor;
//     }
//     motion_player_pair.interpolation_factor = interpolation_factor;

//     // `MATCH_INTERVAL` have passed, match!
//     if *match_time <= 0.0 {
//         // Reset the timers.
//         *match_time = MATCH_INTERVAL;
//         *interpolation_time = 0.0;

//         let start_time = Instant::now();

//         if let Some(motion_asset) = motion_data.get() {
//             let nearest_trajectories = find_nearest_trajectories::<MATCH_TRAJECTORY_COUNT>(
//                 motion_asset,
//                 trajectory,
//                 transform,
//             );

//             info!(
//                 "{MATCH_TRAJECTORY_COUNT} nearest trajectories:\n{:?}",
//                 nearest_trajectories
//             );

//             let traj_duration = start_time.elapsed().as_secs_f64() * 1000.0;
//             let trajectory_duration_str = format!("{:.4}", traj_duration);
//             // println!("Time taken for trajectory matching: {trajectory_duration_str}");
//             motion_matching_result.traj_matching_time = trajectory_duration_str;

//             motion_matching_result.nearest_trajectories = nearest_trajectories;

//             let mut smallest_pose_distance = f32::MAX;
//             let mut best_trajectory_index = 0;

//             let start_pose_time = Instant::now();

//             // println!("Nearest Trajectory length: {}", nearest_trajectories.len());
//             for (i, nearest_trajectory) in nearest_trajectories.iter().enumerate() {
//                 if let Some(nearest_trajectory) = nearest_trajectory {
//                     let (pose_distance, pose) = match_pose(
//                         nearest_trajectory,
//                         motion_asset,
//                         &mut q_transforms,
//                         &mut main_character,
//                     );

//                     motion_matching_result.pose_matching_result[i] = pose_distance;

//                     if pose_distance < smallest_pose_distance {
//                         smallest_pose_distance = pose_distance;
//                         best_trajectory_index = i;
//                         // println!("Chunk Index: {}", best_trajectory_index);
//                     }
//                 }
//             }

//             let pose_duration = start_pose_time.elapsed().as_secs_f64() * 1000.0;
//             let pose_duration_str = format!("{:.4}", pose_duration);
//             // println!("Time taken for pose matching: {pose_duration_str}");
//             motion_matching_result.pose_matching_time = pose_duration_str;

//             if let Some(best_trajectory) = nearest_trajectories[best_trajectory_index] {
//                 motion_matching_result.best_pose_result.chunk_index = best_trajectory.chunk_index;
//                 motion_matching_result.best_pose_result.chunk_offset = best_trajectory.chunk_offset;
//                 motion_matching_result.best_pose_result.trajectory_distance =
//                     best_trajectory.distance;
//                 motion_matching_result.best_pose_result.pose_distance = smallest_pose_distance;

//                 let player_index = motion_player_pair.pair_bool as usize;
//                 motion_player_pair.jump_to_pose(
//                     best_trajectory.chunk_index,
//                     motion_asset
//                         .trajectory_data
//                         .time_from_chunk_offset(best_trajectory.chunk_offset + 3),
//                     player_index,
//                 );
//             }
//         }

//         // Flip boolean for the next match.
//         motion_player_pair.pair_bool = !motion_player_pair.pair_bool;
//     }
// }

#[derive(Component, Default, Debug)]
pub struct BestPoseResult {
    pub chunk_index: usize,
    pub chunk_offset: usize,
    pub trajectory_distance: f32,
    pub pose_distance: f32,
}

#[derive(Default, Resource)]
pub struct MotionMatchingResult {
    pub nearest_trajectories: [Option<NearestTrajectory>; 5],
    pub pose_matching_result: [f32; 5],
    pub best_pose_result: BestPoseResult,
    pub traj_matching_time: String,
    pub pose_matching_time: String,
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

// #[derive(Event, Debug, Deref, DerefMut)]
// pub struct NearestTrajectories([Option<NearestTrajectory>; 5]);

/// /// Find `N` number of nearest trajectories.
///
/// # Panic
///
/// Panic if `N` is 0.
pub fn find_nearest_trajectories<const N: usize>(
    motion_data: &MotionAsset,
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
    let threshold = 10.0;

    let trajectories = &motion_data.trajectory_data;

    let player_local_translations = player_trajectory
        .iter()
        .map(|player_trajectory| {
            player_inv_matrix.transform_point3(Vec3::new(
                player_trajectory.translation.x,
                0.0,
                player_trajectory.translation.y,
            ))
        })
        .map(|v| v.xz())
        .collect::<Vec<_>>();

    for (chunk_index, chunk) in trajectories.iter_chunk().enumerate() {
        let chunk_count = chunk.len();
        if chunk_count < 7 {
            warn!("Chunk ({chunk_index}) has less than 7 trajectories. (only {chunk_count})");
            continue;
        }

        for chunk_offset in 0..chunk_count - 6 {
            let trajectory = &chunk[chunk_offset..chunk_offset + 7];

            // Center point of trajectory
            let inv_matrix = trajectory[3].matrix.inverse();

            let data_local_translations = trajectory
                .iter()
                .map(|trajectory| {
                    inv_matrix.transform_point3(trajectory.matrix.to_scale_rotation_translation().2)
                })
                // Rescale?
                .map(|v| v.xz() * BVH_SCALE_RATIO)
                .collect::<Vec<_>>();

            let distance =
                trajectory_distance(&player_local_translations, &data_local_translations);

            // println!("Distance: {}", distance);
            // if distance > threshold {
            //     continue;
            // }

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

// TODO: Replace this
pub fn trajectory_distance(traj0: &[Vec2], traj1: &[Vec2]) -> f32 {
    // println!("{traj0:?}");
    // println!("{traj1:?}");
    // println!("===============\n");
    let mut distance = 0.0;
    for i in 1..traj0.len() {
        let offset0 = traj0[i] - traj0[i - 1];
        let offset1 = traj1[i] - traj1[i - 1];

        distance += Vec2::distance(offset1, offset0);
    }

    distance
}
