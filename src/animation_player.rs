use std::f32::consts::PI;

use bevy::{asset::LoadState, prelude::*};
use bvh_anim::Bvh;

use crate::{animation_loader::BvhData, character_loader::BvhToCharacter};

#[derive(Component)]
pub struct BoneIndex(pub usize);

#[derive(Component)]
pub struct BoneRotation(pub Quat);

#[derive(Resource)]
pub struct FootTransforms {
    pub left_foot_current: Vec3,
    pub left_foot_previous: Vec3,
    pub right_foot_current: Vec3,
    pub right_foot_previous: Vec3,
}

impl FootTransforms {
    pub fn new() -> Self {
        Self {
            left_foot_current: Vec3::ZERO,
            left_foot_previous: Vec3::ZERO,
            right_foot_current: Vec3::ZERO,
            right_foot_previous: Vec3::ZERO,
        }
    }
}

#[derive(Event)]
pub struct FootTransformsEvent {
    pub left_foot_current: Vec3,
    pub left_foot_previous: Vec3,
    pub right_foot_current: Vec3,
    pub right_foot_previous: Vec3,
}

pub fn match_bones(
    mut commands: Commands,
    mut q_names: Query<(Entity, &Name, &mut Transform, &GlobalTransform)>,
    mut bvh_data: ResMut<BvhData>,
    mut bvh_to_character: ResMut<BvhToCharacter>,
    mut foot_transforms: ResMut<FootTransforms>,
    server: Res<AssetServer>,
    mut event_writer: EventWriter<FootTransformsEvent>,
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

    if let Some(bvh_vec) = &bvh_data.bvh_animation {
        let bvh: Bvh = bvh_vec[1].clone();

        let frame_index = bvh_data.current_frame_index;

        if frame_index < bvh.frames().len() {
            // println!("Frame Index: {}", frame_index);
            // println!("Bvh frame length: {}", bvh.frames().len());
            // let Some(frame) = &bvh.frames()
            if let Some(frame) = bvh.frames().nth(frame_index) {
                // let frame: &bvh_anim::Frame = bvh.frames().last().unwrap();

                // println!("{:#?}", frame);

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

                            let mut offset_x = joint.data().offset().x;
                            let mut offset_y = joint.data().offset().y;
                            let mut offset_z = joint.data().offset().z;

                            let rotation0;
                            let rotation1;
                            let rotation2;

                            if joint.data().channels().len() == 3 {
                                rotation0 = frame[&joint.data().channels()[0]];
                                rotation1 = frame[&joint.data().channels()[1]];
                                rotation2 = frame[&joint.data().channels()[2]];
                            } else {
                                offset_x += frame[&joint.data().channels()[0]];
                                offset_y += frame[&joint.data().channels()[1]];
                                offset_z += frame[&joint.data().channels()[2]];

                                rotation0 = frame[&joint.data().channels()[3]];
                                rotation1 = frame[&joint.data().channels()[4]];
                                rotation2 = frame[&joint.data().channels()[5]];
                            }

                            let rotation = Quat::from_euler(
                                EulerRot::ZYX,
                                rotation0.to_radians(),
                                rotation1.to_radians(),
                                rotation2.to_radians(),
                            );

                            // println!("origin transform: {:?}", transform.translation);
                            // println!("bvh offset: {}, {}, {}", offset_x, offset_y, offset_z);

                            transform.translation = Vec3::new(offset_x, offset_y, offset_z);
                            transform.rotation = rotation;

                            // Update the rotation of the entity for each frame
                            commands.entity(entity).insert(BoneRotation(rotation));
                            // println!("Bone Name: {}, Rotation: {:?}", bone_name, rotation);

                            if bone_name == "LeftFoot" {
                                // Store the current position as the previous for the left foot
                                foot_transforms.left_foot_previous =
                                    foot_transforms.left_foot_current;
                                // Update the current position for the left foot
                                foot_transforms.left_foot_current = global_transform.translation();
                            } else if bone_name == "RightFoot" {
                                foot_transforms.right_foot_previous =
                                    foot_transforms.right_foot_current;
                                foot_transforms.right_foot_current = global_transform.translation();
                            }

                            event_writer.send(FootTransformsEvent {
                                left_foot_current: foot_transforms.left_foot_current,
                                left_foot_previous: foot_transforms.left_foot_previous,
                                right_foot_current: foot_transforms.right_foot_current,
                                right_foot_previous: foot_transforms.right_foot_previous,
                            });
                        }

                        joint_index += 1;
                    }
                }
            }

            bvh_data.current_frame_index += 1;

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

pub fn draw_movement_arrows(
    mut gizmos: Gizmos,
    mut event_reader: EventReader<FootTransformsEvent>,
) {
    // gizmos.arrow(
    //     Vec3::new(11.766598, -0.000002, -0.00001),
    //     Vec3::new(25.19977, 0.000143, 0.000407),
    //     Color::YELLOW,
    // );

    for event in event_reader.read() {
        if event.left_foot_previous != event.left_foot_current {
            gizmos.arrow(
                event.left_foot_previous,
                event.left_foot_current,
                Color::YELLOW,
            );
        }

        if event.right_foot_previous != event.right_foot_current {
            gizmos.arrow(
                event.right_foot_previous,
                event.right_foot_current,
                Color::BLUE,
            );
        }
    }
}
