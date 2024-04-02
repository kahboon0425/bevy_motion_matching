use bevy::{asset::LoadState, prelude::*};
use bvh_anim::Bvh;

use crate::{animation_loader::BvhData, character_loader::BvhToCharacter};

pub struct AnimationPlayerPlugin;

impl Plugin for AnimationPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (match_bones, draw_movement_arrows, animation_time));
        app.insert_resource(HipTransforms::new());
        app.insert_resource(Time { time: 30.0 });
        app.add_event::<HipTransformsEvent>();
        app.add_event::<FrameUpdateEvent>();
    }
}

#[derive(Component)]
pub struct BoneIndex(pub usize);

#[derive(Component)]
pub struct BoneRotation(pub Quat);

#[derive(Resource)]
pub struct HipTransforms {
    pub hip_current_transform: Vec3,
    pub hip_previous_transform: Vec3,
}

impl HipTransforms {
    pub fn new() -> Self {
        Self {
            hip_current_transform: Vec3::ZERO,
            hip_previous_transform: Vec3::ZERO,
        }
    }
}

#[derive(Event)]
pub struct HipTransformsEvent {
    pub current_transform: Vec3,
    pub previous_transform: Vec3,
}

#[derive(Event)]
pub struct FrameUpdateEvent {
    pub time: f32,
}

#[derive(Resource)]
pub struct Time {
    pub time: f32,
}

pub fn match_bones(
    mut commands: Commands,
    mut q_names: Query<(Entity, &Name, &mut Transform, &GlobalTransform)>,
    mut bvh_data: ResMut<BvhData>,
    mut bvh_to_character: ResMut<BvhToCharacter>,
    mut hip_transforms: ResMut<HipTransforms>,
    server: Res<AssetServer>,
    mut event_writer: EventWriter<HipTransformsEvent>,
    time: ResMut<Time>,
    event_reader: EventReader<FrameUpdateEvent>,
) {
    // if bvh_to_character.loaded == true {
    //     return;
    // }

    println!("Checking load state");
    let load_state: LoadState = server
        .get_load_state(bvh_to_character.scene_handle.clone())
        .unwrap();

    match load_state {
        LoadState::Loaded => {
            bvh_to_character.loaded = true;
        }
        _ => {}
    }

    if bvh_to_character.loaded == false {
        return;
    }

    let interpolation_factor = get_pose(time, &mut bvh_data, event_reader);

    if let Some(bvh_vec) = &bvh_data.bvh_animation {
        let bvh: Bvh = bvh_vec[1].clone();

        let current_frame_index = bvh_data.current_frame_index;

        // Loop back to start if at the end
        let next_frame_index = (current_frame_index + 1) % bvh.frames().len();

        if current_frame_index < bvh.frames().len() {
            if let (Some(current_frame), Some(next_frame)) = (
                bvh.frames().nth(current_frame_index),
                bvh.frames().nth(next_frame_index),
            ) {
                println!("Current Frame{:?}", current_frame);
                println!("Next Frame{:?}", next_frame);

                for (entity, name, mut transform, global_transform) in q_names.iter_mut() {
                    let bone_name = &name.as_str()[6..];

                    let mut joint_index: usize = 0;

                    for joint in bvh.joints() {
                        if bone_name == joint.data().name() {
                            // if bone_name == "Hips" {
                            //     continue;
                            // }

                            // println!("{:#?} = {:#?}", bone_name, joint.data().name());

                            commands.entity(entity).insert(BoneIndex(joint_index));

                            let mut current_offset_x = joint.data().offset().x;
                            let mut current_offset_y = joint.data().offset().y;
                            let mut current_offset_z = joint.data().offset().z;

                            let mut next_offset_x = joint.data().offset().x;
                            let mut next_offset_y = joint.data().offset().y;
                            let mut next_offset_z = joint.data().offset().z;

                            let current_rotation0: f32;
                            let current_rotation1: f32;
                            let current_rotation2: f32;

                            let next_rotation0: f32;
                            let next_rotation1: f32;
                            let next_rotation2: f32;

                            if joint.data().channels().len() == 3 {
                                current_rotation0 = current_frame[&joint.data().channels()[0]];
                                current_rotation1 = current_frame[&joint.data().channels()[1]];
                                current_rotation2 = current_frame[&joint.data().channels()[2]];

                                next_rotation0 = next_frame[&joint.data().channels()[0]];
                                next_rotation1 = next_frame[&joint.data().channels()[1]];
                                next_rotation2 = next_frame[&joint.data().channels()[2]];
                            } else {
                                current_offset_x += current_frame[&joint.data().channels()[0]];
                                current_offset_y += current_frame[&joint.data().channels()[1]];
                                current_offset_z += current_frame[&joint.data().channels()[2]];

                                next_offset_x += next_frame[&joint.data().channels()[0]];
                                next_offset_y += next_frame[&joint.data().channels()[1]];
                                next_offset_z += next_frame[&joint.data().channels()[2]];

                                current_rotation0 = current_frame[&joint.data().channels()[3]];
                                current_rotation1 = current_frame[&joint.data().channels()[4]];
                                current_rotation2 = current_frame[&joint.data().channels()[5]];

                                next_rotation0 = next_frame[&joint.data().channels()[3]];
                                next_rotation1 = next_frame[&joint.data().channels()[4]];
                                next_rotation2 = next_frame[&joint.data().channels()[5]];
                            }

                            let current_rotation = Quat::from_euler(
                                EulerRot::ZYX,
                                current_rotation0.to_radians(),
                                current_rotation1.to_radians(),
                                current_rotation2.to_radians(),
                            );

                            let next_rotation = Quat::from_euler(
                                EulerRot::ZYX,
                                next_rotation0.to_radians(),
                                next_rotation1.to_radians(),
                                next_rotation2.to_radians(),
                            );

                            let current_position =
                                Vec3::new(current_offset_x, current_offset_y, current_offset_z);

                            let next_position =
                                Vec3::new(next_offset_x, next_offset_y, next_offset_z);

                            let interpolated_position =
                                current_position.lerp(next_position, interpolation_factor);

                            let interpolated_rotation =
                                current_rotation.slerp(next_rotation, interpolation_factor);

                            transform.rotation = interpolated_rotation;

                            // println!("origin transform: {:?}", transform.translation);
                            // println!("bvh offset: {}, {}, {}", offset_x, offset_y, offset_z);

                            transform.translation = interpolated_position;

                            if bone_name == "Hips" {
                                // Store the current position as the previous for the left foot
                                hip_transforms.hip_previous_transform =
                                    hip_transforms.hip_current_transform;
                                // Update the current position for the left foot
                                hip_transforms.hip_current_transform =
                                    global_transform.translation();
                            }
                            event_writer.send(HipTransformsEvent {
                                current_transform: hip_transforms.hip_current_transform,
                                previous_transform: hip_transforms.hip_previous_transform,
                            });
                        }

                        joint_index += 1;
                    }
                }
            }

            bvh_data.current_frame_index += 1;

            println!(
                "Current Frame Index Resources: {}",
                bvh_data.current_frame_index
            );

            if bvh_data.current_frame_index >= bvh.frames().len() {
                bvh_data.current_frame_index = 0;
            }
        }
    } else {
        println!("BVH data not available");
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct MyRoundGizmos {}

pub fn draw_movement_arrows(mut gizmos: Gizmos, mut event_reader: EventReader<HipTransformsEvent>) {
    for event in event_reader.read() {
        if event.previous_transform != event.current_transform {
            gizmos.arrow(
                event.previous_transform,
                event.current_transform,
                Color::YELLOW,
            );
        }
    }
}

pub fn get_pose(
    mut time: ResMut<Time>,
    bvh_data: &mut ResMut<BvhData>,
    mut event_reader: EventReader<FrameUpdateEvent>,
) -> f32 {
    let duration_per_frame = 0.033333;
    // let total_duration = num_frames as f32 * duration_per_frame;
    let mut frame_index: usize = (time.time / duration_per_frame).floor() as usize;

    println!("Current Frame Index: {}", frame_index);

    for event in event_reader.read() {
        if time.time != event.time {
            time.time = event.time;
            frame_index = (time.time / duration_per_frame).floor() as usize;
            bvh_data.current_frame_index = frame_index;
        }
    }
    // Find how much time has passed since the last frame (finds the remainder)
    // Calculate how far this is through the frame (divides the remainder)
    let factor: f32 = (time.time % duration_per_frame) / duration_per_frame;

    return factor;
}

pub fn animation_time(mut event_writer: EventWriter<FrameUpdateEvent>) {
    event_writer.send(FrameUpdateEvent { time: 30.0 });
}
