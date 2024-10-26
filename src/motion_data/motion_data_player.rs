//! Play motion data based on events and resources.

use crate::motion_matching::NearestTrajectory;
use crate::scene_loader::MainScene;
use crate::{bvh_manager::bvh_player::JointMap, GameMode};
use bevy::prelude::*;

use super::motion_data_asset::JointInfo;
use super::{MotionData, MotionDataHandle};

pub(super) struct MotionDataPlayerPlugin;

impl Plugin for MotionDataPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MotionDataPlayerPair>()
            .add_systems(
                Update,
                motion_data_player_pair_interpolation.run_if(resource_exists::<MotionDataHandle>),
            )
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

pub fn motion_data_player_pair_interpolation(
    mut q_transforms: Query<&mut Transform>,
    q_scene: Query<&JointMap, With<MainScene>>,
    motion_data: MotionData,
    motion_data_player_pair: ResMut<MotionDataPlayerPair>,
) {
    if motion_data_player_pair.is_playing == false {
        return;
    }

    let Some(motion_data) = motion_data.get() else {
        return;
    };

    let motion_poses = &motion_data.poses;

    let current_motion_data_player = &motion_data_player_pair.players[0];
    let next_motion_data_player = &motion_data_player_pair.players[1];

    let current_chunk_index = current_motion_data_player.chunk_index;
    let next_chunk_index = next_motion_data_player.chunk_index;

    let current_poses = motion_poses.get_poses_from_chunk(current_chunk_index);
    let next_poses = motion_poses.get_poses_from_chunk(next_chunk_index);

    let current_chunk_offset = motion_poses.chunk_offset_from_time(current_motion_data_player.time);
    let next_chunk_offset = motion_poses.chunk_offset_from_time(next_motion_data_player.time);

    let (Some(start_pose), Some(end_pose)) = (
        current_poses.get(current_chunk_offset),
        next_poses.get(next_chunk_offset),
    ) else {
        return;
    };

    let interpolation_factor = &motion_data_player_pair.interpolation_factor;

    // Calculate interpolated position and rotation from a joint.
    let calculate_trans_rot = |joint: &JointInfo| -> (Vec3, Quat) {
        let (start_pos, start_rot) = start_pose.get_pos_rot(joint);
        let (end_pos, end_rot) = end_pose.get_pos_rot(joint);

        let translation = Vec3::lerp(start_pos, end_pos, *interpolation_factor);
        let rotation = Quat::slerp(start_rot, end_rot, *interpolation_factor);

        (translation, rotation)
    };

    for joint_map in q_scene.iter() {
        let root_joint = &motion_data.joints()[0];

        if let Some(mut transform) = joint_map
            // Get root_joint transform.
            .get(root_joint.name())
            .and_then(|entity| q_transforms.get_mut(*entity).ok())
        {
            let (translation, rotation) = calculate_trans_rot(root_joint);

            transform.translation.y = translation.y;
            transform.rotation = rotation;
        }

        for joint in motion_data.joints().iter().skip(1) {
            if let Some(mut transform) = joint_map
                // Get joint transform.
                .get(joint.name())
                .and_then(|entity| q_transforms.get_mut(*entity).ok())
            {
                let (translation, rotation) = calculate_trans_rot(joint);

                transform.translation = joint.offset() + translation;
                transform.rotation = rotation;
            }
        }
    }
}

fn motion_data_player(
    mut q_transforms: Query<&mut Transform>,
    q_scene: Query<&JointMap, With<MainScene>>,
    motion_data: MotionData,
    mut motion_data_player_pair: ResMut<MotionDataPlayerPair>,
    time: Res<Time>,
) {
    // Return if playback is not active.
    if !motion_data_player_pair.is_playing {
        return;
    }

    // Retrieve the motion data.
    let Some(motion_data) = motion_data.get() else {
        return;
    };

    let motion_poses = &motion_data.poses;

    for (index, motion_player) in motion_data_player_pair.players.iter().enumerate() {
        let poses = motion_poses.get_poses_from_chunk(motion_player.chunk_index);
        let chunk_offset = motion_poses.chunk_offset_from_time(motion_player.time);

        // Get start and end poses for interpolation.
        let (Some(start_pose), Some(end_pose)) =
            (poses.get(chunk_offset), poses.get(chunk_offset + 1))
        else {
            return;
        };

        // Calculate time factor between poses for interpolation.
        let time_leak = motion_player.time - motion_poses.time_from_chunk_offset(chunk_offset);
        let time_factor = f32::clamp(time_leak / motion_poses.interval(), 0.0, 1.0);

        let calculate_trans_rot = |joint: &JointInfo| -> (Vec3, Quat) {
            let (start_pos, start_rot) = start_pose.get_pos_rot(joint);
            let (end_pos, end_rot) = end_pose.get_pos_rot(joint);

            let translation = Vec3::lerp(start_pos, end_pos, time_factor);
            let rotation = Quat::slerp(start_rot, end_rot, time_factor);

            (translation, rotation)
        };

        for joint_map in q_scene.iter() {
            let root_joint = &motion_data.joints()[0];

            if let Some(mut transform) = joint_map
                .get(root_joint.name())
                .and_then(|entity| q_transforms.get_mut(*entity).ok())
            {
                let (translation, rotation) = calculate_trans_rot(root_joint);
                transform.translation.y = translation.y;
                transform.rotation = rotation;
            }

            for joint in motion_data.joints().iter().skip(1) {
                if let Some(mut transform) = joint_map
                    .get(joint.name())
                    .and_then(|entity| q_transforms.get_mut(*entity).ok())
                {
                    let (translation, rotation) = calculate_trans_rot(joint);
                    transform.translation = joint.offset() + translation;
                    transform.rotation = rotation;
                }
            }
        }
    }

    for motion_player in motion_data_player_pair.players.iter_mut() {
        motion_player.time += time.delta_seconds();
    }
}

#[derive(Resource, Default, Debug)]
pub struct MotionDataPlayerPair {
    pub players: [MotionDataPlayer; 2],
    pub interpolation_factor: f32,
    pub is_playing: bool,
    pub pair_bool: bool,
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
}

impl MotionDataPlayerPair {
    pub fn jump_to_pose(&mut self, chunk_index: usize, time: f32, index: usize) {
        self.players[index].chunk_index = chunk_index;
        self.players[index].time = time;
    }
}
