use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    utils::{
        thiserror::{self, Error},
        BoxedFuture,
    },
};
use bvh_anim::{Frame, Joint};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write};

use crate::{
    bvh::{bvh_asset::BvhAsset, bvh_player::get_pose},
    input_trajectory::Trajectory,
    ui::BuildConfig,
};

pub struct MotionDatabasePlugin;

impl Plugin for MotionDatabasePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<MotionDataAsset>()
            .init_asset_loader::<MotionDataAssetLoader>()
            .add_systems(Startup, load_motion_data)
            .add_systems(Update, match_trajectory)
            .add_systems(Update, (check_all_motion_data, check_motion_data));
    }
}

pub type Pose = Vec<Vec<f32>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct TrajectoryPosition {
    pub position: Vec2,
    pub time: f32,
}

#[derive(Asset, TypePath, Serialize, Deserialize, Default, Debug)]
pub struct MotionDataAsset {
    pub trajectories: Vec<TrajectoryPosition>,
    pub trajectory_offsets: Vec<usize>,
    pub joint_names: Vec<String>,
    pub poses: Vec<Pose>,
    pub pose_offsets: Vec<usize>,
}

// trajectories: [ bvh0:traj0, bvh0:traj1, bvh0:traj2, bvh1:traj0, bvh1:traj1, bvh2:traj0 ]
// offsets: [ 0, 3, 5, 6 ]

impl MotionDataAsset {
    pub fn find_closest_trajectory(&self, user_future_trajectory: &Trajectory) -> Option<usize> {
        let mut best_match = None;
        let mut min_distance = f32::MAX;
        for (i, trajectory_position) in self.trajectories.iter().enumerate() {
            let distance =
                calculate_trajectory_distance(user_future_trajectory, trajectory_position);
            println!("Distance: {} Index:{}", distance, i);

            if distance < min_distance {
                min_distance = distance;
                best_match = Some(i);
            }
        }

        best_match
    }

    // pub fn get_pose_for_trajectory(&self, trajectory_index: usize) -> Option<&Pose> {}
}

pub fn calculate_trajectory_distance(t1: &Trajectory, t2: &TrajectoryPosition) -> f32 {
    // distance = sqrt((p1-q1)^2 + (p2-q2)^2)
    t1.predictions
        .iter()
        .map(|p| {
            let q = &t2.position;
            (*p - *q).length_squared()
        })
        .sum::<f32>()
        .sqrt()
}

pub fn match_trajectory(
    motion_data_assets: Res<Assets<MotionDataAsset>>,
    query_motion_data: Query<&Handle<MotionDataAsset>>,
    user_input_trajectory: Query<&Trajectory>,
) {
    for handle in query_motion_data.iter() {
        if let Some(motion_data) = motion_data_assets.get(handle) {
            // println!("Data: {:?}", motion_data);

            for trajectory in user_input_trajectory.iter() {
                if let Some(trajectory_index) = motion_data.find_closest_trajectory(trajectory) {
                    println!("Best match index: {}", trajectory_index);
                    // if let Some(pose) = motion_data.get_pose_for_trajectory(trajectory_index) {
                    //     //play pose
                    // } else {
                    //     println!("Pose not found for trajectory index: {}", trajectory_index);
                    // }
                } else {
                    println!("No matching trajectory found.");
                }
            }
        }
    }
}

#[derive(Default)]
struct MotionDataAssetLoader;

impl AssetLoader for MotionDataAssetLoader {
    type Asset = MotionDataAsset;
    type Settings = ();
    type Error = MotionDataLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            println!("kkkskf");
            let mut bytes = Vec::new();

            reader.read_to_end(&mut bytes).await?;

            let motion_data: MotionDataAsset = serde_json::from_slice(&bytes)?;

            let motion_data_asset = MotionDataAsset {
                trajectories: motion_data.trajectories,
                trajectory_offsets: motion_data.trajectory_offsets,
                poses: motion_data.poses,
                pose_offsets: motion_data.pose_offsets,
                joint_names: motion_data.joint_names,
            };

            Ok(motion_data_asset)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MotionDataLoaderError {
    #[error("Could not load json file {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not deserialize json data")]
    Serde(#[from] serde_json::Error),
}

pub fn load_motion_data(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load::<MotionDataAsset>("motion_data/motion_data.json");

    commands.spawn(handle);
}

pub fn check_motion_data(
    mut commands: Commands,
    motion_data_assets: Res<Assets<MotionDataAsset>>,
    q_motion_data: Query<(Entity, &Handle<MotionDataAsset>)>,
) {
    let Ok((entity, handle)) = q_motion_data.get_single() else {
        return;
    };

    if let Some(motion_data) = motion_data_assets.get(handle) {
        // println!("Data: {:?}", motion_data);
        commands.entity(entity).despawn();
        commands.entity(entity).remove::<Handle<MotionDataAsset>>();
    }
}

pub fn check_all_motion_data(mut motion_data_event: EventReader<AssetEvent<MotionDataAsset>>) {
    for motion in motion_data_event.read() {
        match motion {
            AssetEvent::Added { id } => println!("Loaded: {:?}", id),
            _ => {} // AssetEvent::Modified { id } => todo!(),
                    // AssetEvent::Removed { id } => todo!(),
                    // AssetEvent::Unused { id } => todo!(),
                    // AssetEvent::LoadedWithDependencies { id } => todo!(),
        }
    }
}

pub fn extract_motion_data(bvh_asset: &Assets<BvhAsset>, build_config: &mut BuildConfig) {
    let mut motion_data = MotionDataAsset::default();

    let mut trajectory_data_len = 0;
    let mut motion_data_len = 0;

    for id in build_config.bvh_assets.iter() {
        let Some(BvhAsset(bvh)) = bvh_asset.get(*id) else {
            return;
        };

        let interval = 0.3333;
        let frame_count = bvh.frames().len();
        let total_duration = bvh.frame_time().as_secs_f32() * frame_count as f32;

        motion_data.trajectory_offsets.push(trajectory_data_len);
        let mut trajectory_index = 0;
        loop {
            let time = interval * trajectory_index as f32;
            if time > total_duration {
                break;
            }
            let (frame_index, _interp_factor) = get_pose(time, bvh);

            if let Some(future_frame) = bvh.frames().nth(frame_index) {
                if let Some(hip_joint) = bvh.joints().find(|j| j.data().name() == "Hips") {
                    let position = get_joint_position(&hip_joint, future_frame);
                    trajectory_data_len += 1;
                    motion_data
                        .trajectories
                        .push(TrajectoryPosition { position, time });
                }
            }
            trajectory_index += 1;
        }

        if motion_data.joint_names.is_empty() {
            motion_data.joint_names = bvh
                .joints()
                .map(|joint| joint.data().name().to_string())
                .collect();
        }

        motion_data.pose_offsets.push(motion_data_len);
        motion_data_len += bvh.num_frames();

        for frame in bvh.frames() {
            let pose = bvh
                .joints()
                .map(|joint| {
                    let channels = joint.data().channels();
                    channels.iter().map(|channel| frame[channel]).collect()
                })
                .collect();

            motion_data.poses.push(pose);
        }
    }

    motion_data.trajectory_offsets.push(trajectory_data_len);
    motion_data.pose_offsets.push(motion_data_len);

    // TODO(perf): Serialize into binary instead
    let convert_to_json = serde_json::to_string(&motion_data).unwrap();

    let mut motion_library = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        // TODO: specify a file name and possibly a  location
        .open("assets/motion_data/motion_data.json")
        .unwrap();

    motion_library
        .write_all(convert_to_json.as_bytes())
        .unwrap();
}

pub fn get_joint_position(joint: &Joint, frame: &Frame) -> Vec2 {
    let channels = joint.data().channels();
    let x = frame[&channels[0]];
    // let y = frame[&channels[1]];
    let z = frame[&channels[2]];
    // Vec3::new(x, y, z)
    Vec2::new(x, z)
}
