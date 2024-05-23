use bevy::prelude::*;
use bvh_anim::{Frame, Joint};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write};

use crate::{
    bvh::{bvh_asset::BvhAsset, bvh_player::get_pose},
    ui::BuildConfig,
};

pub struct MotionDatabasePlugin;

impl Plugin for MotionDatabasePlugin {
    fn build(&self, _app: &mut App) {}
}

pub type Pose = Vec<Vec<f32>>;

#[derive(Serialize, Deserialize)]
pub struct TrajectoryPosition {
    pub position: Vec3,
    pub time: f32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct MotionData {
    pub trajectories: Vec<TrajectoryPosition>,
    pub trajectory_offsets: Vec<usize>,
    pub poses: Vec<Pose>,
    pub pose_offsets: Vec<usize>,
}

// trajectories: [ bvh0:traj0, bvh0:traj1, bvh0:traj2, bvh1:traj0, bvh1:traj1, bvh2:traj0 ]
//      offsets: [ 0, 3, 5, 6 ]

pub fn extract_motion_data(bvh_asset: &Assets<BvhAsset>, build_config: &mut BuildConfig) {
    let mut motion_data = MotionData::default();

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

    // TODO(perf): Serialize into binary instead
    let convert_to_json = serde_json::to_string(&motion_data).unwrap();

    let mut motion_library = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        // TODO: specify a file name and possibly a  location
        .open("motion_data.json")
        .unwrap();

    motion_library
        .write_all(convert_to_json.as_bytes())
        .unwrap();
}

pub fn get_joint_position(joint: &Joint, frame: &Frame) -> Vec3 {
    let channels = joint.data().channels();
    let x = frame[&channels[0]];
    let y = frame[&channels[1]];
    let z = frame[&channels[2]];
    Vec3::new(x, y, z)
}
