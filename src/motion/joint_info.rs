use bevy::prelude::*;
use bevy_bvh_anim::bvh_anim::ChannelType;
use bevy_bvh_anim::prelude::*;
use serde::{Deserialize, Serialize};

/// Serializable joint with minimal required data.
#[derive(Serialize, Deserialize, Debug)]
pub struct JointInfo {
    /// Name of joint.
    name: String,
    /// Offset position of joint.
    offset: Vec3,
    /// Parent index of this joint.
    parent_index: Option<usize>,
    /// Information needed for referencing pose data.
    pose_refs: Vec<PoseRef>,
}

impl JointInfo {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn offset(&self) -> Vec3 {
        self.offset
    }

    pub fn parent_index(&self) -> Option<usize> {
        self.parent_index
    }

    pub fn pose_refs(&self) -> &[PoseRef] {
        &self.pose_refs
    }
}

impl JointInfo {
    pub fn from_joint_data(joint_data: &JointData) -> Self {
        Self {
            name: joint_data.name().to_string(),
            offset: Vec3::new(
                joint_data.offset().x,
                joint_data.offset().y,
                joint_data.offset().z,
            ),
            parent_index: joint_data.parent_index(),
            pose_refs: joint_data
                .channels()
                .iter()
                .map(|c| PoseRef::from(*c))
                .collect(),
        }
    }
}

impl JointTrait for JointInfo {
    fn channels(&self) -> impl Iterator<Item = impl JointChannelTrait> {
        self.pose_refs.iter()
    }

    fn offset(&self) -> Vec3 {
        self.offset
    }

    fn parent_index(&self) -> Option<usize> {
        self.parent_index
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PoseRef {
    pose_index: usize,
    data_type: PoseDataType,
}

impl JointChannelTrait for &PoseRef {
    fn channel_type(&self) -> ChannelType {
        match self.data_type {
            PoseDataType::RotationX => ChannelType::RotationX,
            PoseDataType::RotationY => ChannelType::RotationY,
            PoseDataType::RotationZ => ChannelType::RotationZ,
            PoseDataType::PositionX => ChannelType::PositionX,
            PoseDataType::PositionY => ChannelType::PositionY,
            PoseDataType::PositionZ => ChannelType::PositionZ,
        }
    }

    fn motion_index(&self) -> usize {
        self.pose_index
    }
}

impl From<Channel> for PoseRef {
    fn from(value: Channel) -> Self {
        Self {
            pose_index: value.motion_index(),
            data_type: PoseDataType::from(value.channel_type()),
        }
    }
}

/// The available degrees of freedom along which a `Joint` may be manipulated.
///
/// A complete serializable match of [`ChannelType`].
#[derive(Serialize, Deserialize, Debug)]
pub enum PoseDataType {
    /// Can be rotated along the `x` axis.
    RotationX,
    /// Can be rotated along the `y` axis.
    RotationY,
    /// Can be rotated along the `z` axis.
    RotationZ,
    /// Can be translated along the `x` axis.
    PositionX,
    /// Can be translated along the `y` axis.
    PositionY,
    /// Can be translated along the `z` axis.
    PositionZ,
}

impl From<ChannelType> for PoseDataType {
    fn from(value: ChannelType) -> Self {
        match value {
            ChannelType::RotationX => Self::RotationX,
            ChannelType::RotationY => Self::RotationY,
            ChannelType::RotationZ => Self::RotationZ,
            ChannelType::PositionX => Self::PositionX,
            ChannelType::PositionY => Self::PositionY,
            ChannelType::PositionZ => Self::PositionZ,
        }
    }
}
