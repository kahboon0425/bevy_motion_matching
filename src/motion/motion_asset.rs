use core::f32;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use bevy_bvh_anim::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::LARGE_EPSILON;

use super::joint_info::JointInfo;
use super::pose_data::PoseData;
use super::trajectory_data::{TrajectoryData, TrajectoryDataConfig, TrajectoryDataPoint};

pub(super) struct MotionAssetPlugin;

impl Plugin for MotionAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<MotionAsset>()
            .init_asset_loader::<MotionAssetLoader>();
    }
}

/// A memory and storage efficient storage of [`JointInfo`] and chunks of [`TrajectoryData`] & [`PoseData`].
#[derive(Asset, TypePath, Serialize, Deserialize, Debug)]
pub struct MotionAsset {
    /// Joint data.
    joints: Vec<JointInfo>,
    /// Trajectory data for trajectory matching.
    pub trajectory_data: TrajectoryData,
    /// Pose data for pose matching and animation sampling.
    pub pose_data: PoseData,
    pub animation_file: Vec<String>,
}

impl MotionAsset {
    pub fn new(bvh: &Bvh, config: TrajectoryDataConfig) -> Self {
        Self {
            joints: bvh
                .joints()
                .map(|j| JointInfo::from_joint_data(j.data()))
                .collect(),
            trajectory_data: TrajectoryData::new(config),
            pose_data: PoseData::new(bvh.frame_time().as_secs_f32()),
            animation_file: Vec::new(),
        }
    }

    pub fn append_bvhs<'a>(&mut self, bvhs: impl Iterator<Item = &'a BvhAsset>) {
        let traj_config = *self.trajectory_data.config();
        let pose_interval = self.pose_data.interval_time();

        let mut trajectory_chunk = Vec::<TrajectoryDataPoint>::new();

        for bvh in bvhs {
            let name = bvh.name();
            info!("Building {}...", name);

            let mut formatted_name = name.clone();
            let _ = formatted_name.split_off(name.len() - 4);

            self.animation_file.push(formatted_name);

            let num_frames = bvh.num_frames();
            let frame_time = bvh.frame_time().as_secs_f32();
            let root_joint = bvh
                .root_joint()
                .expect("A root joint should be present in the Bvh.");
            let root_joint = root_joint.data();

            if frame_time != pose_interval {
                warn!(
                    "Frame time ({}) does not match pose interval ({}). Skipping...",
                    frame_time, pose_interval
                );
                continue;
            }

            // 2 frames is a segment, so we need to deduct by 1.
            let bvh_duration = (num_frames.saturating_sub(1)) as f32 * frame_time;
            let num_points = (bvh_duration / traj_config.interval_time) as usize + 1;

            if num_points < 1 {
                warn!("There is no trajectory point at all to use. Skipping...");
                continue;
            }

            if bvh.loopable() == false && num_points < traj_config.num_points {
                warn!(
                    r#"Does not meet the minimum required trajectory point length: >={}. Skipping...
                    (Tip: Set it to loopable if it's loopable to avoid this warning.)"#,
                    traj_config.num_points
                );
                continue;
            }

            let mut prev_time = 0.0;

            let first_pos = bvh.frames().next().unwrap().get_pos(root_joint);

            let mut prev_pos = first_pos;
            let mut prev_world_pos = first_pos;

            // SAFETY: It's ok to go over, we have made sure that the bvh is loopable.
            for p in 0..num_points.max(traj_config.num_points) {
                let mut target_time = traj_config.interval_time * p as f32;

                if bvh.loopable() {
                    // Loop the time if needed.
                    target_time %= bvh_duration;
                }
                // Make sure it's not above the final frame.
                // (With an EPSILON error away :D)
                let time = f32::min(target_time, bvh_duration - LARGE_EPSILON);

                // Interpolate between 2 surrounding frame.
                let start = (time / frame_time) as usize;
                let end = start + 1;

                // Time distance between start frame and current trajectory's time stamp.
                let leak = time - start as f32 * frame_time;
                // Interpolation factor between start and end frame.
                let factor = leak / frame_time;

                // SAFETY: Calculation above should made sure that both
                // start & end frame index is within the bounds of frame count.
                let start_frame = bvh.frames().nth(start).unwrap();
                let end_frame = bvh.frames().nth(end).unwrap();

                let (start_pos, start_rot) = start_frame.get_pos_rot(root_joint);
                let (end_pos, end_rot) = end_frame.get_pos_rot(root_joint);

                let pos = Vec3::lerp(start_pos, end_pos, factor);
                let rot = Quat::slerp(start_rot, end_rot, factor);
                let velocity = ((end_pos - start_pos) / frame_time).xz();

                let pos_offset = match time < prev_time {
                    // From previous pos to current pos.
                    false => pos - prev_pos,
                    // Has looped over
                    true => {
                        // Get last frame
                        let last_pos = bvh.frames().last().unwrap().get_pos(root_joint);

                        // From previous pos to the last pos.
                        let prev_last_pos = last_pos - prev_pos;
                        // From first pos to curr pos.
                        let first_curr_pos = pos - first_pos;

                        prev_last_pos + first_curr_pos
                    }
                };

                // World pos may go out of bounds of the original bvh data.
                let world_pos = prev_world_pos + pos_offset;
                trajectory_chunk.push(TrajectoryDataPoint {
                    matrix: Mat4::from_rotation_translation(rot, world_pos),
                    velocity,
                });

                prev_time = time;
                prev_pos = pos;
                prev_world_pos = world_pos;
            }

            self.trajectory_data
                .append_trajectory_chunk(&mut trajectory_chunk);
            self.pose_data.append_frames(bvh);
        }
        println!("Bvh File Names: {:?}", self.animation_file);
        println!("Bvh File Len: {}", self.animation_file.len());
    }
}

impl MotionAsset {
    pub fn joints(&self) -> &[JointInfo] {
        &self.joints
    }

    pub fn get_joint(&self, index: usize) -> Option<&JointInfo> {
        self.joints.get(index)
    }
}

#[derive(Default)]
struct MotionAssetLoader;

impl AssetLoader for MotionAssetLoader {
    type Asset = MotionAsset;
    type Settings = ();
    type Error = MotionDataLoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let motion_data = serde_json::from_slice::<MotionAsset>(&bytes)?;

        Ok(motion_data)
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
