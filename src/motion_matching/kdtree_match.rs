use bevy::prelude::*;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;

use crate::motion::chunk::ChunkIterator;
use crate::motion::MotionData;
use crate::trajectory::{Trajectory, TrajectoryConfig};
use crate::BVH_SCALE_RATIO;

use super::{
    MatchConfig, MatchTrajectory, MotionMatchingSet, NearestTrajectories, TrajectoryMatch,
};

pub struct KdTreeMatchPlugin;

impl Plugin for KdTreeMatchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            populate_kdtree.run_if(not(resource_exists::<KdTreeResource>)),
        )
        .add_systems(
            Update,
            trajectory_match_with_kdtree
                .in_set(MotionMatchingSet::GlobalMatch)
                .run_if(resource_exists::<KdTreeResource>),
        );
    }
}

pub(super) fn populate_kdtree(
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
    println!("KdTree: {:?}", kdtree);
    commands.insert_resource(KdTreeResource(kdtree));
}

pub(super) fn trajectory_match_with_kdtree(
    q_trajectory: Query<(&Trajectory, &Transform)>,
    mut match_evr: EventReader<TrajectoryMatch>,
    match_config: Res<MatchConfig>,
    mut nearest_trajectories_evw: EventWriter<NearestTrajectories>,
    kd_tree: Res<KdTreeResource>,
) {
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

        // println!("current traj: {:?}", user_traj);

        let mut nearest_trajs = Vec::with_capacity(match_config.max_match_count);
        let nearest_trajectories = kd_tree
            .nearest(
                &traj_offsets,
                match_config.max_match_count,
                &squared_euclidean,
            )
            .unwrap();

        println!("{:?}", nearest_trajectories);

        for (distance, (chunk_index, chunk_offset)) in nearest_trajectories {
            // println!("Nearest Traj Distance: {:?}", distance);
            // if distance > match_config.match_threshold {
            //     continue;
            // }
            if nearest_trajs.len() < match_config.max_match_count {
                // Stack not yet full, push into it
                nearest_trajs.push(MatchTrajectory {
                    distance,
                    chunk_index: *chunk_index,
                    chunk_offset: *chunk_offset,
                });
            } else if let Some(worst_match) = nearest_trajs.last_mut() {
                if distance < worst_match.distance {
                    *worst_match = MatchTrajectory {
                        distance,
                        chunk_index: *chunk_index,
                        chunk_offset: *chunk_offset,
                    };
                }
            }

            // Sort so that trajectories with the largest distance
            // is placed as the final element in the stack
            nearest_trajs.sort_by(|t0, t1| t0.distance.total_cmp(&t1.distance));
        }

        nearest_trajectories_evw.send(NearestTrajectories {
            trajectories: nearest_trajs,
            entity,
        });
    }
}

fn distance_(traj: &[f32], stored_traj: &[f32]) -> f32 {
    let len = traj.len();
    assert_eq!(len, stored_traj.len());
    // println!("user traj: {:?}", traj);
    // println!("stored traj: {:?}", stored_traj);

    let mut offset_distance = 0.0;

    for i in 1..len / 2 {
        let x1 = 2 * i;
        let y1 = x1 + 1;
        let x0 = x1 - 2;
        let y0 = x1 - 1;

        let offset0 = Vec2::new(traj[x1] - traj[x0], traj[y1] - traj[y0]);
        let offset1 = Vec2::new(
            stored_traj[x1] - stored_traj[x0],
            stored_traj[y1] - stored_traj[y0],
        );

        offset_distance += offset0.distance(offset1);
    }

    // println!("Offset Distance: {}", offset_distance);
    offset_distance /= (len / 2).saturating_sub(1) as f32;

    offset_distance
}

#[derive(Resource, Deref, DerefMut)]
pub struct KdTreeResource(KdTree<f32, (usize, usize), Vec<f32>>);
