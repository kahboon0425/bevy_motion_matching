//! Play motion data based on events and resources.

use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::scene_loader::MainScene;
use crate::{bvh_manager::bvh_player::JointMap, GameMode};

use super::motion_data_asset::JointInfo;
use super::{MotionData, MotionDataHandle};

pub(super) struct MotionDataPlayerPlugin;

impl Plugin for MotionDataPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MotionDataPlayerPair>()
            .init_resource::<MotionDataPlayer>() // TODO: Remove this as a resource
            .add_systems(
                Update,
                motion_data_player.run_if(resource_exists::<MotionDataHandle>),
            )
            .add_systems(
                OnEnter(GameMode::Play),
                |mut player: ResMut<MotionDataPlayer>| {
                    player.is_playing = true;
                },
            )
            .add_systems(
                OnExit(GameMode::Play),
                |mut player: ResMut<MotionDataPlayer>| {
                    player.is_playing = false;
                },
            );
    }
}

fn motion_data_player(
    mut q_transforms: Query<&mut Transform>,
    q_scene: Query<&JointMap, With<MainScene>>,
    motion_data: MotionData,
    mut motion_player: ResMut<MotionDataPlayer>,
    time: Res<Time>,
) {
    if motion_player.is_playing == false {
        return;
    }

    let Some(motion_data) = motion_data.get() else {
        return;
    };

    let motion_poses = &motion_data.poses;
    let poses = motion_poses.get_poses_from_chunk(motion_player.chunk_index);
    let chunk_offset = motion_poses.chunk_offset_from_time(motion_player.time);

    let (Some(start_pose), Some(end_pose)) = (poses.get(chunk_offset), poses.get(chunk_offset + 1))
    else {
        return;
    };

    // Calculate time factor between 2 poses so that we can interpolate between them.
    let time_leak = motion_player.time - motion_poses.time_from_chunk_offset(chunk_offset);
    let time_factor = f32::clamp(time_leak / motion_poses.interval(), 0.0, 1.0);

    // Calculate interpolated position and rotation from a joint.
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

    motion_player.time += time.delta_seconds();
}

/// Maps joint name to their respective transform.
#[derive(Resource, Default, Debug, Clone, Deref, DerefMut)]
pub struct JointTransformMaps(pub [HashMap<String, Transform>; 2]);

#[derive(Resource, Default, Debug)]
pub struct MotionDataPlayerPair {
    pub joint_transform_maps: [HashMap<String, Transform>; 2],
    pub players: [MotionDataPlayer; 2],
    pub interpolation_factor: f32,
    pub is_playing: bool,
    pub pair_bool: bool,
}

// impl MotionDataPlayerPair {
//     pub fn get_player(&self, index: usize) -> (&HashMap<String, Transform>, &MotionDataPlayer) {
//         (&self.joint_transform_maps[index], &self.players[index])
//     }
// }

// TODO: Remove this as a resource
/// Insert this resource to start playing motion data.
#[derive(Resource, Default, Debug)]
pub struct MotionDataPlayer {
    /// The current chunk in the motion data asset to play.
    ///
    /// Get poses using [`Poses::get_poses_from_chunk`][get_poses_from_chunk].
    ///
    /// [get_poses_from_chunk]: crate::motion_data::motion_data_asset::Poses::get_poses_from_chunk
    pub chunk_index: usize,
    /// Duration in terms of seconds inside the [`Self::chunk_index`].
    pub time: f32,
    // TODO: Remove this
    /// Is the player currently playing?
    /// Set to false to pause the player and vice versa.
    pub is_playing: bool,
}

impl MotionDataPlayer {
    pub fn jump_to_pose(&mut self, chunk_index: usize, time: f32) {
        self.chunk_index = chunk_index;
        self.time = time;
    }
}
