use bevy::prelude::*;

use crate::bvh_manager::bvh_player::JointMap;
use crate::motion::chunk::ChunkIterator;
use crate::motion::motion_asset::MotionAsset;
use crate::motion::motion_player::{JumpToPose, MotionPose, TrajectoryPosePair};
use crate::motion::{MotionData, MotionHandle};
use crate::player::PlayerMarker;
use crate::pose_matching::match_pose;
use crate::scene_loader::MainScene;
use crate::trajectory::{Trajectory, TrajectoryConfig, TrajectoryDistance, TrajectoryPoint};
use crate::{GameMode, BVH_SCALE_RATIO};

pub struct MotionMatchingPlugin;

impl Plugin for MotionMatchingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MotionMatchingResult>()
            .add_event::<TrajectoryMatch>()
            .add_event::<PredictionMatch>()
            .add_event::<NearestTrajectories>()
            .add_systems(Startup, load_motion_data)
            .add_systems(Update, prediction_match)
            .add_systems(
                Update,
                (test, trajectory_match, pose_match).run_if(in_state(GameMode::Play)),
            );
    }
}

fn test(
    // q_entities: Query<Entity, With<TrajectoryPosePair>>,
    input: Res<ButtonInput<KeyCode>>,
    mut match_evw: EventWriter<TrajectoryMatch>,
) {
    if input.just_pressed(KeyCode::Space) {
        match_evw.send(TrajectoryMatch);
        // for entity in q_entities.iter() {
        //     match_evw.send(TrajectoryMatch(entity));
        // }
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
    mut nearest_trajectories_evw: EventWriter<NearestTrajectories>,
) {
    if match_evr.is_empty() {
        return;
    }
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

        // println!("Entity\n{:?}\n", entity_trajectory);

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

                let distance = entity_trajectory.distance(&data_trajectory);
                // println!("{:?}", data_trajectory);
                // println!("{:#?}", distance);

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
        nearest_trajectories_evw.send(NearestTrajectories(nearest_trajectories_stack));
    }
}

fn pose_match(
    motion_data: MotionData,
    mut q_transforms: Query<&mut Transform, (Without<MainScene>, Without<PlayerMarker>)>,
    q_players: Query<(&JointMap, Entity), With<TrajectoryPosePair>>,
    mut nearest_trajectories_reader: EventReader<NearestTrajectories>,
    mut motion_matching_result: ResMut<MotionMatchingResult>,
    // q_entities: Query<Entity, With<TrajectoryPosePair>>,
    mut jump_evw: EventWriter<JumpToPose>,
) {
    let Some(motion_data) = motion_data.get() else {
        return;
    };
    let mut smallest_pose_distance = f32::MAX;
    let mut best_trajectory_index = 0;

    if nearest_trajectories_reader.is_empty() {
        return;
    }

    for (joint_map, entity) in q_players.iter() {
        for nearest_trajectories in nearest_trajectories_reader.read() {
            // println!("Nearest Traj No: {:?}", nearest_trajectories.len());
            for (i, nearest_traj) in nearest_trajectories.iter().enumerate() {
                if let Some(trajectory) = nearest_traj {
                    let pose_distance =
                        match_pose(trajectory, motion_data, &mut q_transforms, joint_map);

                    motion_matching_result.pose_matching_result[i] = pose_distance;

                    if pose_distance < smallest_pose_distance {
                        smallest_pose_distance = pose_distance;
                        best_trajectory_index = i;
                        // println!("Best Chunk Index: {}", best_trajectory_index);
                    }
                }
            }

            if let Some(best_trajectory) = nearest_trajectories[best_trajectory_index] {
                motion_matching_result.best_pose_result.chunk_index = best_trajectory.chunk_index;
                motion_matching_result.best_pose_result.chunk_offset = best_trajectory.chunk_offset;
                motion_matching_result.best_pose_result.trajectory_distance =
                    best_trajectory.distance;
                motion_matching_result.best_pose_result.pose_distance = smallest_pose_distance;

                // println!("Entity {entity}");

                println!("--- Chunk index: {}", best_trajectory.chunk_index);
                println!("--- Chunk Offset: {}", best_trajectory.chunk_offset);
                println!(
                    "--- Time: {}",
                    motion_data
                        .trajectory_data
                        .time_from_chunk_offset(best_trajectory.chunk_offset)
                );
                jump_evw.send(JumpToPose(
                    MotionPose {
                        chunk_index: best_trajectory.chunk_index,
                        time: motion_data
                            .trajectory_data
                            .time_from_chunk_offset(best_trajectory.chunk_offset),
                    },
                    entity,
                ));
            }
        }
    }
}

/// Match only the prediction trajectory on the current playing trajectory.
///
/// Performs a match [`PredictionMatch`] event.
fn prediction_match(
    motion_data: MotionData,
    q_trajectory: Query<(&Trajectory, &Transform)>,
    mut match_evr: EventWriter<TrajectoryMatch>,
    trajectory_config: Res<TrajectoryConfig>,
    mut jump_evr: EventReader<JumpToPose>,
) {
    let threshold = 0.02;

    let Some(motion_data) = motion_data.get() else {
        return;
    };

    let num_segments = trajectory_config.num_predict_segments();
    let num_points = trajectory_config.num_predict_points();

    for (trajectory, transform) in q_trajectory.iter() {
        let entity_inv_matrix = transform.compute_matrix().inverse();
        let entity_trajectory = trajectory
            .iter()
            .skip(trajectory_config.history_count)
            .map(|&(mut point)| {
                point.translation = entity_inv_matrix
                    .transform_point3(Vec3::new(point.translation.x, 0.0, point.translation.y))
                    .xz();
                point
            })
            .collect::<Vec<_>>();

        for JumpToPose(motion_pose, entity) in jump_evr.read() {
            let curr_chunk_index = motion_pose.chunk_index;
            let curr_time = motion_pose.time;
            // println!("Time {}", curr_time);
            let chunk_offset = motion_data
                .trajectory_data
                .chunk_offset_from_time(curr_time);
            // println!("Chunk Offset: {}", chunk_offset);

            let current_chunk = motion_data.trajectory_data.get_chunk(curr_chunk_index);
            // println!("Current Chunk: {:?}", current_chunk);

            if let Some(chunk) = current_chunk {
                let num_trajectories =
                    calculate_trajectory_count(chunk.len(), chunk_offset, num_segments);
                // println!("Number of traj: {}", num_trajectories);

                if num_trajectories <= num_segments {
                    match_evr.send(TrajectoryMatch);
                    return;
                } else {
                    let trajectory_data = &chunk[chunk_offset..chunk_offset + num_points];

                    // Center point of trajectory
                    let data_inv_matrix = trajectory_data[0].matrix.inverse();
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
                    let distance = entity_trajectory.distance(&data_trajectory);
                    println!("Distance: {}", distance);
                    if distance >= threshold {
                        println!("{}", "Sending");
                        match_evr.send(TrajectoryMatch);
                    }
                }
            }
        }
    }
}

fn calculate_trajectory_count(chunk_len: usize, offset: usize, num_segments: usize) -> usize {
    let length_from_offset = chunk_len.saturating_sub(offset);
    println!("Chunk len from curr chunk offset: {}", length_from_offset);
    if length_from_offset <= num_segments {
        0
    } else {
        length_from_offset - num_segments
    }
}

#[derive(Event, Debug)]
pub struct TrajectoryMatch;

// #[derive(Event, Debug)]
// pub struct TrajectoryMatch(pub Entity);

// TODO: Prediction match must loop back for loopable animations.
#[derive(Event, Debug, Deref)]
pub struct PredictionMatch(MotionPose);

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

#[derive(Event, Debug, Deref, DerefMut)]
pub struct NearestTrajectories([Option<NearestTrajectory>; 5]);

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

pub struct MotionMatchingConfig {
    pub match_count: usize,
    /// Any distance beyond this threshold will not be considered.
    pub match_threshold: f32,
    pub pred_match_threshold: f32,
}
