//! Play motion data based on events and resources.

use bevy::prelude::*;

use crate::bvh_manager::bvh_player::JointMap;
use crate::scene_loader::MainScene;

use super::motion_data_asset::MotionDataAsset;

pub(super) struct MotionDataPlayerPlugin;

impl Plugin for MotionDataPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            motion_data_player.run_if(resource_exists::<MotionDataPlayer>),
        );
    }
}

fn motion_data_player(
    mut q_transforms: Query<&mut Transform>,
    q_scene: Query<&JointMap, With<MainScene>>,
    mut motion_player: ResMut<MotionDataPlayer>,
    motion_assets: Res<Assets<MotionDataAsset>>,
    time: Res<Time>,
) {
    if motion_player.is_playing == false {
        return;
    }

    let Some(motion_data) = motion_assets.get(&motion_player.motion_data) else {
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

    for joint_map in q_scene.iter() {
        for joint in motion_data.joints() {
            let name = joint.name();

            // Get joint transform.
            let Some(mut transform) = joint_map
                .get(name)
                .and_then(|entity| q_transforms.get_mut(*entity).ok())
            else {
                continue;
            };

            let (start_pos, start_rot) = start_pose.get_pos_rot(joint);
            let (end_pos, end_rot) = end_pose.get_pos_rot(joint);

            transform.translation = joint.offset() + Vec3::lerp(start_pos, end_pos, time_factor);
            transform.rotation = Quat::slerp(start_rot, end_rot, time_factor);
        }
    }

    motion_player.time += time.delta_seconds();
}

/// Insert this resource to start playing motion data.
#[derive(Resource, Default, Debug)]
pub struct MotionDataPlayer {
    /// The referenced asset to play.
    pub motion_data: Handle<MotionDataAsset>,
    /// The current chunk in the motion data asset to play.
    ///
    /// Get chunk using [`Poses::get_poses_from_chunk`][get_poses_from_chunk].
    ///
    /// [get_poses_from_chunk]: crate::motion_data::motion_data_asset::Poses::get_poses_from_chunk
    pub chunk_index: usize,
    /// Duration in terms of seconds inside the [`Self::chunk_index`].
    pub time: f32,
    /// Is the player currently playing?
    /// Set to false to pause the player and vice versa.
    pub is_playing: bool,
}
