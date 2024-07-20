use bevy::prelude::*;
use bvh_anim::ChannelType;

use crate::joint_traits::{JointChannelTrait, JointTrait};

/// Stores world and local matrix of each joint.
pub struct JointMatrices<J: JointTrait> {
    joints: Vec<J>,
    world_matrices: Vec<Mat4>,
    local_matrices: Vec<Mat4>,
}

impl<J: JointTrait> JointMatrices<J>
where
    J: Clone,
{
    pub fn new(joints: &[J]) -> Self {
        let joint_count = joints.len();

        let mut bvh_matrices = Self {
            joints: joints.to_vec(),
            world_matrices: vec![Mat4::IDENTITY; joint_count],
            local_matrices: vec![Mat4::IDENTITY; joint_count],
        };

        bvh_matrices.reset_joints();
        bvh_matrices
    }
}

impl<J: JointTrait> JointMatrices<J> {
    /// Reset joints to the default offsets from the bvh data.
    pub fn reset_joints(&mut self) {
        for (i, joint) in self.joints.iter().enumerate() {
            let offset = joint.offset();

            // Local matrix of the current joint
            let local_matrix = Mat4::from_rotation_translation(
                Quat::IDENTITY,
                Vec3::new(offset.x, offset.y, offset.z),
            );
            self.local_matrices[i] = local_matrix;

            match joint.parent_index() {
                Some(parent_index) => {
                    let parent_matrix = self.world_matrices[parent_index];
                    self.world_matrices[i] = Mat4::mul_mat4(&parent_matrix, &local_matrix);
                }
                None => {
                    self.world_matrices[i] = local_matrix;
                }
            }
        }
    }

    /// Applies a single frame from the bvh to all matrices.
    pub fn apply_frame(&mut self, frame: &[f32]) {
        for (i, joint) in self.joints.iter().enumerate() {
            let mut euler = Vec3::ZERO;
            let mut translation = joint.offset();

            for channel in joint.channels() {
                let data = frame[channel.motion_index()];
                // SAFETY: We assume that the provided channel exists in the motion data.
                match channel.channel_type() {
                    ChannelType::RotationX => euler.x = data.to_radians(),
                    ChannelType::RotationY => euler.y = data.to_radians(),
                    ChannelType::RotationZ => euler.z = data.to_radians(),
                    ChannelType::PositionX => translation.x = data,
                    ChannelType::PositionY => translation.y = data,
                    ChannelType::PositionZ => translation.z = data,
                }
            }

            let rotation = Quat::from_euler(EulerRot::XYZ, euler.x, euler.y, euler.z);
            // Local matrix of the current joint
            let local_matrix = Mat4::from_rotation_translation(rotation, translation);
            self.local_matrices[i] = local_matrix;

            match joint.parent_index() {
                Some(parent_index) => {
                    let parent_matrix = self.world_matrices[parent_index];
                    self.world_matrices[i] = Mat4::mul_mat4(&parent_matrix, &local_matrix);
                }
                None => {
                    self.world_matrices[i] = local_matrix;
                    println!("rotation: {}, translation: {}", rotation, translation);
                }
            }
        }
    }

    pub fn root_joint_matrix(&self) -> Mat4 {
        self.world_matrices[0]
    }

    pub fn world_matrices(&self) -> &[Mat4] {
        &self.world_matrices
    }

    pub fn local_matrices(&self) -> &[Mat4] {
        &self.local_matrices
    }

    pub fn world_local_matrices(&self) -> impl Iterator<Item = (&Mat4, &Mat4)> {
        self.world_matrices.iter().zip(self.local_matrices.iter())
    }
}
