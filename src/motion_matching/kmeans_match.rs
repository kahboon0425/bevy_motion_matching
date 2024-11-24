use std::time::Instant;

use bevy::prelude::*;

use crate::{
    motion::{chunk::ChunkIterator, MotionData},
    motion_matching::MatchTrajectory,
    trajectory::{Trajectory, TrajectoryConfig},
    ui::play_mode::MotionMatchingResult,
    Method, BVH_SCALE_RATIO,
};

use super::{MatchConfig, MotionMatchingSet, NearestTrajectories, TrajectoryMatch, PEAK_ALLOC};

use clustering::*;

pub struct KMeansMatchPlugin;

impl Plugin for KMeansMatchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            populate_kmeans
                .run_if(not(resource_exists::<KMeansResource>))
                .run_if(in_state(Method::KMeans)),
        )
        .add_systems(
            Update,
            trajectory_match_with_kmeans
                .in_set(MotionMatchingSet::GlobalMatch)
                .run_if(resource_exists::<KMeansResource>)
                .run_if(in_state(Method::KMeans)),
        );
    }
}
fn populate_kmeans(
    mut commands: Commands,
    motion_data: MotionData,
    trajectory_config: Res<TrajectoryConfig>,
) {
    let Some(motion_data) = motion_data.get() else {
        return;
    };

    let num_segments = trajectory_config.num_segments();
    let num_points = trajectory_config.num_points();

    let mut trajectory_offsets = Vec::new();

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
            for i in 1..data_traj.len() {
                let offset = (data_traj[i] - data_traj[i - 1]) * BVH_SCALE_RATIO;
                traj_offsets.push(offset.x);
                traj_offsets.push(offset.y);
            }

            trajectory_offsets.push((traj_offsets, chunk_index, chunk_offset));
        }
    }

    let data: Vec<Vec<f64>> = trajectory_offsets
        .iter()
        .map(|(offsets, _, _)| offsets.iter().map(|&x| x as f64).collect())
        .collect();

    // Number of clusters, 8 random centroid will be chosen
    let k = 8;
    // Max iterations
    let max_iter = 10;
    let clustering = kmeans(k, &data, max_iter);

    let mut cluster_members: Vec<Vec<(usize, usize, Vec<f32>)>> = vec![Vec::new(); k];

    for (i, cluster_id) in clustering.membership.iter().enumerate() {
        let (offsets, chunk_index, chunk_offset) = &trajectory_offsets[i];
        cluster_members[*cluster_id].push((*chunk_index, *chunk_offset, offsets.clone()));
    }

    commands.insert_resource(KMeansResource {
        centroids: clustering.centroids,
        cluster_memberships: clustering.membership,
        trajectory_offsets,
        cluster_members,
    })
}

fn trajectory_match_with_kmeans(
    q_trajectory: Query<(&Trajectory, &Transform)>,
    mut match_evr: EventReader<TrajectoryMatch>,
    match_config: Res<MatchConfig>,
    mut nearest_trajectories_evw: EventWriter<NearestTrajectories>,
    kmeans: Res<KMeansResource>,
    mut motion_matching_result: ResMut<MotionMatchingResult>,
) {
    // println!("KMeans Method");
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

        for i in 1..traj.len() {
            let offset = traj[i] - traj[i - 1];
            traj_offsets.push(offset.x);
            traj_offsets.push(offset.y);
        }

        let start_time = Instant::now();

        let mut nearest_centroids = Vec::new();
        for (i, centroid) in kmeans.centroids.iter().enumerate() {
            let centroid_f32: Vec<f32> = centroid.0.iter().map(|&x| x as f32).collect();
            let distance = offset_distance(&traj_offsets, &centroid_f32);

            if distance <= match_config.match_threshold {
                nearest_centroids.push((distance, i));
            } else {
                continue;
            }
        }

        let mut nearest_trajs = Vec::with_capacity(match_config.max_match_count);
        for (_distance, centroid_index) in nearest_centroids {
            if let Some(members) = kmeans.cluster_members.get(centroid_index) {
                for (chunk_index, chunk_offset, offsets) in members {
                    let distance = offset_distance(&traj_offsets, offsets);

                    if distance > match_config.match_threshold {
                        continue;
                    }

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
                }
            }
        }

        let traj_duration = start_time.elapsed().as_secs_f64() * 1000.0;

        let kmeans_search_peak_memory = PEAK_ALLOC.peak_usage_as_mb();

        let runs = motion_matching_result.matching_result.runs + 1;

        motion_matching_result.matching_result.avg_time =
            (motion_matching_result.matching_result.avg_time
                * motion_matching_result.matching_result.runs as f64
                + traj_duration)
                / runs as f64;
        motion_matching_result.matching_result.avg_memory =
            (motion_matching_result.matching_result.avg_memory
                * motion_matching_result.matching_result.runs as f64
                + kmeans_search_peak_memory as f64)
                / runs as f64;

        motion_matching_result.matching_result.runs = runs;
        nearest_trajs.sort_by(|t0, t1| t0.distance.total_cmp(&t1.distance));

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

#[derive(Resource)]
pub struct KMeansResource {
    pub centroids: Vec<Centroid>,
    pub cluster_memberships: Vec<usize>,
    // trajectory offsets with chunk index and chunk offset
    pub trajectory_offsets: Vec<(Vec<f32>, usize, usize)>,
    pub cluster_members: Vec<Vec<(usize, usize, Vec<f32>)>>,
}
