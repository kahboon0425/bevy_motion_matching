use bevy::prelude::*;

use crate::bvh_manager::bvh_player::JointMap;
use crate::motion::chunk::ChunkIterator;
use crate::motion::motion_asset::MotionAsset;
use crate::motion_matching::MatchTrajectory;

pub struct PoseMatchingPlugin;

impl Plugin for PoseMatchingPlugin {
    fn build(&self, app: &mut App) {}
}

fn match_pose(
    nearest_trajectory: &MatchTrajectory,
    motion_asset: &MotionAsset,
    q_transforms: &Query<&Transform>,
    joint_map: &JointMap,
) -> f32 {
    let chunk_index = nearest_trajectory.chunk_index;
    let chunk_offset = nearest_trajectory.chunk_offset;
    let poses = motion_asset.pose_data.get_chunk_unchecked(chunk_index);
    let pose = poses.get(chunk_offset).unwrap();

    let mut total_distance = 0.0;

    for joint_info in motion_asset.joints() {
        let joint_name = joint_info.name();

        let Some(entity_transform) = joint_map
            .get(joint_name)
            .and_then(|e| q_transforms.get(*e).ok())
        else {
            continue;
        };

        let entity_pos = entity_transform.translation;
        let entity_rot = entity_transform.rotation;
        let (pose_pos, pose_rot) = pose.get_pos_rot(joint_info);

        total_distance += Vec3::distance(entity_pos, pose_pos);
        total_distance += Quat::angle_between(entity_rot, pose_rot);
    }

    total_distance
}
