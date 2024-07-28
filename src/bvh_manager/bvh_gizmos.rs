use bevy::{color::palettes::css, prelude::*};
use bevy_bvh_anim::prelude::*;

use crate::{scene_loader::MainScene, ui::config::BvhTrailConfig};

use super::bvh_player::SelectedBvhAsset;

const AXIS_LENGTH: f32 = 0.04;
const SPHERE_SIZE: f32 = 0.02;
const RAINBOW: [Srgba; 7] = [
    css::RED,
    css::ORANGE,
    css::YELLOW,
    css::GREEN,
    css::BLUE,
    css::INDIGO,
    css::PURPLE,
];

pub struct BvhGizmosPlugin;

impl Plugin for BvhGizmosPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, armature_gizmos)
            .add_systems(Update, bvh_trail_gizmos);
    }
}

fn armature_gizmos(
    q_character: Query<(Entity, &GlobalTransform), With<MainScene>>,
    q_children: Query<&Children>,
    q_transforms: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    const SKIP_HIERARCHY: usize = 3;

    fn recursive_draw(
        mut index: usize,
        parent: Entity,
        parent_transform: &GlobalTransform,
        q_children: &Query<&Children>,
        q_transforms: &Query<&GlobalTransform>,
        gizmos: &mut Gizmos,
    ) {
        let (_, rotation, translation) = parent_transform.to_scale_rotation_translation();
        gizmos.sphere(
            translation,
            rotation,
            SPHERE_SIZE,
            RAINBOW[index % RAINBOW.len()].with_alpha(0.4),
        );

        index += 1;
        if index > SKIP_HIERARCHY {
            draw_axis(translation, rotation, gizmos);
        }

        if let Ok(children) = q_children.get(parent) {
            for &child in children.iter() {
                if let Ok(transform) = q_transforms.get(child) {
                    let child_translation = transform.translation();

                    if index > SKIP_HIERARCHY {
                        gizmos.line(
                            parent_transform.translation(),
                            child_translation,
                            css::LIGHT_CYAN,
                        );
                    }

                    recursive_draw(index, child, transform, q_children, q_transforms, gizmos);
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

fn bvh_trail_gizmos(
    config: Res<BvhTrailConfig>,
    selected_bvh_asset: Res<SelectedBvhAsset>,
    bvh_assets: Res<Assets<BvhAsset>>,
    mut gizmos: Gizmos,
) {
    if config.draw == false {
        return;
    }
    let step = BvhTrailConfig::MAX_RESOLUTION - config.resolution + 1;

    let Some(bvh) = bvh_assets
        .get(selected_bvh_asset.0)
        .map(|asset| asset.get())
    else {
        return;
    };

    let mut joint_matrices = JointMatrices::new(
        &bvh.joints()
            .map(|joint| joint.data().clone())
            .collect::<Vec<_>>(),
    );

    for frame in bvh.frames().step_by(step) {
        joint_matrices.apply_frame(frame.as_slice());

        for world_matrix in joint_matrices.world_matrices() {
            let (_, rotation, mut translation) = world_matrix.to_scale_rotation_translation();
            // Constant scaling factor of the Bvh data.
            translation *= 0.01;

            gizmos.sphere(
                translation,
                rotation,
                SPHERE_SIZE,
                css::YELLOW.with_alpha(0.4),
            );
            draw_axis(translation, rotation, &mut gizmos);
        }

        for joint in joint_matrices.joints() {
            let Some(parent_index) = joint.parent_index() else {
                continue;
            };

            let (.., parent_translation) =
                joint_matrices.world_matrices()[parent_index].to_scale_rotation_translation();
            let (.., curr_translation) =
                joint_matrices.world_matrices()[joint.index()].to_scale_rotation_translation();

            gizmos.line(
                // Constant scaling factor of the Bvh data.
                parent_translation * 0.01,
                curr_translation * 0.01,
                css::WHEAT,
            );
        }
    }
}

fn draw_axis(translation: Vec3, rotation: Quat, gizmos: &mut Gizmos) {
    let x_dir = rotation * Vec3::X;
    let y_dir = rotation * Vec3::Y;
    let z_dir = rotation * -Vec3::Z;

    gizmos.line(translation, translation + x_dir * AXIS_LENGTH, css::RED);
    gizmos.line(translation, translation + y_dir * AXIS_LENGTH, css::GREEN);
    gizmos.line(translation, translation + z_dir * AXIS_LENGTH, css::BLUE);
}
