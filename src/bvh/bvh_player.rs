use crate::{bvh_asset::BvhAsset, scene_loader::MainScene};
use bevy::{
    asset::{DependencyLoadState, LoadState, RecursiveDependencyLoadState},
    prelude::*,
    utils::hashbrown::HashMap,
};
use bvh_anim::{Bvh, Channel, Frame};

pub struct BvhPlayerPlugin;

impl Plugin for BvhPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedBvhAsset>()
            .add_event::<TargetTimeEvent>()
            .add_systems(Update, generate_bone_map)
            .add_systems(Update, draw_armature)
            .add_systems(Update, bvh_player)
            .register_type::<OriginTransform>();
    }
}

#[derive(Component, Clone, Copy, Reflect)]
pub struct OriginTransform(Transform);

// impl OriginTransform {
//     pub fn get(&self) -> Transform {
//         self.0
//     }
// }

#[allow(clippy::type_complexity)]
fn generate_bone_map(
    mut commands: Commands,
    q_character: Query<(Entity, &Handle<Scene>), (With<MainScene>, Without<BoneMap>)>,
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
        let mut bone_map = BoneHashMap::default();

        for bone_entity in q_children.iter_descendants(entity) {
            if let Ok(transform) = q_transforms.get(bone_entity) {
                commands
                    .entity(bone_entity)
                    .insert(OriginTransform(*transform));
            }

            if let Ok(name) = q_names.get(bone_entity) {
                let bone_name = name.to_string();
                bone_map.insert(bone_name, bone_entity);
            }
        }

        commands.entity(entity).insert(BoneMap(bone_map));

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
                        println!(
                            "{:?}: {:?}",
                            &name,
                            transform.rotation.to_euler(EulerRot::XYZ)
                        );
                    }
                    recursive_print(indent + 1, child, q_children, q_names, q_transforms);
                }
            }
        }

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

    for (mut _scene_transform, bone_map) in q_scene.iter_mut() {
        for joint in bvh.joints() {
            let joint_data = joint.data();
            let bone_name = joint_data.name().to_str().unwrap();
            // Get bone transform
            let Some(&bone_entity) = bone_map.0.get(bone_name) else {
                continue;
            };
            let Ok(mut bone_transform) = q_transforms.get_mut(bone_entity) else {
                continue;
            };

            let mut offset = Vec3::default();

            if joint_data.is_child() {
                let o = joint_data.offset();
                offset = Vec3::new(o.x, o.y, o.z);
            }

            // let origin_translation = origin_transform.get().translation;
            // let origin_rotation = origin_transform.get().rotation.to_euler(EulerRot::XYZ);

            // Get data from 2 frames surrounding the target time
            let mut current_translation = Vec3::new(offset.x, offset.y, offset.z);
            let mut next_translation = Vec3::new(offset.x, offset.y, offset.z);

            let channels = joint_data.channels();

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

            let interpolated_rotation = Quat::slerp(
                Quat::from_euler(
                    EulerRot::XYZ,
                    current_rotation.x.to_radians(),
                    current_rotation.y.to_radians(),
                    current_rotation.z.to_radians(),
                ),
                Quat::from_euler(
                    EulerRot::XYZ,
                    next_rotation.x.to_radians(),
                    next_rotation.y.to_radians(),
                    next_rotation.z.to_radians(),
                ),
                interpolation_factor,
            );

            bone_transform.translation = interpolated_translation;
            bone_transform.rotation = interpolated_rotation;
        }
    }

    *local_time += time.delta_seconds();
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
    pub fn get_rotation(&self, channels: &[Channel]) -> Vec3 {
        Vec3::new(
            self.0[&channels[0]],
            self.0[&channels[1]],
            self.0[&channels[2]],
        )
    }

    pub fn get_translation_rotation(&self, channels: &[Channel]) -> (Vec3, Vec3) {
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

fn draw_armature(
    q_character: Query<(Entity, &GlobalTransform), With<MainScene>>,
    q_children: Query<&Children>,
    q_transforms: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    fn recursive_draw(
        parent: Entity,
        translation: Vec3,
        q_children: &Query<&Children>,
        q_transforms: &Query<&GlobalTransform>,
        gizmos: &mut Gizmos,
    ) {
        gizmos.sphere(translation, Quat::IDENTITY, 0.04, Color::RED);
        if let Ok(children) = q_children.get(parent) {
            for &child in children.iter() {
                if let Ok(transform) = q_transforms.get(child) {
                    let child_translation = transform.translation();
                    gizmos.line(translation, child_translation, Color::CYAN);
                    recursive_draw(child, child_translation, q_children, q_transforms, gizmos);
                }
            }
        }
    }

    if let Ok((entity, transform)) = q_character.get_single() {
        recursive_draw(
            entity,
            transform.translation(),
            &q_children,
            &q_transforms,
            &mut gizmos,
        );
    }
}
