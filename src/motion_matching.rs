use bevy::prelude::*;
use std::time::Instant;

use kdtree_match::KdTreeMatchPlugin;
use kmeans_match::KMeansMatchPlugin;

pub mod kdtree_match;
pub mod kmeans_match;

use crate::bvh_manager::bvh_player::JointMap;
use crate::motion::chunk::ChunkIterator;
use crate::motion::motion_asset::MotionAsset;
use crate::motion::motion_player::{
    JumpToPose, MotionPlayer, MotionPlayerConfig, MotionPose, TrajectoryPosePair,
};
use crate::motion::{MotionData, MotionHandle};
use crate::trajectory::{Trajectory, TrajectoryConfig, TrajectoryDistance, TrajectoryPoint};
use crate::ui::play_mode::MotionMatchingResult;
use crate::{motion_matching, GameMode, MainSet, Method, BVH_SCALE_RATIO};

use peak_alloc::PeakAlloc;
#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

pub struct MotionMatchingPlugin;

impl Plugin for MotionMatchingPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                MotionMatchingSet::Flow,
                MotionMatchingSet::PredictionMatch,
                MotionMatchingSet::GlobalMatch,
                MotionMatchingSet::PoseMatch,
            )
                .chain()
                .in_set(MainSet::MotionMatching)
                .run_if(in_state(GameMode::Play)),
        );

        app.add_plugins(KdTreeMatchPlugin)
            .add_plugins(KMeansMatchPlugin)
            .insert_resource(SelectedMethod {
                method: Method::BruteForceKNN,
            })
            .insert_resource(MatchConfig {
                max_match_count: 5,
                match_threshold: 0.2,
                pred_match_threshold: 0.15,
            })
            .add_event::<TrajectoryMatch>()
            .add_event::<PredictionMatch>()
            .add_event::<NearestTrajectories>()
            .add_systems(PreStartup, load_motion_data)
            .add_systems(
                Update,
                (
                    flow.in_set(MotionMatchingSet::Flow),
                    prediction_match.in_set(MotionMatchingSet::PredictionMatch),
                    trajectory_match
                        .in_set(MotionMatchingSet::GlobalMatch)
                        .run_if(in_state(Method::BruteForceKNN)),
                    pose_match,
                ),
            );
    }
}

pub fn load_motion_data(mut commands: Commands, asset_server: Res<AssetServer>) {
    let file_path = "motion_data/motion_data.json";
    let motion_data = asset_server.load::<MotionAsset>(file_path);

    commands.insert_resource(MotionHandle(motion_data));
}

fn flow(
    q_players: Query<(&MotionPlayer, &TrajectoryPosePair, Entity)>,
    trajectory_config: Res<TrajectoryConfig>,
    motion_player_config: Res<MotionPlayerConfig>,
    mut match_evw: EventWriter<TrajectoryMatch>,
    mut pred_match_evw: EventWriter<PredictionMatch>,
) {
    let predict_time = trajectory_config.predict_time();
    let interp_duration = motion_player_config.interp_duration();

    let max_elapsed_time = predict_time - interp_duration;
    assert!(
        max_elapsed_time > 0.0,
        "Prediction duration cannot be shorter than interpolation duration!"
    );

    for (motion_player, traj_pose_pair, entity) in q_players.iter() {
        let index = motion_player.target_pair_index();
        let Some(traj_pose) = &traj_pose_pair[index] else {
            // Find a new animation to play.
            match_evw.send(TrajectoryMatch(entity));
            continue;
        };

        match traj_pose.elapsed_time() < max_elapsed_time {
            true => {
                // Continue playing the animation...
                continue;
            }
            false => {
                pred_match_evw.send(PredictionMatch {
                    motion_pose: *traj_pose.motion_pose(),
                    entity,
                });
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
    match_config: Res<MatchConfig>,
    trajectory_config: Res<TrajectoryConfig>,
    mut pred_match_evr: EventReader<PredictionMatch>,
    mut match_evw: EventWriter<TrajectoryMatch>,
) {
    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    let trajectory_data = &motion_asset.trajectory_data;
    let pose_data = &motion_asset.pose_data;

    let num_points = trajectory_config.num_predict_points();

    for pred_match in pred_match_evr.read() {
        let Ok((trajectory, transform)) = q_trajectory.get(pred_match.entity) else {
            continue;
        };

        let inv_matrix = transform.compute_matrix().inverse();
        let traj = trajectory
            .iter()
            // Only match the prediction trajectory.
            .skip(trajectory_config.history_count)
            .map(|&(mut point)| {
                point.translation = inv_matrix
                    .transform_point3(Vec3::new(point.translation.x, 0.0, point.translation.y))
                    .xz();
                point
            })
            .collect::<Vec<_>>();

        let mut chunk_offset = trajectory_data.chunk_offset_from_time(pred_match.time);

        let (Some(data_traj_chunk), Some(loopable)) = (
            trajectory_data.get_chunk(pred_match.chunk_index),
            pose_data.is_chunk_loopable(pred_match.chunk_index),
        ) else {
            match_evw.send(TrajectoryMatch(pred_match.entity));
            continue;
        };

        // Do we have enough trajectories?
        if data_traj_chunk.len().saturating_sub(chunk_offset) < num_points {
            match loopable {
                true => chunk_offset = 0,
                false => {
                    match_evw.send(TrajectoryMatch(pred_match.entity));
                    continue;
                }
            }
        }

        let data_traj = &data_traj_chunk[chunk_offset..chunk_offset + num_points];

        // Center point of trajectory.
        let data_inv_matrix = data_traj[0].matrix.inverse();
        let data_traj = data_traj
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

        if traj.distance(&data_traj) > match_config.pred_match_threshold {
            match_evw.send(TrajectoryMatch(pred_match.entity));
        }
    }
}

/// Search for the best match trajectory from [`MotionData`].
///
/// Performs a match every [`TrajectoryMatch`] event.
fn trajectory_match(
    motion_data: MotionData,
    q_trajectory: Query<(&Trajectory, &Transform)>,
    mut match_evr: EventReader<TrajectoryMatch>,
    trajectory_config: Res<TrajectoryConfig>,
    match_config: Res<MatchConfig>,
    mut nearest_trajectories_evw: EventWriter<NearestTrajectories>,
    mut motion_matching_result: ResMut<MotionMatchingResult>,
) {
    println!("Brute Force KNN Method");
    PEAK_ALLOC.reset_peak_usage();
    let Some(motion_data) = motion_data.get() else {
        return;
    };

    let num_segments = trajectory_config.num_segments();
    let num_points = trajectory_config.num_points();

    for traj_match in match_evr.read() {
        let entity = **traj_match;
        let Ok((traj, transform)) = q_trajectory.get(entity) else {
            continue;
        };

        let inv_matrix = transform.compute_matrix().inverse();
        let traj = traj
            .iter()
            .map(|&(mut point)| {
                point.translation = inv_matrix
                    .transform_point3(Vec3::new(point.translation.x, 0.0, point.translation.y))
                    .xz();
                point
            })
            .collect::<Vec<_>>();

        // println!("current traj: {:?}", traj);
        let mut nearest_trajs = Vec::with_capacity(match_config.max_match_count);

        let start_time = Instant::now();
        for (chunk_index, chunk) in motion_data.trajectory_data.iter_chunk().enumerate() {
            // Number of trajectory in this chunk.
            let num_trajectories = chunk.len() - num_segments;

            for chunk_offset in 0..num_trajectories {
                let data_traj = &chunk[chunk_offset..chunk_offset + num_points];

                // Center point of trajectory
                let data_inv_matrix = data_traj[trajectory_config.history_count].matrix.inverse();

                let data_traj = data_traj
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

                let distance = traj.distance(&data_traj);

                // Distance must be below the threshold.
                if distance > match_config.match_threshold {
                    continue;
                }

                if nearest_trajs.len() < match_config.max_match_count {
                    // Stack not yet full, push into it
                    nearest_trajs.push(MatchTrajectory {
                        distance,
                        chunk_index,
                        chunk_offset,
                    });
                } else if let Some(worst_match) = nearest_trajs.last_mut() {
                    if distance < worst_match.distance {
                        *worst_match = MatchTrajectory {
                            distance,
                            chunk_index,
                            chunk_offset,
                        };
                    }
                }

                // Sort so that trajectories with the largest distance
                // is placed as the final element in the stack
                nearest_trajs.sort_by(|t0, t1| t0.distance.total_cmp(&t1.distance));
            }
        }

        let knn_search_peak_memory = PEAK_ALLOC.peak_usage_as_mb();
        println!(
            "KNN search peak memory usage: {} MB",
            knn_search_peak_memory
        );
        let traj_duration = start_time.elapsed().as_secs_f64() * 1000.0;
        let trajectory_duration_str = format!("{:.4}", traj_duration);
        println!("Time taken for trajectory matching: {trajectory_duration_str}");

        let runs = motion_matching_result.matching_result.runs + 1;

        motion_matching_result.matching_result.avg_time =
            (motion_matching_result.matching_result.avg_time
                * motion_matching_result.matching_result.runs as f64
                + traj_duration)
                / runs as f64;
        motion_matching_result.matching_result.avg_memory =
            (motion_matching_result.matching_result.avg_memory
                * motion_matching_result.matching_result.runs as f64
                + knn_search_peak_memory as f64)
                / runs as f64;
        motion_matching_result.matching_result.runs = runs;

        nearest_trajectories_evw.send(NearestTrajectories {
            trajectories: nearest_trajs,
            entity,
        });
    }
}

fn pose_match(
    motion_data: MotionData,
    q_transforms: Query<&Transform>,
    q_joint_maps: Query<&JointMap>,
    mut nearest_trajectories_evr: EventReader<NearestTrajectories>,
    mut motion_matching_result: ResMut<MotionMatchingResult>,
    mut jump_evw: EventWriter<JumpToPose>,
) {
    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    for trajs in nearest_trajectories_evr.read() {
        motion_matching_result.trajectories_poses.clear();

        // Ignore if there is no trajectories at all.
        if trajs.is_empty() {
            continue;
        }

        let Ok(joint_map) = q_joint_maps.get(trajs.entity) else {
            continue;
        };

        let mut smallest_pose_dist = f32::MAX;
        let mut best_traj_index = 0;

        for (i, traj) in trajs.iter().enumerate() {
            // Get pose based on trajectory data.
            // TODO: check for looping. (UNSAFE!)
            let pose = motion_asset
                .pose_data
                .get_chunk(traj.chunk_index)
                .and_then(|poses| poses.get(traj.chunk_offset))
                .unwrap();

            let mut pose_dist = 0.0;

            for joint_info in motion_asset.joints() {
                let joint_name = joint_info.name();

                if let Some(transform) = joint_map
                    .get(joint_name)
                    .and_then(|e| q_transforms.get(*e).ok())
                {
                    let (pose_pos, pose_rot) = pose.get_pos_rot(joint_info);

                    // Calcualte distance and angle difference.
                    pose_dist += Vec3::distance(transform.translation, pose_pos);
                    pose_dist += Quat::angle_between(transform.rotation, pose_rot);
                }
            }
            pose_dist /= motion_asset.joints().len() as f32;
            pose_dist *= BVH_SCALE_RATIO;

            if pose_dist < smallest_pose_dist {
                smallest_pose_dist = pose_dist;
                best_traj_index = i;
            }

            motion_matching_result
                .trajectories_poses
                .push((*traj, pose_dist));
        }

        motion_matching_result.selected_trajectory = best_traj_index;

        let best_traj = &trajs[best_traj_index];
        jump_evw.send(JumpToPose {
            motion_pose: MotionPose {
                chunk_index: best_traj.chunk_index,
                time: motion_asset
                    .trajectory_data
                    .time_from_chunk_offset(best_traj.chunk_offset),
            },
            entity: trajs.entity,
        });
    }
}

#[derive(Event, Debug, Deref, DerefMut)]
pub struct TrajectoryMatch(pub Entity);

// TODO: Prediction match must loop back for loopable animations.
#[derive(Event, Debug, Deref, DerefMut)]
pub struct PredictionMatch {
    #[deref]
    pub motion_pose: MotionPose,
    pub entity: Entity,
}

#[derive(Component, Default, Debug)]
pub struct BestPoseResult {
    pub chunk_index: usize,
    pub chunk_offset: usize,
    pub trajectory_distance: f32,
    pub pose_distance: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct MatchTrajectory {
    /// Error distance from this trajectory to the trajecctory that is being compared to.
    pub distance: f32,
    /// Index pointing to the chunk that holds this trajectory.
    pub chunk_index: usize,
    /// Offset index into the chunk that holds this trajectory.
    pub chunk_offset: usize,
}

/// A vec of [`MatchTrajectory`] that has the least [`MatchTrajectory::distance`].
#[derive(Event, Debug, Deref, DerefMut)]
pub struct NearestTrajectories {
    #[deref]
    trajectories: Vec<MatchTrajectory>,
    entity: Entity,
}

#[derive(Resource, Debug)]
pub struct MatchConfig {
    /// Maximum number of trajectory matches.
    pub max_match_count: usize,
    /// Any distance beyond this threshold will not be considered.
    pub match_threshold: f32,
    pub pred_match_threshold: f32,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionMatchingSet {
    Flow,
    PredictionMatch,
    GlobalMatch,
    PoseMatch,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct SelectedMethod {
    pub method: Method,
}
