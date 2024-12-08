use std::time::Instant;

use bevy::prelude::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;

use crate::motion::chunk::ChunkIterator;
use crate::motion::MotionData;
use crate::trajectory::{Trajectory, TrajectoryConfig};
use crate::ui::play_mode::MotionMatchingResult;
use crate::{Method, BVH_SCALE_RATIO};

use super::{
    MatchConfig, MatchTrajectory, MotionMatchingSet, NearestTrajectories, TrajectoryMatch,
    PEAK_ALLOC,
};

pub struct KdTreeMatchPlugin;

impl Plugin for KdTreeMatchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            populate_kdtree
                .run_if(not(resource_exists::<KdTreeResource>))
                .run_if(in_state(Method::KdTree)),
        )
        .add_systems(
            Update,
            trajectory_match_with_kdtree
                .in_set(MotionMatchingSet::GlobalMatch)
                .run_if(resource_exists::<KdTreeResource>)
                .run_if(in_state(Method::KdTree)),
        );
    }
}

fn populate_kdtree(
    mut commands: Commands,
    motion_data: MotionData,
    trajectory_config: Res<TrajectoryConfig>,
) {
    let Some(motion_data) = motion_data.get() else {
        return;
    };

    let num_segments = trajectory_config.num_segments();
    let num_points = trajectory_config.num_points();

    let mut kdtree = KdTree::new(num_segments * 2);

    // Populate KD-Tree with motion data
    for (chunk_index, chunk) in motion_data.trajectory_data.iter_chunk().enumerate() {
        let num_trajectories = chunk.len() - num_segments;

        for chunk_offset in 0..num_trajectories {
            let data_traj = &chunk[chunk_offset..chunk_offset + num_points];
            let data_inv_matrix = data_traj[trajectory_config.history_count].matrix.inverse();

            let data_traj = data_traj
                .iter()
                .map(|point| {
                    let (.., translation) = point.matrix.to_scale_rotation_translation();
                    data_inv_matrix.transform_point3(translation).xz()
                })
                .collect::<Vec<_>>();

            let mut traj_offsets = Vec::new();
            // Add each offset from the trajectory to the KD-Tree
            for i in 1..data_traj.len() {
                let offset = (data_traj[i] - data_traj[i - 1]) * BVH_SCALE_RATIO;
                traj_offsets.push(offset.x);
                traj_offsets.push(offset.y);
            }

            kdtree
                .add(traj_offsets, (chunk_index, chunk_offset))
                .unwrap();
        }
    }
    commands.insert_resource(KdTreeResource(kdtree));
}

fn trajectory_match_with_kdtree(
    q_trajectory: Query<(&Trajectory, &Transform)>,
    mut match_evr: EventReader<TrajectoryMatch>,
    match_config: Res<MatchConfig>,
    mut nearest_trajectories_evw: EventWriter<NearestTrajectories>,
    kd_tree: Res<KdTreeResource>,
    mut motion_matching_result: ResMut<MotionMatchingResult>,
) {
    // println!("KDTree Method");
    PEAK_ALLOC.reset_peak_usage();
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
                point.translation
            })
            .collect::<Vec<_>>();

        let mut traj_offsets = Vec::new();
        // Create trajectory offset.
        for i in 1..traj.len() {
            let offset = traj[i] - traj[i - 1];
            traj_offsets.push(offset.x);
            traj_offsets.push(offset.y);
        }

        let start_time = Instant::now();

        let nearest_trajs = kd_tree
            .nearest(
                &traj_offsets,
                match_config.max_match_count,
                &squared_euclidean,
            )
            .unwrap()
            .into_iter()
            .filter(|(distance, (..))| *distance < match_config.match_threshold)
            .map(|(distance, &(chunk_index, chunk_offset))| MatchTrajectory {
                distance,
                chunk_index,
                chunk_offset,
            })
            .collect::<Vec<_>>();

        let traj_duration = start_time.elapsed().as_secs_f64() * 1000.0;

        let kdtree_search_peak_memory = PEAK_ALLOC.peak_usage_as_mb();

        let runs = motion_matching_result.matching_result.runs + 1;

        motion_matching_result.matching_result.avg_time =
            (motion_matching_result.matching_result.avg_time
                * motion_matching_result.matching_result.runs as f64
                + traj_duration)
                / runs as f64;
        motion_matching_result.matching_result.avg_memory =
            (motion_matching_result.matching_result.avg_memory
                * motion_matching_result.matching_result.runs as f64
                + kdtree_search_peak_memory as f64)
                / runs as f64;
        motion_matching_result.matching_result.runs = runs;

        nearest_trajectories_evw.send(NearestTrajectories {
            trajectories: nearest_trajs,
            entity,
        });
    }
}

pub fn offset_distance(offsets0: &[f32], offsets1: &[f32]) -> f32 {
    let len = offsets0.len();
    debug_assert_eq!(len, offsets1.len());

    let mut offset_distance = 0.0;

    for i in 0..len / 2 {
        let x_index = i * 2;
        let y_index = x_index + 1;

        let offset0 = Vec2::new(offsets0[x_index], offsets0[y_index]);
        let offset1 = Vec2::new(offsets1[x_index], offsets1[y_index]);

        offset_distance += offset0.distance(offset1);
    }

    offset_distance /= (len / 2).saturating_sub(1) as f32;
    offset_distance
}

#[derive(Resource, Deref, DerefMut)]
pub struct KdTreeResource(KdTree<f32, (usize, usize), Vec<f32>>);
