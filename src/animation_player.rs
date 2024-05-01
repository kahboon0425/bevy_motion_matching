use bevy::{asset::LoadState, prelude::*, utils::hashbrown::HashMap};
use bvh_anim::{Bvh, Channel, Frame};

use crate::{
    animation_loader::BvhData,
    character_loader::{BvhToCharacter, MainCharacter},
};

pub struct AnimationPlayerPlugin;

impl Plugin for AnimationPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, match_bones)
            .add_systems(Update, draw_movement_arrows)
            .add_systems(Update, test)
            .insert_resource(HipTransforms::new())
            .add_event::<HipTransformsEvent>()
            .add_event::<TargetTimeEvent>()
            .add_systems(Update, store_bones);
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
pub struct BoneRotation(pub Quat);

#[derive(Component, Default, Debug)]
pub struct BoneMap(pub HashMap<String, Entity>);

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
pub struct TargetTimeEvent {
    pub time: f32,
}

pub fn store_bones(
    mut commands: Commands,
    q_character: Query<Entity, (With<MainCharacter>, Without<BoneMap>)>,
    q_names: Query<&Name>,
    children: Query<&Children>,
    bvh_to_character: ResMut<BvhToCharacter>,
) {
    if bvh_to_character.loaded == false {
        return;
    }

    for character_entity in q_character.iter() {
        let mut bone_map = BoneMap::default();

        for bone_entity in children.iter_descendants(character_entity) {
            if let Ok(name) = q_names.get(bone_entity) {
                let bone_name = name[6..].to_string();
                bone_map.0.insert(bone_name, bone_entity);
            }
        }

        commands.entity(character_entity).insert(bone_map);
    }
}

pub fn match_bones(
    q_bone_map: Query<&BoneMap, With<MainCharacter>>,
    mut q_transform: Query<(&mut Transform, &GlobalTransform), Without<MainCharacter>>,
    mut q_character: Query<&mut Transform, With<MainCharacter>>,
    bvh_data: Res<BvhData>,
    mut bvh_to_character: ResMut<BvhToCharacter>,
    mut hip_transforms: ResMut<HipTransforms>,
    server: Res<AssetServer>,
    mut event_writer: EventWriter<HipTransformsEvent>,
    time: Res<Time>,
    mut local_time: Local<f32>,
    mut event_reader: EventReader<TargetTimeEvent>,
) {
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

    for event in event_reader.read() {
        *local_time = event.time;
    }

    let (frame_index, interpolation_factor) = get_pose(*local_time, bvh_animation_data);

    let current_frame_index = frame_index;

    // Loop back to start if at the end
    let next_frame_index = (current_frame_index + 1) % bvh_animation_data.frames().len();

    if let (Some(current_frame), Some(next_frame)) = (
        bvh_animation_data.frames().nth(current_frame_index),
        bvh_animation_data.frames().nth(next_frame_index),
    ) {
        let current_frame = FrameData(current_frame);
        let next_frame = FrameData(next_frame);

        for joint in bvh_animation_data.joints() {
            let bone_names = joint.data().name().to_str().unwrap();
            for bone_map in q_bone_map.iter() {
                if let Some(&bone_entity) = bone_map.0.get(bone_names) {
                    if let Ok((mut transform, global_transform)) = q_transform.get_mut(bone_entity)
                    {
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

                        if bone_names == "Hips" {
                            for mut c_transform in q_character.iter_mut() {
                                c_transform.translation = interpolated_translation * 0.01;
                                c_transform.translation.y = 0.0;
                            }
                            // Store the current position as the previous for the left foot
                            hip_transforms.hip_previous_transform =
                                hip_transforms.hip_current_transform;
                            // Update the current position for the left foot
                            hip_transforms.hip_current_transform = global_transform.translation();
                        } else {
                            transform.translation = interpolated_translation;
                        }

                        event_writer.send(HipTransformsEvent {
                            current_transform: hip_transforms.hip_current_transform,
                            previous_transform: hip_transforms.hip_previous_transform,
                        });
                    }
                }
            }
        }
    }

    *local_time += time.delta_seconds();
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

pub fn get_pose(local_time: f32, bvh_data: &Bvh) -> (usize, f32) {
    let duration_per_frame = bvh_data.frame_time().as_secs_f32();

    let total_animation_time = duration_per_frame * bvh_data.frames().len() as f32;

    let animation_time = local_time % total_animation_time;

    let frame_index =
        (animation_time / duration_per_frame).floor() as usize % bvh_data.frames().len();

    let interpolation_factor = (animation_time % duration_per_frame) / duration_per_frame;

    (frame_index, interpolation_factor)
}

pub fn test(input: Res<ButtonInput<KeyCode>>, mut target_time_event: EventWriter<TargetTimeEvent>) {
    if input.just_pressed(KeyCode::Space) {
        target_time_event.send(TargetTimeEvent { time: 50.0 });
    }
}
