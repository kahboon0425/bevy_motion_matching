use bevy::prelude::*;
use bevy_bvh_anim::bvh_anim::ChannelType;
use bevy_bvh_anim::joint_traits::JointChannelTrait;

use crate::bvh_manager::bvh_player::JointMap;
use crate::motion::chunk::ChunkIterator;
use crate::motion::motion_asset::MotionAsset;
use crate::motion_matching::NearestTrajectory;
use crate::player::PlayerMarker;
use crate::scene_loader::MainScene;

pub struct PoseMatchingPlugin;

impl Plugin for PoseMatchingPlugin {
    fn build(&self, app: &mut App) {}
}

pub fn match_pose(
    nearest_trajectory: &NearestTrajectory,
    motion_data: &MotionAsset,
    q_transforms: &mut Query<&mut Transform, (Without<MainScene>, Without<PlayerMarker>)>,
    main_character: &mut Query<&JointMap, With<MainScene>>,
) -> (f32, Vec<f32>) {
    let chunk_index = nearest_trajectory.chunk_index;
    let chunk_offset = nearest_trajectory.chunk_offset;
    let poses = motion_data.pose_data.get_chunk_unchecked(chunk_index);
    let pose = poses.get(chunk_offset).unwrap();

    let mut total_distance = 0.0;

    for bone_map in main_character.iter() {
        let joints = motion_data.joints();

        for joint_data in joints.iter() {
            let bone_name = joint_data.name();
            let Some(&bone_entity) = bone_map.0.get(bone_name) else {
                continue;
            };

            let Ok(current_transform) = q_transforms.get_mut(bone_entity) else {
                continue;
            };

            let current_translation = current_transform.translation;
            let current_rotation = current_transform.rotation;

            let mut pose_translation = Vec3::ZERO;
            let mut pose_rotation = Vec3::ZERO;

            for pose_ref in joint_data.pose_refs() {
                let pose_value = pose[pose_ref.motion_index()];

                match pose_ref.channel_type() {
                    ChannelType::RotationX => pose_rotation.x = pose_value,
                    ChannelType::RotationY => pose_rotation.y = pose_value,
                    ChannelType::RotationZ => pose_rotation.z = pose_value,
                    ChannelType::PositionX => pose_translation.x = pose_value,
                    ChannelType::PositionY => pose_translation.y = pose_value,
                    ChannelType::PositionZ => pose_translation.z = pose_value,
                }
            }

            let pose_rotation_in_quat = Quat::from_euler(
                EulerRot::XYZ,
                pose_rotation.x.to_radians(),
                pose_rotation.y.to_radians(),
                pose_rotation.z.to_radians(),
            );

            // println!("Bone: {}", bone_name);
            let distance = calculate_pose_distance(
                current_translation,
                pose_translation + joint_data.offset(),
                current_rotation,
                pose_rotation_in_quat,
            );

            total_distance += distance;
        }
    }

    (total_distance, pose.to_vec())
}

pub fn calculate_pose_distance(
    current_translation: Vec3,
    pose_translation: Vec3,
    current_rotation: Quat,
    pose_rotation: Quat,
) -> f32 {
    let distance_for_position = (current_translation - pose_translation).length_squared();

    let rotation_distance = current_rotation.angle_between(pose_rotation);

    // println!(
    //     "position distance: {}, rotation distance: {}",
    //     distance_for_position, rotation_distance
    // );

    let sum_distance = distance_for_position + rotation_distance;

    sum_distance
}
