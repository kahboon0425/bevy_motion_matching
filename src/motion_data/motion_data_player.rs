//! Play motion data based on events and resources.

use crate::motion_matching::NearestTrajectory;
use crate::scene_loader::MainScene;
use crate::{bvh_manager::bvh_player::JointMap, GameMode};
use bevy::prelude::*;

use super::motion_data_asset::{JointInfo, Poses};
use super::{MotionData, MotionDataHandle};

pub(super) struct MotionDataPlayerPlugin;

impl Plugin for MotionDataPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MotionDataPlayerPair>()
            .add_systems(
                Update,
                motion_data_player.run_if(resource_exists::<MotionDataHandle>),
            )
            .add_systems(
                OnEnter(GameMode::Play),
                |mut player: ResMut<MotionDataPlayerPair>| {
                    player.is_playing = true;
                },
            )
            .add_systems(
                OnExit(GameMode::Play),
                |mut player: ResMut<MotionDataPlayerPair>| {
                    player.is_playing = false;
                },
            );
    }
}

// pub fn motion_data_player_pair_interpolation(
//     mut q_transforms: Query<&mut Transform>,
//     q_scene: Query<(&JointMap, Entity), With<MainScene>>,
//     motion_data: MotionData,
//     motion_data_player_pair: ResMut<MotionDataPlayerPair>,
// ) {
//     if motion_data_player_pair.is_playing == false {
//         return;
//     }

//     let Some(motion_data) = motion_data.get() else {
//         return;
//     };

//     let motion_poses = &motion_data.poses;

//     let current_motion_data_player = &motion_data_player_pair.players[0];
//     let next_motion_data_player = &motion_data_player_pair.players[1];

//     let current_chunk_index = current_motion_data_player.chunk_index;
//     let next_chunk_index = next_motion_data_player.chunk_index;

//     let current_poses = motion_poses.get_poses_from_chunk(current_chunk_index);
//     let next_poses = motion_poses.get_poses_from_chunk(next_chunk_index);

//     let current_chunk_offset = motion_poses.chunk_offset_from_time(current_motion_data_player.time);
//     let next_chunk_offset = motion_poses.chunk_offset_from_time(next_motion_data_player.time);

//     let (Some(start_pose), Some(end_pose)) = (
//         current_poses.get(current_chunk_offset),
//         next_poses.get(next_chunk_offset),
//     ) else {
//         return;
//     };

//     let interpolation_factor = motion_data_player_pair.interpolation_factor;

//     // Calculate interpolated position and rotation from a joint.
//     let calculate_trans_rot = |joint: &JointInfo| -> (Vec3, Quat) {
//         let (start_pos, start_rot) = start_pose.get_pos_rot(joint);
//         let (end_pos, end_rot) = end_pose.get_pos_rot(joint);

//         let translation = Vec3::lerp(start_pos, end_pos, interpolation_factor);
//         let rotation = Quat::slerp(start_rot, end_rot, interpolation_factor);

//         (translation, rotation)
//     };

//     for (joint_map, entity) in q_scene.iter() {
//         let root_joint = &motion_data.joints()[0];

//         // if let Some(mut transform) = joint_map
//         //     // Get root_joint transform.
//         //     .get(root_joint.name())
//         //     .and_then(|entity| q_transforms.get_mut(*entity).ok())
//         // {
//         //     let (translation, rotation) = calculate_trans_rot(root_joint);

//         //     transform.translation.y = translation.y;
//         //     transform.rotation = rotation;
//         // }

//         if let Ok(mut transform) = q_transforms.get_mut(entity) {
//             println!("Original Position {:?}", transform);
//             let (translation, rotation) = calculate_trans_rot(root_joint);

//             // transform.translation *= 0.01;
//             // transform.translation.y = translation.y;
//             // transform.rotation = rotation;

//             // Get current hip position
//             let (start_pos, start_rot) = start_pose.get_pos_rot(root_joint);
//             println!(
//                 "Current Hip Translation: {:?}, Rotation: {}",
//                 start_pos, start_rot
//             );

//             // Get next hip position
//             let (next_pos, next_rot) = end_pose.get_pos_rot(root_joint);

//             println!(
//                 "Next Hip Translation: {:?}, Rotation: {}",
//                 next_pos, next_rot
//             );

//             // Get different in position and rotation
//             let diff_in_x_pos = next_pos.x - start_pos.x;
//             let diff_in_z_pos = next_pos.z - start_pos.z;
//             let diff_in_rot = next_rot.angle_between(start_rot);
//             println!("Diff in Rotation: {}", diff_in_rot);

//             println!("Position diff x: {}, z: {}", diff_in_x_pos, diff_in_z_pos);
//             // Add to the original transform and rotation

//             transform.translation.z += -0.004;
//             // transform.translation.x += diff_in_x_pos * 0.01;
//             // transform.translation.z += diff_in_z_pos * 0.01;
//             // transform.translation.y = translation.y;
//         }

//         for joint in motion_data.joints().iter().skip(1) {
//             if let Some(mut transform) = joint_map
//                 // Get joint transform.
//                 .get(joint.name())
//                 .and_then(|entity| q_transforms.get_mut(*entity).ok())
//             {
//                 let (translation, rotation) = calculate_trans_rot(joint);

//                 transform.translation = joint.offset() + translation;
//                 transform.rotation = rotation;
//             }
//         }
//     }
// }

fn motion_data_player(
    mut q_transforms: Query<&mut Transform>,
    q_scene: Query<(&JointMap, Entity), With<MainScene>>,
    motion_data: MotionData,
    mut motion_player_pair: ResMut<MotionDataPlayerPair>,
    time: Res<Time>,
) {
    // Return if playback is not active.
    if !motion_player_pair.is_playing {
        return;
    }

    for motion_player in motion_player_pair.players.iter_mut() {
        motion_player.time += time.delta_seconds();
    }

    // Retrieve the motion data.
    let Some(motion_data) = motion_data.get() else {
        return;
    };

    let motion_poses = &motion_data.poses;

    let interpolated_trans_rot = |joint: &JointInfo| -> Option<(Vec3, Quat)> {
        // Motion player 0
        let (trans0, rot0) =
            motion_player_pair.players[0].current_trans_rot(motion_poses, joint)?;
        // Motion player 1
        let (trans1, rot1) =
            motion_player_pair.players[1].current_trans_rot(motion_poses, joint)?;

        // Interpolate between motion player 0 and motion player 1.
        let factor = motion_player_pair.interpolation_factor;

        Some((trans0.lerp(trans1, factor), rot0.slerp(rot1, factor)))
    };

    for (joint_map, entity) in q_scene.iter() {
        let root_joint = &motion_data.joints()[0];

        if let Some(mut transform) = joint_map
            .get(root_joint.name())
            .and_then(|entity| q_transforms.get_mut(*entity).ok())
        {
            let Some((translation, rotation)) = interpolated_trans_rot(root_joint) else {
                return;
            };
            transform.translation.y = translation.y;
            transform.rotation = rotation;
        }

        let calculate_offset = |motion_player_index: usize| -> Option<(Vec3, f32)> {
            let (prev_trans, prev_rot) = motion_player_pair.players[motion_player_index]
                .previous_trans_rot(motion_poses, root_joint)?;
            let (curr_trans, curr_rot) = motion_player_pair.players[motion_player_index]
                .current_trans_rot(motion_poses, root_joint)?;

            Some((
                curr_trans - prev_trans,
                Quat::angle_between(curr_rot, prev_rot),
            ))
        };

        if let Ok(mut transform) = q_transforms.get_mut(entity) {
            let Some((trans_offset0, rot_offset0)) = calculate_offset(0) else {
                return;
            };
            let Some((trans_offset1, rot_offset1)) = calculate_offset(1) else {
                return;
            };

            // Interpolate between motion player 0 and motion player 1.
            let factor = motion_player_pair.interpolation_factor;
            let trans_offset = trans_offset0.lerp(trans_offset1, factor);
            let rot_offset = rot_offset0.lerp(rot_offset1, factor);

            // transform.translation.x += trans_offset.x * 0.1;
            // transform.translation.z += trans_offset.z * 0.1;
        }

        for joint in motion_data.joints().iter().skip(1) {
            if let Some(mut transform) = joint_map
                .get(joint.name())
                .and_then(|entity| q_transforms.get_mut(*entity).ok())
            {
                let Some((translation, rotation)) = interpolated_trans_rot(joint) else {
                    return;
                };
                transform.translation = joint.offset() + translation;
                transform.rotation = rotation;
            }
        }
    }

    for motion_player in motion_player_pair.players.iter_mut() {
        motion_player.prev_time = motion_player.time;
    }
}

#[derive(Component, Default, Debug)]
pub struct MotionDataPlayer {
    /// The current chunk in the motion data asset to play.
    ///
    /// Get poses using [`Poses::get_poses_from_chunk`][get_poses_from_chunk].
    ///
    /// [get_poses_from_chunk]: crate::motion_data::motion_data_asset::Poses::get_poses_from_chunk
    pub chunk_index: usize,
    /// Duration in terms of seconds inside the [`Self::chunk_index`].
    pub time: f32,
    /// Previous duration in terms of seconds inside the [`Self::chunk_index`].
    pub prev_time: f32,
}

impl MotionDataPlayer {
    pub fn current_trans_rot(
        &self,
        motion_poses: &Poses,
        joint: &JointInfo,
    ) -> Option<(Vec3, Quat)> {
        self.calculate_trans_rot_impl(motion_poses, joint, self.time)
    }

    pub fn previous_trans_rot(
        &self,
        motion_poses: &Poses,
        joint: &JointInfo,
    ) -> Option<(Vec3, Quat)> {
        self.calculate_trans_rot_impl(motion_poses, joint, self.prev_time)
    }

    fn calculate_trans_rot_impl(
        &self,
        motion_poses: &Poses,
        joint: &JointInfo,
        time: f32,
    ) -> Option<(Vec3, Quat)> {
        let poses = motion_poses.get_poses_from_chunk(self.chunk_index);
        let chunk_offset = motion_poses.chunk_offset_from_time(time);

        // Get start and end poses for interpolation.
        let (start_pose, end_pose) = (
            poses.get(chunk_offset)?,
            poses
                .get(chunk_offset + 1)
                .or_else(|| poses.get(chunk_offset))?,
        );

        // Calculate time factor between poses for interpolation.
        let time_leak = self.time - motion_poses.time_from_chunk_offset(chunk_offset);
        let time_factor = f32::clamp(time_leak / motion_poses.interval(), 0.0, 1.0);

        let (start_pos, start_rot) = start_pose.get_pos_rot(joint);
        let (end_pos, end_rot) = end_pose.get_pos_rot(joint);

        let translation = Vec3::lerp(start_pos, end_pos, time_factor);
        let rotation = Quat::slerp(start_rot, end_rot, time_factor);

        Some((translation, rotation))
    }
}

#[derive(Resource, Default, Debug)]
pub struct MotionDataPlayerPair {
    pub players: [MotionDataPlayer; 2],
    pub interpolation_factor: f32,
    pub is_playing: bool,
    pub pair_bool: bool,
}

impl MotionDataPlayerPair {
    pub fn jump_to_pose(&mut self, chunk_index: usize, time: f32, index: usize) {
        self.players[index].chunk_index = chunk_index;
        self.players[index].time = time;
        self.players[index].prev_time = time;
    }
}
