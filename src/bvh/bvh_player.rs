use bevy::{
    asset::{DependencyLoadState, LoadState, RecursiveDependencyLoadState},
    color::palettes::css,
    prelude::*,
    utils::hashbrown::HashMap,
};
use bevy_bvh_anim::prelude::*;

use crate::{
    scene_loader::MainScene,
    ui::config::{DrawBvhTrail, PlaybackState},
};

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

/// Original transform when it was first loaded.
#[derive(Component, Clone, Copy, Reflect)]
pub struct OriginTransform(Transform);

impl OriginTransform {
    pub fn get(&self) -> Transform {
        self.0
    }
}

#[allow(dead_code)]
/// Bvh joint original translations and euler angles.
#[derive(Component, Default, Debug, Clone)]
pub struct BvhOriginMap(pub HashMap<Entity, (Vec3, Vec3)>);

/// Maps bone name to their respective entity.
#[derive(Component, Default, Debug, Clone)]
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
        let mut bone_map = BoneMap::default();

        for bone_entity in q_children.iter_descendants(entity) {
            if let Ok(&transform) = q_transforms.get(bone_entity) {
                commands
                    .entity(bone_entity)
                    .insert(OriginTransform(transform));
            }

            if let Ok(name) = q_names.get(bone_entity) {
                let bone_name = name.to_string();
                bone_map.0.insert(bone_name, bone_entity);
            }
        }

        commands.entity(entity).insert(bone_map);

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

#[allow(clippy::too_many_arguments)]
fn bvh_player(
    mut q_transforms: Query<&mut Transform, Without<MainScene>>,
    mut q_scene: Query<(&mut Transform, &BoneMap), With<MainScene>>,
    mut event_reader: EventReader<TargetTimeEvent>,
    time: Res<Time>,
    selected_bvh_asset: Res<SelectedBvhAsset>,
    bvh_asset: Res<Assets<BvhAsset>>,
    mut playback_state: ResMut<PlaybackState>,
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

    for (mut _scene_transform, bone_map) in q_scene.iter_mut() {
        for joint in bvh.joints() {
            let joint_data = joint.data();
            let bone_name = joint_data.name().to_str().unwrap();

            let Some(&bone_entity) = bone_map.0.get(bone_name) else {
                continue;
            };
            // Get bone transform
            let Ok(mut transform) = q_transforms.get_mut(bone_entity) else {
                continue;
            };

            let o = joint_data.offset();
            let offset = Vec3::new(o.x, o.y, o.z);

            // Get data from 2 frames surrounding the target time
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
    *local_time = playback_state.current_time;
    if playback_state.is_playing {
        *local_time += time.delta_seconds();
        playback_state.current_time = *local_time % playback_state.duration
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

fn draw_armature(
    q_character: Query<(Entity, &GlobalTransform), With<MainScene>>,
    q_children: Query<&Children>,
    q_transforms: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    const RAINBOW: [Srgba; 7] = [
        css::RED,
        css::ORANGE,
        css::YELLOW,
        css::GREEN,
        css::BLUE,
        css::INDIGO,
        css::PURPLE,
    ];
    const SPHERE_SIZE: f32 = 0.03;
    const AXIS_LENGTH: f32 = 0.1;

    fn recursive_draw(
        mut color_index: usize,
        parent: Entity,
        parent_transform: &GlobalTransform,
        q_children: &Query<&Children>,
        q_transforms: &Query<&GlobalTransform>,
        gizmos: &mut Gizmos,
    ) {
        gizmos.sphere(
            parent_transform.translation(),
            Quat::IDENTITY,
            SPHERE_SIZE,
            RAINBOW[color_index % RAINBOW.len()].with_alpha(0.4),
        );
        gizmos.line(
            parent_transform.translation(),
            parent_transform.translation() + parent_transform.right() * AXIS_LENGTH,
            css::RED,
        );
        gizmos.line(
            parent_transform.translation(),
            parent_transform.translation() + parent_transform.up() * AXIS_LENGTH,
            css::GREEN,
        );
        gizmos.line(
            parent_transform.translation(),
            parent_transform.translation() + parent_transform.forward() * AXIS_LENGTH,
            css::BLUE,
        );

        color_index += 1;
        if let Ok(children) = q_children.get(parent) {
            for &child in children.iter() {
                if let Ok(transform) = q_transforms.get(child) {
                    let child_translation = transform.translation();
                    gizmos.line(
                        parent_transform.translation(),
                        child_translation,
                        css::LIGHT_CYAN,
                    );

                    recursive_draw(
                        color_index,
                        child,
                        transform,
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
            transform,
            &q_children,
            &q_transforms,
            &mut gizmos,
        );
    }
}

fn draw_bvh_trail(draw: Res<DrawBvhTrail>) {
    if draw.get() == false {
        return;
    }

    todo!()
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

// pub fn test(input: Res<ButtonInput<KeyCode>>, mut target_time_event: EventWriter<TargetTimeEvent>) {
//     if input.just_pressed(KeyCode::Space) {
//         target_time_event.send(TargetTimeEvent { time: 50.0 });
//     }
// }
