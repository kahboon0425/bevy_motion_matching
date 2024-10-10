use bevy::prelude::*;
use bevy_bvh_anim::bvh_anim::ChannelType;
use bevy_bvh_anim::joint_traits::JointChannelTrait;

use crate::bvh_manager::bvh_player::BoneMap;
use crate::motion_data::motion_data_asset::{MotionDataAsset, Pose};
use crate::motion_data::{MotionData, MotionDataHandle};
use crate::player::PlayerMarker;
use crate::pose_matching::{apply_pose, match_pose};
use crate::scene_loader::MainScene;
use crate::trajectory::Trajectory;

pub struct NearestTrajectoryRetrieverPlugin;

impl Plugin for NearestTrajectoryRetrieverPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<BestTrajectory>()
            .add_systems(Startup, load_motion_data)
            .add_systems(Update, match_trajectory)
            .add_systems(Update, play_pose);
    }
}

pub fn load_motion_data(mut commands: Commands, asset_server: Res<AssetServer>) {
    let file_path = "motion_data/motion_data.json";
    let motion_data_handle = asset_server.load::<MotionDataAsset>(file_path);
    commands.insert_resource(MotionDataHandle(motion_data_handle));
}

#[derive(Event, Debug)]
pub struct BestTrajectory(NearestTrajectory);

pub fn match_trajectory(
    motion_data: MotionData,
    user_input_trajectory: Query<(&Trajectory, &Transform), With<PlayerMarker>>,
    mut q_transforms: Query<&mut Transform, (Without<MainScene>, Without<PlayerMarker>)>,
    mut main_character: Query<(&mut Transform, &BoneMap), (With<MainScene>, Without<PlayerMarker>)>,
    mut best_trajectory_event: EventWriter<BestTrajectory>,
    time: Res<Time>,
    mut time_passed: Local<f32>,
) {
    *time_passed += time.delta_seconds();

    if *time_passed >= 1.0 {
        if let Some(motion_data) = motion_data.get() {
            for (trajectory, transform) in user_input_trajectory.iter() {
                let nearest_trajectories =
                    find_nearest_trajectories::<1>(motion_data, trajectory, transform);
                println!("10 nearest trajectory: {:?}", nearest_trajectories);

                let mut smallest_pose_distance = f32::MAX;
                let mut best_pose: Vec<f32> = vec![];
                let mut best_trajectory_index = 0;

                // println!("Nearest Trajectory length: {}", nearest_trajectories.len());
                for (i, nearest_trajectory) in nearest_trajectories.iter().enumerate() {
                    if let Some(nearest_trajectory) = nearest_trajectory {
                        let (pose_distance, pose) = match_pose(
                            nearest_trajectory,
                            motion_data,
                            &mut q_transforms,
                            &mut main_character,
                        );

                        if pose_distance < smallest_pose_distance {
                            smallest_pose_distance = pose_distance;
                            best_pose = pose;
                            best_trajectory_index = i;
                        }
                    }
                }
                let best_trajectory = nearest_trajectories[best_trajectory_index].unwrap();
                println!("Best Pose Trajectory: {:?}", best_trajectory);
                best_trajectory_event.send(BestTrajectory(best_trajectory));

                apply_pose(
                    motion_data,
                    &mut q_transforms,
                    &mut main_character,
                    best_pose,
                );
            }
        }

        // Reset the timer
        *time_passed = 0.0
    }
}

pub fn play_pose(
    motion_data: MotionData,
    mut q_transforms: Query<&mut Transform, Without<MainScene>>,
    mut main_character: Query<(&mut Transform, &BoneMap), With<MainScene>>,
    mut best_trajectory_event: EventReader<BestTrajectory>,
    time: Res<Time>,
    mut local_time: Local<f32>,
) {
    if let Some(motion_data) = motion_data.get() {
        for (mut _scene_transform, bone_map) in main_character.iter_mut() {
            let joints = &motion_data.joints;

            for joint_data in joints.iter() {
                let bone_name = &joint_data.name;
                let Some(&bone_entity) = bone_map.0.get(bone_name) else {
                    continue;
                };

                let Ok(mut transform) = q_transforms.get_mut(bone_entity) else {
                    continue;
                };

                let mut current_pose_translation = Vec3::ZERO;
                let mut next_pose_translation = Vec3::ZERO;

                let mut current_pose_rotation = Vec3::ZERO;
                let mut next_pose_rotation = Vec3::ZERO;

                let mut interpolation_factor = 0.0;

                for pose_ref in &joint_data.pose_refs {
                    for best_trajectory in best_trajectory_event.read() {
                        let time = best_trajectory.0.chunk_offset as f32 * 0.16667;

                        let chunk_index = best_trajectory.0.chunk_index;
                        println!("Chunk Index: {}", chunk_index);

                        let frame_time = 0.016667;

                        // let (current_frame_index, interpolation_factor) =
                        //     get_poses(*local_time, motion_data, time);

                        let frame_count = motion_data.poses.get_poses(chunk_index).len();

                        // let frame_index = (time / frame_time).floor() as usize % frame_count;
                        // println!(
                        //     "Animation time: {}, frame_index, {}, test_animation_time: {}",
                        //     time, chunk_index, frame_index
                        // );

                        let current_frame_index = best_trajectory.0.chunk_offset;
                        let next_frame_index =
                            usize::clamp(current_frame_index + 1, 0, frame_count - 1);
                        println!(
                            "Current_frame_index: {}, Next_frame_index: {}",
                            current_frame_index, next_frame_index
                        );

                        // // println!("Frame count: {}", start_frame.len());

                        let poses = motion_data.poses.get_poses(chunk_index);

                        let start_pose = poses.get(current_frame_index).unwrap();
                        let next_pose = poses.get(next_frame_index).unwrap();
                        println!("Next pose: {:?}", next_pose);

                        let start_pose_value = start_pose[pose_ref.motion_index()];
                        let next_pose_value = next_pose[pose_ref.motion_index()];

                        interpolation_factor = (time % frame_time) / frame_time;
                        println!("Interpolation factor: {}", interpolation_factor);

                        match pose_ref.channel_type() {
                            ChannelType::RotationX => current_pose_rotation.x = start_pose_value,
                            ChannelType::RotationY => current_pose_rotation.y = start_pose_value,
                            ChannelType::RotationZ => current_pose_rotation.z = start_pose_value,
                            ChannelType::PositionX => current_pose_translation.x = start_pose_value,
                            ChannelType::PositionY => current_pose_translation.y = start_pose_value,
                            ChannelType::PositionZ => current_pose_translation.z = start_pose_value,
                        }

                        match pose_ref.channel_type() {
                            ChannelType::RotationX => next_pose_rotation.x = next_pose_value,
                            ChannelType::RotationY => next_pose_rotation.y = next_pose_value,
                            ChannelType::RotationZ => next_pose_rotation.z = next_pose_value,
                            ChannelType::PositionX => next_pose_translation.x = next_pose_value,
                            ChannelType::PositionY => next_pose_translation.y = next_pose_value,
                            ChannelType::PositionZ => next_pose_translation.z = next_pose_value,
                        }
                    }

                    let s_pose_rotation = Quat::from_euler(
                        EulerRot::XYZ,
                        current_pose_rotation.x.to_radians(),
                        current_pose_rotation.y.to_radians(),
                        current_pose_rotation.z.to_radians(),
                    );
                    let n_pose_rotation = Quat::from_euler(
                        EulerRot::XYZ,
                        next_pose_rotation.x.to_radians(),
                        next_pose_rotation.y.to_radians(),
                        next_pose_rotation.z.to_radians(),
                    );

                    let interp_translation = Vec3::lerp(
                        current_pose_translation,
                        next_pose_translation,
                        interpolation_factor,
                    );
                    let interp_rotation =
                        Quat::slerp(s_pose_rotation, n_pose_rotation, interpolation_factor);

                    transform.rotation = interp_rotation;

                    transform.translation = interp_translation + joint_data.offset;
                }
            }
        }
    }

    // *local_time += time.delta_seconds();
}

// pub fn get_poses(local_time: f32, bvh_data: &MotionDataAsset, current_time: f32) -> (usize, f32) {
//     let duration_per_frame = bvh_data.frame_time().as_secs_f32();

//     let total_animation_time = duration_per_frame * bvh_data.frames().len() as f32;

//     let animation_time = local_time % total_animation_time;

//     let frame_index =
//         (animation_time / duration_per_frame).floor() as usize % bvh_data.frames().len();

//     let interpolation_factor = (animation_time % duration_per_frame) / duration_per_frame;

//     (frame_index, interpolation_factor)
// }

#[derive(Clone, Copy, Debug)]
pub struct NearestTrajectory {
    /// Error distance from this trajectory to the trajecctory that is being compared to.
    pub distance: f32,
    /// Index pointing to the chunk that holds this trajectory.
    pub chunk_index: usize,
    /// Offset index into the chunk that holds this trajectory.
    pub chunk_offset: usize,
}

/// # Panic
///
/// Panic if `N` is 0.
pub fn find_nearest_trajectories<const N: usize>(
    motion_data: &MotionDataAsset,
    player_trajectory: &Trajectory,
    player_transform: &Transform,
) -> [Option<NearestTrajectory>; N] {
    assert!(
        N > 0,
        "Unable to find closest trajectory if the number of closest trajectory needed is 0."
    );

    let player_inv_matrix = player_transform.compute_matrix().inverse();
    let mut stack_count = 0;
    let mut nearest_trajectories_stack = [None::<NearestTrajectory>; N];

    let trajectories = &motion_data.trajectories;
    for (chunk_index, chunk) in trajectories.iter_chunk().enumerate() {
        let chunk_count = chunk.len();
        if chunk_count < 7 {
            // warn!("Chunk ({chunk_index}) has less than 7 trajectories.");
            continue;
        }

        // println!("Chunk Counttttttt: {}", chunk_count);
        // println!("Chunk Indexxxxxxx: {}", chunk_index);

        for chunk_offset in 0..chunk_count - 7 {
            let trajectory = &chunk[chunk_offset..chunk_offset + 7];

            // Center point of trajectory
            let inv_matrix = trajectory[3].inverse();

            let player_local_translations = player_trajectory
                .values
                .iter()
                .map(|player_trajectory| {
                    player_inv_matrix.transform_point3(Vec3::new(
                        player_trajectory.x,
                        0.0,
                        player_trajectory.y,
                    ))
                })
                .map(|v| v.xz())
                .collect::<Vec<_>>();

            let local_translations = trajectory
                .iter()
                .map(|trajectory| {
                    inv_matrix.transform_point3(trajectory.to_scale_rotation_translation().2)
                })
                .map(|v| v.xz())
                .collect::<Vec<_>>();

            let distance =
                calculate_trajectory_distance(&player_local_translations, &local_translations);

            if stack_count < N {
                // Stack not yet full, push into it
                nearest_trajectories_stack[stack_count] = Some(NearestTrajectory {
                    distance,
                    chunk_index,
                    chunk_offset,
                });
            } else if let Some(max_trajectory) = nearest_trajectories_stack[N - 1] {
                if distance < max_trajectory.distance {
                    nearest_trajectories_stack[N - 1] = Some(NearestTrajectory {
                        distance,
                        chunk_index,
                        chunk_offset,
                    })
                }
            }

            stack_count = usize::min(stack_count + 1, N);

            // Sort so that trajectories with the largest distance
            // is placed as the final element in the stack
            nearest_trajectories_stack.sort_by(|t0, t1| match (t0, t1) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(_), None) => std::cmp::Ordering::Less,
                (Some(t0), Some(t1)) => t0.distance.total_cmp(&t1.distance),
            });
        }
    }

    nearest_trajectories_stack
}

pub fn calculate_trajectory_distance(t1: &[Vec2], t2: &[Vec2]) -> f32 {
    // distance = sqrt((p1-q1)^2 + (p2-q2)^2)
    t1.iter()
        .zip(t2.iter())
        .map(|(p, traj)| (*p - *traj).length_squared())
        .sum::<f32>()
}
