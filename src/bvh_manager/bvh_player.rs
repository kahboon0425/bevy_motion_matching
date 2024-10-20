use bevy::{
    asset::{DependencyLoadState, LoadState, RecursiveDependencyLoadState},
    prelude::*,
    utils::hashbrown::HashMap,
};
use bevy_bvh_anim::prelude::*;

use crate::{scene_loader::MainScene, GameMode};

pub struct BvhPlayerPlugin;

impl Plugin for BvhPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedBvhAsset>()
            .init_resource::<BvhPlayer>()
            .add_systems(Update, (generate_bone_map, bvh_player))
            .add_systems(
                OnEnter(GameMode::Config),
                |mut player: ResMut<BvhPlayer>| {
                    player.is_playing = true;
                },
            )
            .add_systems(OnExit(GameMode::Config), |mut player: ResMut<BvhPlayer>| {
                player.is_playing = false;
            });
    }
}

fn generate_bone_map(
    mut commands: Commands,
    q_character: Query<(Entity, &Handle<Scene>), (With<MainScene>, Without<JointMap>)>,
    q_names: Query<&Name>,
    q_children: Query<&Children>,
    q_transforms: Query<&Transform>,
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
        let mut joint_map = JointMap::default();

        for bone_entity in q_children.iter_descendants(entity) {
            if let Ok(name) = q_names.get(bone_entity) {
                let bone_name = name.to_string();
                joint_map.insert(bone_name, bone_entity);
            }
        }

        commands.entity(entity).insert(joint_map);

        /// Recurisvely print the bone hierarchy.
        fn recursive_print(
            indent: usize,
            parent: Entity,
            q_children: &Query<&Children>,
            q_names: &Query<&Name>,
            q_transforms: &Query<&Transform>,
        ) {
            if let Ok(children) = q_children.get(parent) {
                for &child in children.iter() {
                    for _ in 0..indent {
                        print!("| ");
                    }
                    if let (Ok(name), Ok(transform)) = (q_names.get(child), q_transforms.get(child))
                    {
                        let rotation = quat_to_eulerdeg(transform.rotation);
                        print!("{}: ", &name);
                        println!("({:.2}, {:.2}, {:.2})", rotation.x, rotation.y, rotation.z,);
                    }
                    recursive_print(indent + 1, child, q_children, q_names, q_transforms);
                }
            }
        }

        println!("\nBONE HIERARCHY");
        recursive_print(0, entity, &q_children, &q_names, &q_transforms);
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

fn bvh_player(
    mut q_transforms: Query<&mut Transform>,
    q_scene: Query<&JointMap, With<MainScene>>,
    time: Res<Time>,
    selected_bvh_asset: Res<SelectedBvhAsset>,
    bvh_assets: Res<Assets<BvhAsset>>,
    mut bvh_player: ResMut<BvhPlayer>,
    mut local_time: Local<f32>,
) {
    let Some(bvh) = bvh_assets.get(selected_bvh_asset.0) else {
        return;
    };
    let bvh = bvh.get();

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

    for joint_map in q_scene.iter() {
        for joint in bvh.joints() {
            let joint_data = joint.data();
            let bone_name = joint_data.name().to_str().unwrap();

            let Some(&bone_entity) = joint_map.get(bone_name) else {
                continue;
            };
            // Get bone transform
            let Ok(mut transform) = q_transforms.get_mut(bone_entity) else {
                continue;
            };

            let o = joint_data.offset();
            let offset = Vec3::new(o.x, o.y, o.z);

            // Get data from 2 frames surrounding the target time.
            let mut curr_translation = offset;
            let mut next_translation = offset;

            let channels = joint_data.channels();

            let curr_euler;
            let next_euler;

            if channels.len() == 3 {
                curr_euler = current_frame.get_euler(channels);
                next_euler = next_frame.get_euler(channels);
            } else {
                let current_offset;
                let next_offset;
                (current_offset, curr_euler) = current_frame.get_translation_euler(channels);
                (next_offset, next_euler) = next_frame.get_translation_euler(channels);

                // Overwrite translation if it exists
                curr_translation = current_offset;
                next_translation = next_offset;
            }

            let curr_rotation = eulerdeg_to_quat(curr_euler);
            let next_rotation = eulerdeg_to_quat(next_euler);

            // Interpolate between the 2 frames
            let interp_translation =
                Vec3::lerp(curr_translation, next_translation, interpolation_factor);
            let interp_rotation = Quat::slerp(curr_rotation, next_rotation, interpolation_factor);

            transform.translation = interp_translation;
            transform.rotation = interp_rotation;
        }
    }

    // Should not do anything is current_time has not been mutated anywhere else,
    // otherwise, local_time will be set to the mutated value.
    *local_time = bvh_player.current_time;
    if bvh_player.is_playing {
        *local_time += time.delta_seconds();
        bvh_player.current_time = *local_time % bvh_player.duration
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

pub fn quat_to_eulerdeg(rotation: Quat) -> Vec3 {
    let euler = rotation.to_euler(EulerRot::XYZ);
    Vec3::new(
        euler.0.to_degrees(),
        euler.1.to_degrees(),
        euler.2.to_degrees(),
    )
}

pub fn eulerdeg_to_quat(euler: Vec3) -> Quat {
    Quat::from_euler(
        EulerRot::XYZ,
        euler.x.to_radians(),
        euler.y.to_radians(),
        euler.z.to_radians(),
    )
}

/// Maps joint name to their respective entity.
#[derive(Component, Default, Debug, Clone, Deref, DerefMut)]
pub struct JointMap(pub HashMap<String, Entity>);

#[derive(Resource, Default, Debug)]
pub struct SelectedBvhAsset(pub AssetId<BvhAsset>);

#[derive(Debug)]
pub struct FrameData<'a>(pub &'a Frame);

impl<'a> FrameData<'a> {
    pub fn get_euler(&self, channels: &[Channel]) -> Vec3 {
        Vec3::new(
            self.0[&channels[0]],
            self.0[&channels[1]],
            self.0[&channels[2]],
        )
    }

    pub fn get_translation_euler(&self, channels: &[Channel]) -> (Vec3, Vec3) {
        (
            Vec3::new(
                self.0[&channels[0]],
                self.0[&channels[1]],
                self.0[&channels[2]],
            ),
            Vec3::new(
                self.0[&channels[3]],
                self.0[&channels[4]],
                self.0[&channels[5]],
            ),
        )
    }
}

#[derive(Resource, Default)]
pub struct BvhPlayer {
    pub is_playing: bool,
    pub current_time: f32,
    pub duration: f32,
}
