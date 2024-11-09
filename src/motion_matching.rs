use bevy::prelude::*;

use crate::bvh_manager::bvh_player::JointMap;
use crate::motion::chunk::ChunkIterator;
use crate::motion::motion_asset::MotionAsset;
use crate::motion::motion_player::{JumpToPose, MotionPose};
use crate::motion::{MotionData, MotionHandle};
use crate::trajectory::{Trajectory, TrajectoryConfig, TrajectoryDistance, TrajectoryPoint};
use crate::ui::play_mode::MotionMatchingResult;
use crate::{GameMode, MainSet, BVH_SCALE_RATIO};

pub struct MotionMatchingPlugin;

impl Plugin for MotionMatchingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MotionMatchingConfig {
            max_match_count: 5,
            match_threshold: 1.0,
            pred_match_threshold: 1.0,
        })
        .add_event::<TrajectoryMatch>()
        .add_event::<PredictionMatch>()
        .add_event::<NearestTrajectories>()
        .add_systems(Startup, load_motion_data)
        .add_systems(
            Update,
            (test, trajectory_match, pose_match)
                .in_set(MainSet::MotionMatching)
                .run_if(in_state(GameMode::Play)),
        );
    }
}

fn test(input: Res<ButtonInput<KeyCode>>, mut match_evw: EventWriter<TrajectoryMatch>) {
    if input.just_pressed(KeyCode::Space) {
        match_evw.send(TrajectoryMatch);
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
    q_trajectory: Query<(&Trajectory, &Transform, Entity)>,
    mut match_evr: EventReader<TrajectoryMatch>,
    trajectory_config: Res<TrajectoryConfig>,
    match_config: Res<MotionMatchingConfig>,
    mut nearest_trajectories_evw: EventWriter<NearestTrajectories>,
) {
    if match_evr.is_empty() {
        return;
    }
    match_evr.clear();

    let Some(motion_data) = motion_data.get() else {
        return;
    };

    let num_segments = trajectory_config.num_segments();
    let num_points = trajectory_config.num_points();

    for (trajectory, transform, entity) in q_trajectory.iter() {
        let inv_matrix = transform.compute_matrix().inverse();
        let traj = trajectory
            .iter()
            .map(|&(mut point)| {
                point.translation = inv_matrix
                    .transform_point3(Vec3::new(point.translation.x, 0.0, point.translation.y))
                    .xz();
                // x axis is reversed in bevy.
                point.translation.x = -point.translation.x;
                point
            })
            .collect::<Vec<_>>();

        let mut nearest_trajs = Vec::with_capacity(match_config.max_match_count);

        for (chunk_index, chunk) in motion_data.trajectory_data.iter_chunk().enumerate() {
            println!("CHUNK #{chunk_index}");
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
                println!("{distance}");

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

/// Match only the prediction trajectory on the current playing trajectory.
///
/// Performs a match [`PredictionMatch`] event.
fn prediction_match(mut match_evr: EventReader<PredictionMatch>) {
    // if match_evr.is_empty() {
    //     return;
    // }
    // match_evr.clear();
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
pub struct MotionMatchingConfig {
    /// Maximum number of trajectory matches.
    pub max_match_count: usize,
    /// Any distance beyond this threshold will not be considered.
    pub match_threshold: f32,
    pub pred_match_threshold: f32,
}
