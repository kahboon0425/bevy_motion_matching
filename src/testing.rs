use std::{fs::File, io::Write};

use bevy::{ecs::system::SystemState, prelude::*};
use bevy_egui::egui;
use clustering::{kmeans, Centroid};
use kdtree::{distance::squared_euclidean, KdTree};
use serde::{Deserialize, Serialize};

use crate::{
    motion::{chunk::ChunkIterator, MotionData},
    motion_matching::{
        kdtree_match::offset_distance, kmeans_match::KMeansResource, MatchConfig, MatchTrajectory,
    },
    trajectory::{TestingData, TrajectoryConfig},
    BVH_SCALE_RATIO,
};

pub struct TestingPlugin;

impl Plugin for TestingPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<TestingState>()
            .add_systems(
                PreUpdate,
                (
                    populate_kmeans::<20, 150>.run_if(not(resource_exists::<KMeansResource>)),
                    populate_kdtree.run_if(not(resource_exists::<KdTreeStructure>)),
                ),
            )
            .init_resource::<NearestTrajectory>()
            .add_systems(OnEnter(TestingState::Loading), load_testing_data)
            .add_systems(
                Update,
                check_motion_data_asset_loaded.run_if(in_state(TestingState::Loading)),
            )
            .add_systems(
                OnEnter(TestingState::Loaded),
                (
                    traj_matching_with_kdtree.run_if(resource_exists::<KdTreeStructure>),
                    traj_matching_with_kmeans.run_if(resource_exists::<KMeansStructure>),
                    traj_matching_with_knn,
                )
                    .chain(),
            )
            .add_systems(OnEnter(TestingState::Save), write_to_csv);
    }
}

fn check_motion_data_asset_loaded(
    motion_data: MotionData,
    mut next_testing_state: ResMut<NextState<TestingState>>,
) {
    if motion_data.get().is_some() {
        next_testing_state.set(TestingState::Loaded);
    }
}

pub fn generate_testing_data(ui: &mut egui::Ui, world: &mut World) {
    let mut params = SystemState::<(Res<TestingData>,)>::new(world);

    let testing_data = params.get_mut(world);

    if ui.button("Generate Testing Data").clicked() {
        let convert_to_json = serde_json::to_string(&*testing_data.0).unwrap();

        let mut asset_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open("assets/testing_dataset.json")
            .unwrap();

        asset_file.write_all(convert_to_json.as_bytes()).unwrap();
    }
    ui.add_space(10.0);
}

fn load_testing_data(mut commands: Commands) {
    let file_path = "./assets/testing_dataset.json";

    let file = File::open(file_path).expect("Failed to open the file");

    // Deserialize the JSON content
    let test_data: Vec<Vec<Vec2>> =
        serde_json::from_reader(file).expect("Error while reading or parsing JSON");

    commands.insert_resource(TestData(test_data));
}

fn traj_matching_with_knn(
    motion_data: MotionData,
    test_data: Res<TestData>,
    match_config: Res<MatchConfig>,
    trajectory_config: Res<TrajectoryConfig>,
    mut nearest_trajectories: ResMut<NearestTrajectory>,
    mut next_testing_state: ResMut<NextState<TestingState>>,
) {
    next_testing_state.set(TestingState::Save);
    let Some(motion_data) = motion_data.get() else {
        return;
    };
    let num_segments = trajectory_config.num_segments();
    let num_points = trajectory_config.num_points();

    let mut nearest_trajs = Vec::new();
    for traj in test_data.iter() {
        let mut nearest_traj = MatchTrajectory {
            distance: f32::MAX,
            chunk_index: 0,
            chunk_offset: 0,
        };
        for (chunk_index, chunk) in motion_data.trajectory_data.iter_chunk().enumerate() {
            let num_trajectories = chunk.len() - num_segments;

            for chunk_offset in 0..num_trajectories {
                let data_traj = &chunk[chunk_offset..chunk_offset + num_points];

                let data_inv_matrix = data_traj[trajectory_config.history_count].matrix.inverse();

                let data_traj = data_traj
                    .iter()
                    .map(|point| {
                        let (.., translation) = point.matrix.to_scale_rotation_translation();
                        data_inv_matrix.transform_point3(translation).xz() * BVH_SCALE_RATIO
                    })
                    .collect::<Vec<_>>();

                let distance = distance(traj, &data_traj);

                if distance > match_config.match_threshold {
                    continue;
                }

                if distance < nearest_traj.distance {
                    nearest_traj = MatchTrajectory {
                        distance,
                        chunk_index,
                        chunk_offset,
                    };
                }
            }
        }
        nearest_trajs.push(nearest_traj);
    }
    nearest_trajectories.knn = nearest_trajs;
}

fn distance(lhs: &[Vec2], rhs: &[Vec2]) -> f32 {
    let len = lhs.len();
    assert_eq!(len, rhs.len());

    let mut offset_distance = 0.0;

    for i in 1..len {
        let offset0 = lhs[i] - lhs[i - 1];
        let offset1 = rhs[i] - rhs[i - 1];

        offset_distance += Vec2::distance(offset1, offset0);
    }

    offset_distance /= len.saturating_sub(1) as f32;

    offset_distance
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
    commands.insert_resource(KdTreeStructure(kdtree));
}

fn traj_matching_with_kdtree(
    match_config: Res<MatchConfig>,
    kd_tree: Res<KdTreeStructure>,
    mut nearest_trajectories: ResMut<NearestTrajectory>,
    test_data: Res<TestData>,
) {
    let mut nearest_trajs = Vec::new();

    for traj in test_data.iter() {
        let mut nearest_traj = MatchTrajectory {
            distance: f32::MAX,
            chunk_index: 0,
            chunk_offset: 0,
        };
        let mut traj_offsets = Vec::new();
        // Create trajectory offset.
        for i in 1..traj.len() {
            let offset = traj[i] - traj[i - 1];
            traj_offsets.push(offset.x);
            traj_offsets.push(offset.y);
        }

        if let Ok(results) = kd_tree.nearest(
            &traj_offsets,
            match_config.max_match_count,
            &squared_euclidean,
        ) {
            for (distance, &(chunk_index, chunk_offset)) in results {
                if distance < match_config.match_threshold && distance < nearest_traj.distance {
                    nearest_traj = MatchTrajectory {
                        distance,
                        chunk_index,
                        chunk_offset,
                    };
                }
            }
        }
        nearest_trajs.push(nearest_traj);
    }
    nearest_trajectories.kdtree = nearest_trajs;
}

fn populate_kmeans<const K: usize, const MAX_ITER: usize>(
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

    let clustering = kmeans(K, &data, MAX_ITER);

    let mut cluster_members: Vec<Vec<(usize, usize, Vec<f32>)>> = vec![Vec::new(); K];

    for (i, cluster_id) in clustering.membership.iter().enumerate() {
        let (offsets, chunk_index, chunk_offset) = &trajectory_offsets[i];
        cluster_members[*cluster_id].push((*chunk_index, *chunk_offset, offsets.clone()));
    }

    commands.insert_resource(KMeansStructure {
        centroids: clustering.centroids,
        cluster_members,
    })
}

fn traj_matching_with_kmeans(
    match_config: Res<MatchConfig>,
    kmeans: Res<KMeansStructure>,
    mut nearest_trajectories: ResMut<NearestTrajectory>,
    test_data: Res<TestData>,
) {
    let mut nearest_trajs = Vec::new();

    for traj in test_data.iter() {
        let mut nearest_traj = MatchTrajectory {
            distance: f32::MAX,
            chunk_index: 0,
            chunk_offset: 0,
        };

        let mut traj_offsets = Vec::new();

        for i in 1..traj.len() {
            let offset = traj[i] - traj[i - 1];
            traj_offsets.push(offset.x);
            traj_offsets.push(offset.y);
        }

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

        for (_distance, centroid_index) in nearest_centroids {
            if let Some(members) = kmeans.cluster_members.get(centroid_index) {
                for (chunk_index, chunk_offset, offsets) in members {
                    let distance = offset_distance(&traj_offsets, offsets);

                    if distance > match_config.match_threshold {
                        continue;
                    }

                    if distance < nearest_traj.distance {
                        nearest_traj = MatchTrajectory {
                            distance,
                            chunk_index: *chunk_index,
                            chunk_offset: *chunk_offset,
                        };
                    }
                }
            }
        }

        nearest_trajs.push(nearest_traj);
    }
    nearest_trajectories.kmeans = nearest_trajs;
}

fn write_to_csv(test_data: Res<TestData>, nearest_trajectories: Res<NearestTrajectory>) {
    let file = File::create("assets/traj_matching_result.csv").expect("Failed to create CSV file");
    let mut writer = csv::Writer::from_writer(file);

    let mut kd_tree_chunk_index_score = 0;
    let mut kd_tree_chunk_offset_score = 0;
    let mut kmeans_chunk_index_score = 0;
    let mut kmeans_chunk_offset_score = 0;

    let data_count = nearest_trajectories.knn.len();

    writer
        .write_record(vec![
            "Trajectories".to_string(),
            "kNN_chunk_index".to_string(),
            "kNN_chunk_offset".to_string(),
            "kDTree_chunk_index".to_string(),
            "kDTree_chunk_offset".to_string(),
            "kMeans_chunk_index".to_string(),
            "kMeans_chunk_offset".to_string(),
        ])
        .expect("Failed to write CSV headers");
    for (i, traj_data) in test_data.iter().enumerate() {
        let traj_str = format!("{:?}", traj_data);

        if let (Some(knn), Some(kdtree), Some(kmeans)) = (
            nearest_trajectories.knn.get(i),
            nearest_trajectories.kdtree.get(i),
            nearest_trajectories.kmeans.get(i),
        ) {
            if kdtree.chunk_index == knn.chunk_index {
                kd_tree_chunk_index_score += 1;
            }
            if kdtree.chunk_offset == knn.chunk_offset {
                kd_tree_chunk_offset_score += 1;
            }
            if kmeans.chunk_index == knn.chunk_index {
                kmeans_chunk_index_score += 1;
            }
            if kmeans.chunk_offset == knn.chunk_offset {
                kmeans_chunk_offset_score += 1;
            }

            writer
                .write_record(&[
                    traj_str,
                    knn.chunk_index.to_string(),
                    knn.chunk_offset.to_string(),
                    kdtree.chunk_index.to_string(),
                    kdtree.chunk_offset.to_string(),
                    kmeans.chunk_index.to_string(),
                    kmeans.chunk_offset.to_string(),
                ])
                .expect("Failed to write CSV record");
            writer.flush().expect("Failed to flush CSV writer");
        }
    }

    // println!("kd_tree_chunk_index_score: {}", kd_tree_chunk_index_score);
    // println!("kd_tree_chunk_offset_score: {}", kd_tree_chunk_offset_score);
    // println!("kmeans_chunk_index_score: {}", kmeans_chunk_index_score);
    // println!("kmeans_chunk_offset_score: {}", kmeans_chunk_offset_score);

    let kdtree_accuracy = (kd_tree_chunk_index_score as f64 + kd_tree_chunk_offset_score as f64)
        / (data_count as f64 * 2.0)
        * 100.0;

    let kmeans_accuracy = (kmeans_chunk_index_score as f64 + kmeans_chunk_offset_score as f64)
        / (data_count as f64 * 2.0)
        * 100.0;

    println!("kd_tree_accuracy: {:.2} %", kdtree_accuracy);
    println!(
        "kmeans_accuracy (k: 20, max_iter: 150): {:.2} %",
        kmeans_accuracy
    );
}

#[derive(Resource, Debug, Default, Deref, DerefMut, Serialize, Deserialize)]
struct TestData(Vec<Vec<Vec2>>);

#[derive(Resource, Debug, Clone, Default)]
struct NearestTrajectory {
    knn: Vec<MatchTrajectory>,
    kmeans: Vec<MatchTrajectory>,
    kdtree: Vec<MatchTrajectory>,
}

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TestingState {
    #[default]
    Loading,
    Loaded,
    Save,
}

#[derive(Resource, Deref, DerefMut)]
struct KdTreeStructure(KdTree<f32, (usize, usize), Vec<f32>>);

#[derive(Resource)]
struct KMeansStructure {
    centroids: Vec<Centroid>,
    cluster_members: Vec<Vec<(usize, usize, Vec<f32>)>>,
}
