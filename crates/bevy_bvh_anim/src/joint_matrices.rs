use bevy::prelude::*;
use bvh_anim::{Bvh, ChannelType, Frame, JointData};

/// Stores world and local matrix of each joint.
pub struct JointMatrices {
    joints: Vec<JointData>,
    world_matrices: Vec<Mat4>,
    local_matrices: Vec<Mat4>,
}

impl JointMatrices {
    pub fn from_bvh(bvh: &Bvh) -> Self {
        let joints = bvh.joints().map(|j| j.data().clone()).collect::<Vec<_>>();
        let joint_count = joints.len();

        let mut bvh_matrices = Self {
            joints,
            world_matrices: vec![Mat4::IDENTITY; joint_count],
            local_matrices: vec![Mat4::IDENTITY; joint_count],
        };

        bvh_matrices.reset_joints();
        bvh_matrices
    }

    pub fn from_joints(joints: &[JointData]) -> Self {
        let joint_count = joints.len();

        let mut bvh_matrices = Self {
            joints: joints.to_vec(),
            world_matrices: vec![Mat4::IDENTITY; joint_count],
            local_matrices: vec![Mat4::IDENTITY; joint_count],
        };

        bvh_matrices.reset_joints();
        bvh_matrices
    }

    /// Reset joints to the default offsets from the bvh data.
    pub fn reset_joints(&mut self) {
        for joint in self.joints.iter() {
            let joint_index = joint.index();

            let offset = joint.offset();

            // Local matrix of the current joint
            let local_matrix = Mat4::from_rotation_translation(
                Quat::IDENTITY,
                Vec3::new(offset.x, offset.y, offset.z),
            );
            self.local_matrices[joint_index] = local_matrix;

            match joint.parent_index() {
                Some(parent_index) => {
                    let parent_matrix = self.world_matrices[parent_index];
                    self.world_matrices[joint_index] =
                        Mat4::mul_mat4(&parent_matrix, &local_matrix);
                }
                None => {
                    self.world_matrices[joint_index] = local_matrix;
                }
            }
        }
    }

    /// Applies a single frame from the bvh to all matrices.
    pub fn apply_frame(&mut self, frame: &Frame) {
        for joint in self.joints.iter() {
            let joint_index = joint.index();

            let offset = joint.offset();
            let mut translation = Vec3::new(offset.x, offset.y, offset.z);
            let mut euler = Vec3::ZERO;

            for channel in joint.channels() {
                // SAFETY: We assume that the provided channel exists in the motion data.
                match channel.channel_type() {
                    ChannelType::RotationX => euler.x = frame.get(channel).unwrap().to_radians(),
                    ChannelType::RotationY => euler.y = frame.get(channel).unwrap().to_radians(),
                    ChannelType::RotationZ => euler.z = frame.get(channel).unwrap().to_radians(),
                    ChannelType::PositionX => translation.x = *frame.get(channel).unwrap(),
                    ChannelType::PositionY => translation.y = *frame.get(channel).unwrap(),
                    ChannelType::PositionZ => translation.z = *frame.get(channel).unwrap(),
                }
            }

            let rotation = Quat::from_euler(EulerRot::XYZ, euler.x, euler.y, euler.z);
            // Local matrix of the current joint
            let local_matrix = Mat4::from_rotation_translation(rotation, translation);
            self.local_matrices[joint_index] = local_matrix;

            match joint.parent_index() {
                Some(parent_index) => {
                    let parent_matrix = self.world_matrices[parent_index];
                    self.world_matrices[joint_index] =
                        Mat4::mul_mat4(&parent_matrix, &local_matrix);
                }
                None => {
                    self.world_matrices[joint_index] = local_matrix;
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
