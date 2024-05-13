use bevy::{
    asset::{DependencyLoadState, LoadState, RecursiveDependencyLoadState},
    prelude::*,
    utils::hashbrown::HashMap,
};
use bvh_anim::{Bvh, Channel, Frame};

use crate::{bvh_asset::BvhAsset, scene_loader::MainScene};

pub struct BvhPlayerPlugin;

impl Plugin for BvhPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedBvhAsset>()
            .add_event::<TargetTimeEvent>()
            .add_systems(Update, generate_bone_map)
            .add_systems(Update, bvh_player);
    }
}

#[allow(clippy::type_complexity)]
pub fn generate_bone_map(
    mut commands: Commands,
    q_character: Query<(Entity, &Handle<Scene>), (With<MainScene>, Without<BoneMap>)>,
    q_names: Query<&Name>,
    children: Query<&Children>,
    server: Res<AssetServer>,
    mut asset_loaded: Local<bool>,
) {
    let Ok((entity, scene_handle)) = q_character.get_single() else {
        return;
    };

    let Some(load_states) = server.get_load_states(scene_handle) else {
        return;
    };

    if *asset_loaded {
        let mut bone_map = BoneHashMap::default();

        for bone_entity in children.iter_descendants(entity) {
            if let Ok(name) = q_names.get(bone_entity) {
                let bone_name = name[6..].to_string();
                bone_map.insert(bone_name, bone_entity);
            }
        }

        commands.entity(entity).insert(BoneMap(bone_map));
    }

    if matches!(
        load_states,
        (
            LoadState::Loaded,
            DependencyLoadState::Loaded,
            RecursiveDependencyLoadState::Loaded
        )
    ) {
        // Notify to load asset in the next frame
        // Somehow children will not be present at the loaded frame
        *asset_loaded = true;
    }
}

pub fn bvh_player(
    mut q_transforms: Query<&mut Transform, Without<MainScene>>,
    mut q_scene: Query<(&mut Transform, &BoneMap), With<MainScene>>,
    mut event_reader: EventReader<TargetTimeEvent>,
    time: Res<Time>,
    selected_bvh_asset: Res<SelectedBvhAsset>,
    bvh_asset: Res<Assets<BvhAsset>>,
    mut local_time: Local<f32>,
) {
    let Some(BvhAsset(bvh)) = bvh_asset.get(selected_bvh_asset.0) else {
        return;
    };

    for event in event_reader.read() {
        *local_time = event.time;
    }

    let (current_frame_index, interpolation_factor) = get_pose(*local_time, bvh);
    let next_frame_index = usize::clamp(current_frame_index + 1, 0, bvh.frames().len() - 1);

    let (Some(current_frame), Some(next_frame)) = (
        bvh.frames().nth(current_frame_index),
        bvh.frames().nth(next_frame_index),
    ) else {
        return;
    };

    let current_frame = FrameData(current_frame);
    let next_frame = FrameData(next_frame);

    for (mut scene_transform, bone_map) in q_scene.iter_mut() {
        for joint in bvh.joints() {
            let bone_name = joint.data().name().to_str().unwrap();
            // Get bone transform
            let Some(&bone_entity) = bone_map.0.get(bone_name) else {
                continue;
            };
            let Ok(mut bone_transform) = q_transforms.get_mut(bone_entity) else {
                continue;
            };

            let offset = joint.data().offset();

            // Get data from 2 frames surrounding the target time
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
                (next_offset, next_rotation) = next_frame.get_translation_rotation(channels);

                current_translation += current_offset;
                next_translation += next_offset;
            }

            // Interpolate between the 2 frames
            let interpolated_translation =
                Vec3::lerp(current_translation, next_translation, interpolation_factor);

            let interpolated_rotation =
                Quat::slerp(current_rotation, next_rotation, interpolation_factor);

            bone_transform.rotation = interpolated_rotation;

            if bone_name == "Hips" {
                // Mutate the scene transform rather than the hips bone
                scene_transform.translation = interpolated_translation * 0.01;
                scene_transform.translation.y = 0.0;
            } else {
                bone_transform.translation = interpolated_translation;
            }
        }
    }

    *local_time += time.delta_seconds();
}

pub fn get_pose(local_time: f32, bvh_data: &Bvh) -> (usize, f32) {
    let duration_per_frame = bvh_data.frame_time().as_secs_f32();
    // println!("BvhData {}", bvh_data);

    let total_animation_time = duration_per_frame * bvh_data.frames().len() as f32;

    let animation_time = local_time % total_animation_time;

    let frame_index =
        (animation_time / duration_per_frame).floor() as usize % bvh_data.frames().len();

    let interpolation_factor = (animation_time % duration_per_frame) / duration_per_frame;

    (frame_index, interpolation_factor)
}

// pub fn test(input: Res<ButtonInput<KeyCode>>, mut target_time_event: EventWriter<TargetTimeEvent>) {
//     if input.just_pressed(KeyCode::Space) {
//         target_time_event.send(TargetTimeEvent { time: 50.0 });
//     }
// }

pub type BoneHashMap = HashMap<String, Entity>;

#[derive(Component, Default, Debug)]
pub struct BoneMap(pub HashMap<String, Entity>);

#[derive(Resource, Default, Debug)]
pub struct SelectedBvhAsset(pub AssetId<BvhAsset>);

#[derive(Event)]
pub struct TargetTimeEvent {
    pub time: f32,
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
