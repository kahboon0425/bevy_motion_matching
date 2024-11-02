pub use bvh_anim;

use bevy::prelude::*;
use bvh_anim::{ChannelType, Frame, JointData};

pub mod prelude {
    pub use crate::bvh_asset::{BvhAsset, BvhAssetPlugin};
    pub use crate::joint_matrices::JointMatrices;
    pub use crate::joint_traits::{JointChannelTrait, JointTrait};
    pub use crate::FrameExt;
    // Re-exports bvh_anim's commonly used types
    pub use bvh_anim::{
        bvh, Axis as BvhAxis, Bvh, Channel, Frame, Frames, Joint, JointData, JointName,
    };
}
pub mod bvh_asset;
pub mod joint_matrices;
pub mod joint_traits;

pub trait FrameExt {
    fn get_pos_rot(&self, joint_data: &JointData) -> (Vec3, Quat);
}

impl FrameExt for Frame {
    #[must_use]
    fn get_pos_rot(&self, joint_data: &JointData) -> (Vec3, Quat) {
        let mut pos = Vec3::ZERO;
        let mut euler = Vec3::ZERO;

        for channel in joint_data.channels() {
            let Some(&data) = self.get(channel) else {
                continue;
            };

            match channel.channel_type() {
                ChannelType::RotationX => euler.x = data,
                ChannelType::RotationY => euler.y = data,
                ChannelType::RotationZ => euler.z = data,
                ChannelType::PositionX => pos.x = data,
                ChannelType::PositionY => pos.y = data,
                ChannelType::PositionZ => pos.z = data,
            }
        }

        (
            pos,
            Quat::from_euler(EulerRot::XYZ, euler.x, euler.y, euler.z),
        )
    }
}
