use bevy::{color::palettes::css, prelude::*};
use bevy_bvh_anim::prelude::*;

use crate::draw_axes::{ColorPalette, DrawAxes};
use crate::motion_data::motion_data_asset::Pose;
use crate::scene_loader::MainScene;
use crate::ui::config::BvhTrailConfig;
use crate::BVH_SCALE_RATIO;

use super::bvh_player::SelectedBvhAsset;

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
    mut axes: ResMut<DrawAxes>,
    palette: Res<ColorPalette>,
) {
    const SKIP_HIERARCHY: usize = 3;

    fn recursive_draw(
        mut index: usize,
        parent: Entity,
        parent_transform: &GlobalTransform,
        q_children: &Query<&Children>,
        q_transforms: &Query<&GlobalTransform>,
        gizmos: &mut Gizmos,
        axes: &mut DrawAxes,
        palette: &ColorPalette,
    ) {
        let gradient = [
            &palette.red,
            &palette.orange,
            &palette.yellow,
            &palette.green,
            &palette.blue,
            &palette.purple,
        ];

        gizmos.cuboid(
            parent_transform
                .compute_transform()
                .with_scale(Vec3::splat(0.1)),
            gradient[index % gradient.len()].with_alpha(0.4),
        );

        index += 1;
        if index > SKIP_HIERARCHY {
            axes.draw(
                parent_transform.compute_matrix(),
                1.0 / BVH_SCALE_RATIO * 0.04,
            );
        }

        if let Ok(children) = q_children.get(parent) {
            for &child in children.iter() {
                if let Ok(transform) = q_transforms.get(child) {
                    let child_translation = transform.translation();

                    if index > SKIP_HIERARCHY {
                        gizmos.line(
                            parent_transform.translation(),
                            child_translation,
                            palette.base5,
                        );
                    }

                    recursive_draw(
                        index,
                        child,
                        transform,
                        q_children,
                        q_transforms,
                        gizmos,
                        axes,
                        palette,
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
            &mut axes,
            &palette,
        );
    }
}

fn bvh_trail_gizmos(
    config: Res<BvhTrailConfig>,
    selected_bvh_asset: Res<SelectedBvhAsset>,
    bvh_assets: Res<Assets<BvhAsset>>,
    mut gizmos: Gizmos,
    mut axes: ResMut<DrawAxes>,
    palette: Res<ColorPalette>,
) {
    if config.draw == false {
        return;
    }

    if config.interval < BvhTrailConfig::MIN_INTERVAL {
        return;
    }

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

    let frame_time = bvh.frame_time().as_secs_f32();
    let total_duration = frame_time * bvh.num_frames() as f32;

    let mut time = 0.0;

    while time < total_duration {
        let index = (time / frame_time) as usize;

        let Some(curr_frame) = bvh.frames().nth(index) else {
            break;
        };
        let Some(next_frame) = bvh.frames().nth(index + 1) else {
            break;
        };

        let leak = time - frame_time * index as f32;
        let factor = leak / frame_time;
        let curr_pose = Pose::from_frame(curr_frame);
        let next_pose = Pose::from_frame(next_frame);
        let pose = Pose::lerp(&curr_pose, &next_pose, factor);

        joint_matrices.apply_frame(&pose);

        for world_matrix in joint_matrices.world_matrices() {
            let (_, rotation, translation) = world_matrix.to_scale_rotation_translation();
            gizmos.cuboid(
                Transform::from_translation(translation * BVH_SCALE_RATIO)
                    .with_rotation(rotation)
                    .with_scale(Vec3::splat(0.06)),
                palette.blue.with_alpha(0.5),
            );
            axes.draw(
                world_matrix.mul_scalar(BVH_SCALE_RATIO),
                1.0 / BVH_SCALE_RATIO * 0.04,
            );
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
                parent_translation * BVH_SCALE_RATIO,
                curr_translation * BVH_SCALE_RATIO,
                palette.base6,
            );
        }

        time += config.interval;
    }
}
