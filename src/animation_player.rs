use bevy::{asset::LoadState, prelude::*};
use bvh_anim::{Bvh, Channel, Frame};

use crate::{animation_loader::BvhData, character_loader::BvhToCharacter};

pub struct AnimationPlayerPlugin;

impl Plugin for AnimationPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (match_bones, draw_movement_arrows, query_pose));
        app.insert_resource(HipTransforms::new());
        app.insert_resource(CustomTime { time: 30.0 });
        app.add_event::<HipTransformsEvent>();
    }
}

#[derive(Debug)]
pub struct FrameData<'a>(pub &'a Frame);

impl<'a> FrameData<'a> {
    pub fn get_rotation(&self, channels: &[Channel]) -> Quat {
        Quat::from_euler(
            EulerRot::ZYX,
            self.0[&channels[0]].to_radians(),
            self.0[&channels[1]].to_radians(),
            self.0[&channels[2]].to_radians(),
        )
    }

    pub fn get_translation_rotation(&self, channels: &[Channel]) -> (Vec3, Quat) {
        (
            Vec3::new(
                self.0[&channels[0]],
                self.0[&channels[1]],
                self.0[&channels[2]],
            ),
            Quat::from_euler(
                EulerRot::ZYX,
                self.0[&channels[3]].to_radians(),
                self.0[&channels[4]].to_radians(),
                self.0[&channels[5]].to_radians(),
            ),
        )
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

#[derive(Resource)]
pub struct CustomTime {
    pub time: f32,
}

pub fn match_bones(
    mut commands: Commands,
    mut q_names: Query<(Entity, &Name, &mut Transform, &GlobalTransform)>,
    bvh_data: Res<BvhData>,
    mut bvh_to_character: ResMut<BvhToCharacter>,
    mut hip_transforms: ResMut<HipTransforms>,
    server: Res<AssetServer>,
    mut event_writer: EventWriter<HipTransformsEvent>,
    time: Res<Time>,
    custom_time: Res<CustomTime>,
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

    let bvh_animation_data = bvh_data.get_bvh_animation_data(1);

    let (frame_index, interpolation_factor) = get_pose(custom_time, time, bvh_animation_data);

    let current_frame_index = frame_index;

    // Loop back to start if at the end
    let next_frame_index = (current_frame_index + 1) % bvh_animation_data.frames().len();

    if let (Some(current_frame), Some(next_frame)) = (
        bvh_animation_data.frames().nth(current_frame_index),
        bvh_animation_data.frames().nth(next_frame_index),
    ) {
        let current_frame = FrameData(current_frame);
        let next_frame = FrameData(next_frame);

        println!("Current Frame{:?}", current_frame);
        println!("Next Frame{:?}", next_frame);

        for (entity, name, mut transform, global_transform) in q_names.iter_mut() {
            let bone_name = &name.as_str()[6..];

            let mut joint_index: usize = 0;

            for joint in bvh_animation_data.joints() {
                if bone_name == joint.data().name() {
                    commands.entity(entity).insert(BoneIndex(joint_index));

                    let offset = joint.data().offset();

                    let mut current_translation = Vec3::new(offset.x, offset.y, offset.z);
                    let mut next_translation = Vec3::new(offset.x, offset.y, offset.z);

                    let channels = joint.data().channels();

                    let current_rotation;
                    let next_rotation;

                    if channels.len() == 3 {
                        current_rotation = current_frame.get_rotation(channels);
                        next_rotation = next_frame.get_rotation(channels);
                    } else {
                        let current_offset;
                        let next_offset;
                        (current_offset, current_rotation) =
                            current_frame.get_translation_rotation(channels);
                        (next_offset, next_rotation) =
                            next_frame.get_translation_rotation(channels);

                        current_translation += current_offset;
                        next_translation += next_offset;
                    }

                    let interpolated_translation =
                        Vec3::lerp(current_translation, next_translation, interpolation_factor);

                    let interpolated_rotation =
                        Quat::slerp(current_rotation, next_rotation, interpolation_factor);

                    transform.rotation = interpolated_rotation;
                    transform.translation = interpolated_translation;

                    if bone_name == "Hips" {
                        // Store the current position as the previous for the left foot
                        hip_transforms.hip_previous_transform =
                            hip_transforms.hip_current_transform;
                        // Update the current position for the left foot
                        hip_transforms.hip_current_transform = global_transform.translation();
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

pub fn get_pose(custom_time: Res<CustomTime>, time: Res<Time>, bvh_data: &Bvh) -> (usize, f32) {
    let duration_per_frame = bvh_data.frame_time().as_secs_f32();

    println!("Frame time: {:?}", bvh_data.frame_time()); //33.33ms
    println!("Frame time in seconds: {:?}", duration_per_frame); // 0.033333

    // return how much time has advanced since startup
    let elapsed_time = time.elapsed_seconds() as f32;
    println!("Elapsed Time: {}", elapsed_time);

    let total_animation_time = duration_per_frame * bvh_data.frames().len() as f32;

    // current animation time
    let animation_time = elapsed_time % total_animation_time;

    // get animation data at specific time
    let targeted_time = (custom_time.time + elapsed_time) % total_animation_time;

    // % ensure looping through frame
    let frame_index =
        (animation_time / duration_per_frame).floor() as usize % bvh_data.frames().len();

    let factor = (animation_time % duration_per_frame) / duration_per_frame;

    (frame_index, factor)
}

pub fn query_pose(mut custom_time: ResMut<CustomTime>) {
    custom_time.time = 100.0;
}
