//! Play motion data based on events and resources.

use bevy::prelude::*;

use crate::transform2d::Transform2d;
use crate::{bvh_manager::bvh_player::JointMap, GameMode};
use crate::{BVH_SCALE_RATIO, LARGE_EPSILON};

use super::chunk::ChunkIterator;
use super::pose_data::{Pose, PoseData};
use super::MotionData;

pub(super) struct MotionPlayerPlugin;

impl Plugin for MotionPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                MotionPlayerSet::JumpToPose,
                MotionPlayerSet::ApplyPose,
                (
                    MotionPlayerSet::ApplyJointTransform,
                    MotionPlayerSet::ApplyRootTransform,
                ),
                MotionPlayerSet::Interpolate,
            )
                .chain(),
        );

        app.insert_resource(MotionPlayerConfig {
            interpolation_duration: 0.3333,
        })
        .add_event::<JumpToPose>()
        .add_systems(
            Update,
            (
                jump_to_pose.in_set(MotionPlayerSet::JumpToPose),
                apply_trajectory_pose.in_set(MotionPlayerSet::ApplyPose),
                pose_to_joint_transforms.in_set(MotionPlayerSet::ApplyJointTransform),
                apply_root_transform.in_set(MotionPlayerSet::ApplyRootTransform),
                (update_trajectory_pose_time, update_interpolation_factor)
                    .in_set(MotionPlayerSet::Interpolate),
                test.before(MotionPlayerSet::JumpToPose),
            )
                .run_if(in_state(GameMode::Play)),
        );
    }
}

/// Reset root and joint transforms to T-Pose.
fn init_pose() {}

fn test(
    q_entities: Query<Entity, With<TrajectoryPosePair>>,
    input: Res<ButtonInput<KeyCode>>,
    mut jump_evw: EventWriter<JumpToPose>,
    mut time: ResMut<Time<Virtual>>,
) {
    if input.pressed(KeyCode::ControlLeft) {
        time.set_relative_speed(0.2);
    } else if input.pressed(KeyCode::ShiftLeft) {
        time.set_relative_speed(2.0);
    } else {
        time.set_relative_speed(1.0);
    }

    if input.just_pressed(KeyCode::Space) {
        for entity in q_entities.iter() {
            jump_evw.send(JumpToPose(
                MotionPose {
                    chunk_index: 0,
                    time: 2.0,
                },
                entity,
            ));
        }
    }
}

fn apply_root_transform(
    motion_data: MotionData,
    mut q_motion_players: Query<(
        &MotionPlayer,
        &TrajectoryPosePair,
        &JointMap,
        &mut Transform2d,
    )>,
    mut q_transforms: Query<&mut Transform>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let Some(root_joint) = motion_data
        .get()
        // SAFETY: We assume there is a root joint.
        .map(|asset| asset.get_joint(0).unwrap())
    else {
        return;
    };

    for (motion_player, traj_pose_pair, joint_map, mut transform2d) in q_motion_players.iter_mut() {
        let Some(mut root_joint_transform) = joint_map
            .get(root_joint.name())
            .and_then(|e| q_transforms.get_mut(*e).ok())
        else {
            return;
        };

        let mut final_root_config = [None; 2];

        for i in 0..2 {
            if let Some(traj_pose) = &traj_pose_pair[i] {
                let root_transform2d = traj_pose.entity_root_transform2d;

                let (traj_root_pos, traj_root_rot) = traj_pose.traj_root_pos_rot;
                let (traj_pos, traj_rot) = traj_pose.pose.get_pos_rot(root_joint);

                // Offset from trajectory root to current trajcetory.
                let mut offset_pos = (traj_pos - traj_root_pos) * BVH_SCALE_RATIO;
                // traj_root_rot * offset_rot = traj_rot.
                let offset_rot = (traj_root_rot.inverse() * traj_rot).normalize();
                let offset_forward = offset_rot.mul_vec3(Vec3::Z).xz().normalize();
                let offset_forward_angle = f32::atan2(offset_forward.x, offset_forward.y);

                offset_pos = Quat::from_rotation_y(root_transform2d.angle).mul_vec3(offset_pos);

                let translation = root_transform2d.translation + offset_pos.xz();
                let angle = Quat::from_rotation_y(root_transform2d.angle + offset_forward_angle)
                    .to_scaled_axis()
                    .y;

                let local_y_pos = traj_pos.y;
                let local_xz_rot =
                    (Quat::from_rotation_y(offset_forward_angle).inverse() * traj_rot).normalize();

                final_root_config[i] = Some(RootConfig {
                    world_transform2d: Transform2d { translation, angle },
                    local_y_pos,
                    local_xz_rot,
                });
            }
        }

        let root_config = match (
            motion_player.interpolation_factor,
            final_root_config[0],
            final_root_config[1],
        ) {
            // First data only.
            (_, Some(root_config), None) | (0.0, Some(root_config), _) => root_config,
            // Second data only.
            (_, None, Some(root_config)) | (1.0, _, Some(root_config)) => root_config,
            (t, Some(root_config0), Some(root_config1)) => {
                RootConfig::lerp(root_config0, root_config1, t)
            }
            _ => continue,
        };

        *transform2d = root_config.world_transform2d;
        root_joint_transform.translation.y = root_config.local_y_pos;
        root_joint_transform.rotation = root_config.local_xz_rot;
    }

    #[derive(Clone, Copy)]
    struct RootConfig {
        world_transform2d: Transform2d,
        local_y_pos: f32,
        local_xz_rot: Quat,
    }

    impl RootConfig {
        fn lerp(self, rhs: Self, t: f32) -> Self {
            Self {
                world_transform2d: Transform2d {
                    translation: Vec2::lerp(
                        self.world_transform2d.translation,
                        rhs.world_transform2d.translation,
                        t,
                    ),
                    angle: Quat::slerp(
                        Quat::from_rotation_y(self.world_transform2d.angle),
                        Quat::from_rotation_y(rhs.world_transform2d.angle),
                        t,
                    )
                    .to_scaled_axis()
                    .y,
                },
                local_y_pos: f32::lerp(self.local_y_pos, rhs.local_y_pos, t),
                local_xz_rot: Quat::slerp(self.local_xz_rot, rhs.local_xz_rot, t),
            }
        }
    }
}

/// Handle the [`JumpToPose`] event.
fn jump_to_pose(
    motion_data: MotionData,
    mut jump_evr: EventReader<JumpToPose>,
    mut q_motion_players: Query<(&mut MotionPlayer, &mut TrajectoryPosePair, &Transform2d)>,
) {
    let Some((pose_data, root_joint)) = motion_data
        .get()
        // SAFETY: We assume there is a root joint.
        .map(|asset| (&asset.pose_data, asset.get_joint(0).unwrap()))
    else {
        return;
    };

    for JumpToPose(motion_pose, entity) in jump_evr.read() {
        let Ok((mut motion_player, mut traj_pose_pair, transform2d)) =
            q_motion_players.get_mut(*entity)
        else {
            continue;
        };

        motion_player.switch_target_index();
        let Some(pose) = motion_pose.get_pose(pose_data) else {
            continue;
        };

        let index = motion_player.target_frame_index;
        traj_pose_pair[index] = Some(TrajectoryPose {
            motion_pose: *motion_pose,
            traj_root_pos_rot: pose.get_pos_rot(root_joint),
            entity_root_transform2d: *transform2d,
            pose,
            elapsed_time: 0.0,
        });
    }
}

fn update_interpolation_factor(
    mut q_motion_players: Query<&mut MotionPlayer>,
    time: Res<Time>,
    motion_player_config: Res<MotionPlayerConfig>,
) {
    assert!(
        motion_player_config.interpolation_duration > 0.0,
        "Interpolation duration cannot be 0 or below!"
    );

    for mut motion_player in q_motion_players.iter_mut() {
        motion_player.update_interpolation_factor(
            time.delta_seconds() / motion_player_config.interpolation_duration,
        );
    }
}

fn update_trajectory_pose_time(
    mut q_traj_pose_pairs: Query<&mut TrajectoryPosePair>,
    time: Res<Time>,
) {
    for mut traj_pose_pair in q_traj_pose_pairs.iter_mut() {
        for traj_pose in traj_pose_pair.iter_mut().filter_map(Some).flatten() {
            traj_pose.update_time(time.delta_seconds())
        }
    }
}

fn apply_trajectory_pose(
    motion_data: MotionData,
    mut q_traj_pose_pairs: Query<&mut TrajectoryPosePair>,
) {
    let Some(pose_data) = motion_data.get().map(|asset| &asset.pose_data) else {
        return;
    };

    for mut traj_pose_pair in q_traj_pose_pairs.iter_mut() {
        for traj_pose in traj_pose_pair.iter_mut().filter_map(Some).flatten() {
            traj_pose.try_apply_pose(pose_data);
        }
    }
}

/// Apply pose data to joint transforms.
/// Note: This does not apply the root transform.
fn pose_to_joint_transforms(
    motion_data: MotionData,
    q_motion_players: Query<(&TrajectoryPosePair, &MotionPlayer, &JointMap)>,
    mut q_transforms: Query<&mut Transform>,
) {
    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    for (traj_pose_pair, motion_player, joint_map) in q_motion_players.iter() {
        let Some(pose) = traj_pose_pair.get_interpolated_pose(motion_player.interpolation_factor)
        else {
            return;
        };

        for joint in motion_asset.joints().iter().skip(1) {
            if let Some(mut transform) = joint_map
                .get(joint.name())
                .and_then(|entity| q_transforms.get_mut(*entity).ok())
            {
                let (pos, rot) = pose.get_pos_rot(joint);
                transform.translation = joint.offset() + pos;
                transform.rotation = rot;
            }
        }
    }
}

#[derive(SystemSet, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum MotionPlayerSet {
    /// Handles [`JumpToPose`] event.
    JumpToPose,
    /// Performs [`TrajectoryPose::try_apply_pose`].
    ApplyPose,
    /// Apply interpolated pose from [`TrajectoryPosePair`] to their respective joint transforms.
    ApplyJointTransform,
    /// Apply transform to root joint.
    ApplyRootTransform,
    Interpolate,
}

#[derive(Event, Debug)]
pub struct JumpToPose(pub MotionPose, pub Entity);

#[derive(Bundle, Default)]
pub struct MotionPlayerBundle {
    pub motion_player: MotionPlayer,
    pub traj_pose_pair: TrajectoryPosePair,
}

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct TrajectoryPosePair([Option<TrajectoryPose>; 2]);

impl TrajectoryPosePair {
    pub fn get_interpolated_pose(&self, factor: f32) -> Option<Pose> {
        let pose0 = self[0].as_ref().map(|traj_pose| &traj_pose.pose);
        let pose1 = self[1].as_ref().map(|traj_pose| &traj_pose.pose);

        match (factor, pose0, pose1) {
            (_, Some(pose), None) | (0.0, Some(pose), _) => Some(pose.clone()),
            (_, None, Some(pose)) | (1.0, _, Some(pose)) => Some(pose.clone()),
            (t, Some(pose0), Some(pose1)) => Some(Pose::lerp(pose0, pose1, t)),
            _ => None,
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct MotionPlayer {
    /// Interpolation factor between [`Self::motion_poses`].
    interpolation_factor: f32,
    /// The target index of [`Self::motion_poses`].
    /// Also denotes which direction [`Self::interpolation_factor`] is going towards.
    target_frame_index: usize,
}

#[derive(Resource, Debug)]
pub struct MotionPlayerConfig {
    /// Duration for [`Self::interpolation_factor`] to go between 0 and 1.
    interpolation_duration: f32,
}

impl MotionPlayer {
    fn switch_target_index(&mut self) {
        self.target_frame_index = (self.target_frame_index + 1) % 2;
    }

    /// Update [`Self::interpolation_factor`] based on [`Self::interpolation_duration`].
    fn update_interpolation_factor(&mut self, delta_factor: f32) {
        match self.target_frame_index {
            0 => {
                self.interpolation_factor = f32::max(0.0, self.interpolation_factor - delta_factor);
            }
            1 => {
                self.interpolation_factor = f32::min(1.0, self.interpolation_factor + delta_factor);
            }
            x => {
                error!("Target frame index of MotionPlayer is neither 0 nor 1! It's {x}...");
            }
        }
    }
}

/// A pose frame inside [`PoseData`].
#[derive(Debug, Clone, Copy)]
pub struct MotionPose {
    /// The chunk index in [`PoseData`].
    pub chunk_index: usize,
    /// Duration in terms of seconds inside the [`Self::chunk_index`].
    pub time: f32,
}

impl MotionPose {
    /// Get an interpolated pose from [`PoseData`].
    ///
    /// Returns [`None`] when [`Self::chunk_index`] is invalid.
    #[must_use]
    pub fn get_pose(&self, pose_data: &PoseData) -> Option<Pose> {
        let interval_time = pose_data.interval_time();

        let poses = pose_data.get_chunk(self.chunk_index)?;

        // 2 poses is a segment, so we need to deduct by 1.
        let total_duration = interval_time * (poses.len().saturating_sub(1)) as f32;

        // Make sure it's not above the final frame.
        // (With an EPSILON error away :D)
        let time = f32::min(self.time, total_duration - LARGE_EPSILON);
        // let time = f32::min(self.time, total_duration - f32::EPSILON);

        // Interpolate between 2 surrounding frame.
        let start = (time / interval_time) as usize;
        let end = start + 1;

        // Time distance between start frame and current time stamp.
        let leak = time - start as f32 * interval_time;
        // Interpolation factor between start and end pose.
        let factor = leak / interval_time;

        let start_pose = &poses[start];
        let end_pose = &poses[end];

        Some(Pose::lerp(start_pose, end_pose, factor))
    }
}

#[derive(Debug)]
pub struct TrajectoryPose {
    motion_pose: MotionPose,
    /// The matrix of the pose's root joint where the trajectory starts.
    traj_root_pos_rot: (Vec3, Quat),
    /// The matrix of the entity's root joint where the trajectory starts.
    entity_root_transform2d: Transform2d,
    pose: Pose,
    /// Time passed since [`Self::motion_pose`] was set.
    elapsed_time: f32,
}

impl TrajectoryPose {
    /// Apply pose from [`Self::motion_pose`] to [`Self::pose`] if possible. (See [`MotionPose`]).
    /// Returns true if successful and vice versa.
    pub fn try_apply_pose(&mut self, pose_data: &PoseData) -> bool {
        if let Some(pose) = self.motion_pose.get_pose(pose_data) {
            self.pose = pose;
            return true;
        }

        false
    }

    /// Loop [`Self::motion_pose`] time if possible.
    /// Returns true if successful and vice versa.
    ///
    /// Note: This does not apply the pose itself. (See [`Self::try_apply_pose`])
    pub fn try_loop_pose(&mut self, pose_data: &PoseData) -> bool {
        if pose_data.is_chunk_loopable(self.motion_pose.chunk_index) != Some(true) {
            return false;
        }

        // SAFETY: Already checked above.
        let poses = pose_data.get_chunk_unchecked(self.motion_pose.chunk_index);
        let duration = pose_data.interval_time() * poses.len().saturating_sub(1) as f32;

        // Loop time.
        self.motion_pose.time %= duration;

        true
    }

    /// Increase [`Self::elapsed_time`] and [`Self::motion_pose`] time.
    pub fn update_time(&mut self, delta_seconds: f32) {
        self.elapsed_time += delta_seconds;
        self.motion_pose.time += delta_seconds;
    }
}

fn remove_y_rotation(quaternion: Quat) -> Quat {
    // Decompose the quaternion into forward and up vectors
    let forward = quaternion.mul_vec3(Vec3::Z).normalize();

    // Create a new quaternion that aligns the forward vector while keeping it horizontal
    Quat::from_rotation_arc(forward, Vec3::new(forward.x, 0.0, forward.z).normalize())
}
