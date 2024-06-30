use bevy::{
    asset::{DependencyLoadState, LoadState, RecursiveDependencyLoadState},
    prelude::*,
    utils::hashbrown::HashMap,
};
use bevy_bvh_anim::prelude::*;

use crate::{bvh_library::BvhLibrary, scene_loader::MainScene};

pub struct BvhPlayerPlugin;

impl Plugin for BvhPlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedBvhAsset>()
            .add_event::<TargetTimeEvent>()
            .add_systems(Update, generate_bone_map)
            .add_systems(Update, generate_bone_transform_map)
            .add_systems(Update, draw_armature)
            .add_systems(Update, bvh_player)
            .register_type::<OriginTransform>();
    }
}

/// Original transform when it was first loaded.
#[derive(Component, Clone, Copy, Reflect)]
pub struct OriginTransform(Transform);

impl OriginTransform {
    pub fn get(&self) -> Transform {
        self.0
    }
}

pub type BoneTransformHashMap = HashMap<Entity, Transform>;
/// Maps from one bone transform to another.
#[derive(Component, Clone)]
pub struct BoneTransformMap(pub BoneTransformHashMap);

pub type BoneHashMap = HashMap<String, Entity>;
/// Maps bone name to their respective entity.
#[derive(Component, Default, Debug)]
pub struct BoneMap(pub BoneHashMap);

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

#[allow(clippy::type_complexity)]
fn generate_bone_transform_map(
    mut commands: Commands,
    q_bone_map: Query<(Entity, &BoneMap), (With<MainScene>, Without<BoneTransformMap>)>,
    q_origin_transforms: Query<&OriginTransform>,
    bvh_library: Res<BvhLibrary>,
    bvh_asset: Res<Assets<BvhAsset>>,
) {
    let Ok((entity, bone_map)) = q_bone_map.get_single() else {
        return;
    };

    let Some(bvh_map) = bvh_library.get_map().and_then(|h| bvh_asset.get(h)) else {
        return;
    };

    let bvh = bvh_map.get();
    let Some(frame) = bvh.frames().next() else {
        return;
    };
    let frame = FrameData(frame);
    let mut bone_transform_map = BoneTransformHashMap::new();

    println!("Translation Map:");
    // Iterate all the joints in the bvh and calculate the transform map
    for joint in bvh.joints() {
        let joint_data = joint.data();
        let bone_name = joint_data.name().to_str().unwrap();
        // Get bone origin transform
        let Some(&bone_entity) = bone_map.0.get(bone_name) else {
            continue;
        };
        let Ok(origin_transform) = q_origin_transforms.get(bone_entity) else {
            continue;
        };
        let origin_transform = origin_transform.get();

        // Calculate bvh joint transform matrix
        let offset = joint_data.offset();
        let mut translation = Vec3::new(offset.x, offset.y, offset.z);

        let channels = joint_data.channels();
        let rotation;

        if channels.len() == 3 {
            rotation = frame.get_rotation(channels);
        } else {
            let translation_offset;
            (translation_offset, rotation) = frame.get_translation_rotation(channels);

            translation += translation_offset;
        }

        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            rotation.x.to_radians(),
            rotation.y.to_radians(),
            rotation.z.to_radians(),
        );
        let joint_matrix = Mat4::from_rotation_translation(rotation, translation);

        // Calculate origin bone transform matrix
        let origin_matrix = Mat4::from_rotation_translation(
            origin_transform.rotation,
            origin_transform.translation,
        );
        // Calculate transform map using joint matrix and origin matrix
        let transform_map =
            Transform::from_matrix(Mat4::mul_mat4(&joint_matrix.inverse(), &origin_matrix));

        bone_transform_map.insert(bone_entity, transform_map);

        println!("{} | {}", origin_transform.translation, translation);
    }

    commands
        .entity(entity)
        .insert(BoneTransformMap(bone_transform_map));
}

fn bvh_player(
    mut q_transforms: Query<&mut Transform, Without<MainScene>>,
    mut q_scene: Query<(&mut Transform, &BoneMap, &BoneTransformMap), With<MainScene>>,
    mut event_reader: EventReader<TargetTimeEvent>,
    time: Res<Time>,
    selected_bvh_asset: Res<SelectedBvhAsset>,
    bvh_asset: Res<Assets<BvhAsset>>,
    mut local_time: Local<f32>,
) {
    let Some(bvh) = bvh_asset.get(selected_bvh_asset.0) else {
        return;
    };
    let bvh = bvh.get();

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

    for (mut _scene_transform, bone_map, _bone_transform_map) in q_scene.iter_mut() {
        for joint in bvh.joints() {
            let joint_data = joint.data();
            let bone_name = joint_data.name().to_str().unwrap();

            let Some(&bone_entity) = bone_map.0.get(bone_name) else {
                continue;
            };
            // Get bone transform
            let Ok(mut bone_transform) = q_transforms.get_mut(bone_entity) else {
                continue;
            };
            // Get bone transform map
            // let Some(&transform_map) = bone_transform_map.0.get(&bone_entity) else {
            //     continue;
            // };

            let mut offset = Vec3::default();

            if joint_data.is_child() {
                let o = joint_data.offset();
                offset = Vec3::new(o.x, o.y, o.z);
            }

            // let o = joint_data.offset();
            // let offset = Vec3::new(o.x, o.y, o.z);

            // Get data from 2 frames surrounding the target time
            let mut current_translation = offset;
            let mut next_translation = offset;

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

            let interpolated_transform = Transform::from_translation(interpolated_translation)
                .with_rotation(interpolated_rotation);
            // .mul_transform(transform_map);

            *bone_transform = interpolated_transform;
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

fn draw_armature(
    q_character: Query<(Entity, &GlobalTransform), With<MainScene>>,
    q_children: Query<&Children>,
    q_transforms: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    const RAINBOW: [Color; 7] = [
        Color::RED,
        Color::ORANGE,
        Color::YELLOW,
        Color::GREEN,
        Color::BLUE,
        Color::INDIGO,
        Color::PURPLE,
    ];

    fn recursive_draw(
        mut index: usize,
        parent: Entity,
        translation: Vec3,
        q_children: &Query<&Children>,
        q_transforms: &Query<&GlobalTransform>,
        gizmos: &mut Gizmos,
    ) {
        gizmos.sphere(
            translation,
            Quat::IDENTITY,
            0.04,
            RAINBOW[index % RAINBOW.len()],
        );

        if let Ok(children) = q_children.get(parent) {
            for &child in children.iter() {
                if let Ok(transform) = q_transforms.get(child) {
                    let child_translation = transform.translation();
                    gizmos.line(translation, child_translation, Color::CYAN);
                    index += 1;

                    recursive_draw(
                        index,
                        child,
                        child_translation,
                        q_children,
                        q_transforms,
                        gizmos,
                    );
                }
            }
        }
    }

    if let Ok((entity, transform)) = q_character.get_single() {
        recursive_draw(
            0,
            entity,
            transform.translation(),
            &q_children,
            &q_transforms,
            &mut gizmos,
        );
    }
}

// pub fn test(input: Res<ButtonInput<KeyCode>>, mut target_time_event: EventWriter<TargetTimeEvent>) {
//     if input.just_pressed(KeyCode::Space) {
//         target_time_event.send(TargetTimeEvent { time: 50.0 });
//     }
// }
