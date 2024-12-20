use bevy::prelude::*;
use bevy_bvh_anim::bvh_anim::ChannelType;
use bevy_bvh_anim::prelude::*;
use serde::{Deserialize, Serialize};

use super::chunk::{ChunkIterator, ChunkOffsets};
use super::joint_info::JointInfo;

#[derive(Serialize, Deserialize, Default, Debug, Deref, DerefMut, Clone)]
pub struct Pose(pub Vec<f32>);

impl Pose {
    #[inline]
    pub fn from_frame(frame: &Frame) -> Self {
        Self(frame.as_slice().to_vec())
    }

    /// Get position and rotation.
    #[must_use]
    pub fn get_pos_rot(&self, joint_info: &JointInfo) -> (Vec3, Quat) {
        let mut pos = Vec3::ZERO;
        let mut euler = Vec3::ZERO;

        for pose_ref in joint_info.pose_refs() {
            let i = pose_ref.motion_index();
            match pose_ref.channel_type() {
                ChannelType::RotationX => euler.x = self[i].to_radians(),
                ChannelType::RotationY => euler.y = self[i].to_radians(),
                ChannelType::RotationZ => euler.z = self[i].to_radians(),
                ChannelType::PositionX => pos.x = self[i],
                ChannelType::PositionY => pos.y = self[i],
                ChannelType::PositionZ => pos.z = self[i],
            }
        }

        (
            pos,
            Quat::from_euler(EulerRot::XYZ, euler.x, euler.y, euler.z),
        )
    }

    /// Get position and rotation in euler angles (in radians).
    #[must_use]
    pub fn get_pos_euler(&self, joint_info: &JointInfo) -> (Vec3, Vec3) {
        let mut pos = Vec3::ZERO;
        let mut euler = Vec3::ZERO;

        for pose_ref in joint_info.pose_refs() {
            let i = pose_ref.motion_index();
            match pose_ref.channel_type() {
                ChannelType::RotationX => euler.x = self[i].to_radians(),
                ChannelType::RotationY => euler.y = self[i].to_radians(),
                ChannelType::RotationZ => euler.z = self[i].to_radians(),
                ChannelType::PositionX => pos.x = self[i],
                ChannelType::PositionY => pos.y = self[i],
                ChannelType::PositionZ => pos.z = self[i],
            }
        }

        (pos, euler)
    }

    /// Get position only.
    #[must_use]
    pub fn get_pos(&self, joint_info: &JointInfo) -> Vec3 {
        let mut pos = Vec3::ZERO;

        for pose_ref in joint_info.pose_refs() {
            let i = pose_ref.motion_index();
            match pose_ref.channel_type() {
                ChannelType::PositionX => pos.x = self[i],
                ChannelType::PositionY => pos.y = self[i],
                ChannelType::PositionZ => pos.z = self[i],
                _ => {}
            }
        }

        pos
    }

    /// Get rotation only.
    #[must_use]
    pub fn get_rot(&self, joint_info: &JointInfo) -> Quat {
        let mut euler = Vec3::ZERO;

        for pose_ref in joint_info.pose_refs() {
            let i = pose_ref.motion_index();
            match pose_ref.channel_type() {
                ChannelType::RotationX => euler.x = self[i].to_radians(),
                ChannelType::RotationY => euler.y = self[i].to_radians(),
                ChannelType::RotationZ => euler.z = self[i].to_radians(),
                _ => {}
            }
        }

        Quat::from_euler(EulerRot::XYZ, euler.x, euler.y, euler.z)
    }

    /// Get rotation in euler angles (in radians).
    #[must_use]
    pub fn get_euler(&self, joint_info: &JointInfo) -> Vec3 {
        let mut euler = Vec3::ZERO;

        for pose_ref in joint_info.pose_refs() {
            let i = pose_ref.motion_index();
            match pose_ref.channel_type() {
                ChannelType::RotationX => euler.x = self[i].to_radians(),
                ChannelType::RotationY => euler.y = self[i].to_radians(),
                ChannelType::RotationZ => euler.z = self[i].to_radians(),
                _ => {}
            }
        }

        euler
    }

    /// Get position and rotation.
    #[must_use]
    pub fn get_matrix(&self, joint_info: &JointInfo) -> Mat4 {
        let (pos, rot) = self.get_pos_rot(joint_info);
        Mat4::from_rotation_translation(rot, pos)
    }

    pub fn lerp(&self, rhs: &Self, factor: f32) -> Self {
        let data = self
            .0
            .iter()
            .enumerate()
            .map(|(i, x)| f32::lerp(*x, rhs[i], factor))
            .collect::<Vec<_>>();

        Self(data)
    }
}

/// Stores chunks of poses.
#[derive(Serialize, Deserialize, Debug)]
pub struct PoseData {
    /// Pose data that can be sampled using [`JointInfo`].
    poses: Vec<Pose>,
    /// Offset index of [`Self::poses`] chunks.
    ///
    /// # Example
    ///
    /// \[0, 3, 5, 7\] contains chunk [0, 3), [3, 5), [5, 7)
    offsets: ChunkOffsets,
    /// Is a chunk loopable?
    loopables: Vec<bool>,
    /// Duration between each pose in seconds.
    interval_time: f32,
}

impl PoseData {
    pub fn new(interval: f32) -> Self {
        assert!(
            interval > 0.0,
            "Interval time between poses must be greater than 0!"
        );

        Self {
            poses: Vec::new(),
            offsets: ChunkOffsets::new(),
            loopables: Vec::new(),
            interval_time: interval,
        }
    }

    pub(super) fn append_frames(&mut self, bvh: &BvhAsset) {
        let frames = bvh.frames();

        self.offsets.push_chunk(frames.len());
        self.poses.extend(frames.map(Pose::from_frame));
        self.loopables.push(bvh.loopable());
    }

    pub fn is_chunk_loopable(&self, chunk_index: usize) -> Option<bool> {
        self.loopables.get(chunk_index).copied()
    }

    pub fn is_chunk_loopable_unchecked(&self, chunk_index: usize) -> bool {
        self.loopables[chunk_index]
    }

    /// Calculate the time value from a chunk offset index.
    pub fn time_from_chunk_offset(&self, chunk_offset: usize) -> f32 {
        chunk_offset as f32 * self.interval_time
    }

    /// Calculate the floored chunk offset index from a time value.
    pub fn chunk_offset_from_time(&self, time: f32) -> usize {
        (time / self.interval_time) as usize
    }
}

// Getter functions
impl PoseData {
    pub fn interval_time(&self) -> f32 {
        self.interval_time
    }
}

impl ChunkIterator for PoseData {
    type Item = Pose;

    fn offsets(&self) -> &ChunkOffsets {
        &self.offsets
    }

    fn items(&self) -> &[Self::Item] {
        &self.poses
    }
}
