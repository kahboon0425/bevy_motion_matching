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
use std::{fs::{self, File}, io::{BufReader, Write}};

use crate::{
    bvh::{bvh_asset::BvhAsset, bvh_player::get_pose},
    ui::BuildConfig,
};

pub struct MotionDatabasePlugin;

impl Plugin for MotionDatabasePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<MotionDataAsset>()
            .init_asset_loader::<MotionDataAssetLoader>();
    }
}

pub type Pose = Vec<Vec<f32>>;

#[derive(Serialize, Deserialize, Debug)]
pub struct TrajectoryPosition {
    pub position: Vec3,
    pub time: f32,
}

#[derive(Asset, TypePath, Serialize, Deserialize, Default, Debug)]
pub struct MotionDataAsset {
    pub trajectories: Vec<TrajectoryPosition>,
    pub trajectory_offsets: Vec<usize>,
    pub joint_names: Vec<String>,
    pub joint_name_offsets: Vec<usize>,
    pub poses: Vec<Pose>,
    pub pose_offsets: Vec<usize>,
}

// trajectories: [ bvh0:traj0, bvh0:traj1, bvh0:traj2, bvh1:traj0, bvh1:traj1, bvh2:traj0 ]
//      offsets: [ 0, 3, 5, 6 ]

// joint_name_offsets output will be like this
// joint_name_offsets:[0,6,9,12,15,18,21,24,27,30,33,36,39,42,45,48,51,54,57,60,63,66,69]

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
            let mut bytes = Vec::new();

            reader.read_to_end(&mut bytes).await?;

            let motion_data: MotionDataAsset = serde_json::from_slice(&bytes)?;

            let motion_data_asset = MotionDataAsset {
                trajectories: motion_data.trajectories,
                trajectory_offsets: motion_data.trajectory_offsets,
                poses: motion_data.poses,
                pose_offsets: motion_data.pose_offsets,
                joint_names: motion_data.joint_names,
                joint_name_offsets: motion_data.joint_name_offsets,
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
    #[error("Could not load json file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not deserialize using serde: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn extract_motion_data(bvh_asset: &Assets<BvhAsset>, build_config: &mut BuildConfig) {
    let mut motion_data = MotionDataAsset::default();

    let mut trajectory_data_len = 0;
    let mut motion_data_len = 0;
    let mut joint_name_offsets = Vec::new();
    let mut joint_name_data_len = 0;

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
            for joint in bvh.joints() {
                motion_data
                    .joint_names
                    .push(joint.data().name().to_string());
                joint_name_offsets.push(joint_name_data_len);
                joint_name_data_len += joint.data().channels().len();
            }
            joint_name_offsets.push(joint_name_data_len);
        }

        motion_data.pose_offsets.push(motion_data_len);
        motion_data_len += bvh.num_frames();

        motion_data.poses.push(
            bvh.frames()
                .map(|f| f.as_slice().to_owned())
                .collect::<Vec<_>>(),
        );
    }

    motion_data.trajectory_offsets.push(trajectory_data_len);
    motion_data.pose_offsets.push(motion_data_len);
    motion_data.joint_name_offsets = joint_name_offsets;

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

pub fn load_motion_data_onto() -> MotionDataAsset {
    let file_path = "assets/motion_data/motion_data.json";
        let file = File::open(file_path).expect("file should open read only");
        let reader = BufReader::new(file);
        let motion_data: MotionDataAsset = serde_json::from_reader(reader).expect("file should be proper JSON");
        return motion_data;   
}

pub fn get_joint_position(joint: &Joint, frame: &Frame) -> Vec3 {
    let channels = joint.data().channels();
    let x = frame[&channels[0]];
    let y = frame[&channels[1]];
    let z = frame[&channels[2]];
    Vec3::new(x, y, z)
}
