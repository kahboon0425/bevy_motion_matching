use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::texture::{
    ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy_bvh_anim::prelude::JointMatrices;

use crate::draw_axes::{ColorPalette, DrawAxes};
use crate::motion::chunk::ChunkIterator;
use crate::motion::motion_player::MotionPlayerBundle;
use crate::motion::pose_data::Pose;
use crate::motion::trajectory_data::TrajectoryDataPoint;
use crate::motion::MotionData;
use crate::motion_matching::NearestTrajectories;
use crate::player::{PlayerBundle, PlayerMarker};
use crate::trajectory::{TrajectoryBundle, TrajectoryConfig};
use crate::ui::play_mode::MotionMatchingResult;
use crate::BVH_SCALE_RATIO;

/// Load glb file and setup the scene.
pub struct SceneLoaderPlugin;

impl Plugin for SceneLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_scene, spawn_light, spawn_ground))
            .add_systems(
                PostUpdate,
                draw_nearest_traj_arrow.after(TransformSystem::TransformPropagate),
            )
            .add_systems(
                PostUpdate,
                draw_nearest_pose_armature.after(TransformSystem::TransformPropagate),
            );
    }
}

#[derive(Component)]
pub struct MainScene;

fn draw_nearest_traj_arrow(
    motion_data: MotionData,
    trajectory_config: Res<TrajectoryConfig>,
    q_player_transform: Query<&Transform, With<PlayerMarker>>,
    motion_matching_result: Res<MotionMatchingResult>,
    mut nearest_traj: Local<Vec<(NearestTrajectories, Mat4, usize)>>,
    mut nearest_trajectories_evr: EventReader<NearestTrajectories>,
    mut gizmos: Gizmos,
    palette: Res<ColorPalette>,
) {
    const MAX_TRAJ: usize = 10;

    if nearest_traj.len() > MAX_TRAJ {
        nearest_traj.remove(0);
    }

    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    let Ok(player_transform) = q_player_transform.get_single() else {
        return;
    };

    let curr_player_matrix = player_transform.compute_matrix();

    let num_points = trajectory_config.num_points();

    for trajs in nearest_trajectories_evr.read() {
        if trajs.is_empty() {
            continue;
        }

        nearest_traj.push((
            trajs.clone(),
            curr_player_matrix,
            motion_matching_result.selected_trajectory,
        ));
    }

    for (trajs, snapped_player_matrix, selected_index) in nearest_traj.iter() {
        for (i, traj) in trajs.iter().enumerate() {
            let color = match i == *selected_index {
                true => palette.green,
                false => palette.base4.with_alpha(0.8),
            };

            let chunk = motion_asset.trajectory_data.get_chunk(traj.chunk_index);

            if let Some(chunk) = chunk {
                let data_traj = &chunk[traj.chunk_offset..traj.chunk_offset + num_points];

                // Center point of trajectory
                let data_inv_matrix = data_traj[trajectory_config.history_count].matrix.inverse();

                let get_translation = |point: &TrajectoryDataPoint| -> Vec3 {
                    let (.., translation) = point.matrix.to_scale_rotation_translation();
                    let mut translation =
                        data_inv_matrix.transform_point3(translation) * BVH_SCALE_RATIO;
                    translation.y = 0.0;
                    translation = snapped_player_matrix.transform_point3(translation);
                    translation
                };

                let mut previous_translation = get_translation(&data_traj[0]);

                for point in data_traj[1..].iter() {
                    let translation = get_translation(point);

                    gizmos.line(translation, previous_translation, color);
                    gizmos.arrow(previous_translation, translation, color);
                    previous_translation = translation;
                }
            }
        }
    }
}

fn draw_nearest_pose_armature(
    motion_data: MotionData,
    q_player_transform: Query<&Transform, With<PlayerMarker>>,
    mut nearest_trajectories_evr: EventReader<NearestTrajectories>,
    motion_matching_result: Res<MotionMatchingResult>,
    mut gizmos: Gizmos,
    palette: Res<ColorPalette>,
    mut nearest_traj: Local<Vec<(NearestTrajectories, Mat4, usize)>>,
) {
    const MAX_TRAJ: usize = 10;

    if nearest_traj.len() > MAX_TRAJ {
        nearest_traj.remove(0);
    }

    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    let Ok(player_transform) = q_player_transform.get_single() else {
        return;
    };

    let curr_player_matrix = player_transform.compute_matrix();

    for trajs in nearest_trajectories_evr.read() {
        if trajs.is_empty() {
            continue;
        }

        nearest_traj.push((
            trajs.clone(),
            curr_player_matrix,
            motion_matching_result.selected_trajectory,
        ));
    }

    const POSE_OFFSET: f32 = 1.0;

    let mut joint_matrices = JointMatrices::new(motion_asset.joints());

    for (i, (trajs, snapped_player_matrix, selected_pose)) in nearest_traj.iter().enumerate() {
        for (i, traj) in trajs.iter().enumerate() {
            let pose = motion_asset
                .pose_data
                .get_chunk(traj.chunk_index)
                .and_then(|poses| poses.get(traj.chunk_offset))
                .unwrap();
            joint_matrices.apply_frame(&pose);

            let pose_translation_offset = Vec3::new(i as f32 * POSE_OFFSET, 0.0, 0.0);
            for (joint_index, joint) in joint_matrices.joints().iter().enumerate() {
                let Some(parent_index) = joint.parent_index() else {
                    continue;
                };

                let (.., parent_translation) =
                    joint_matrices.world_matrices()[parent_index].to_scale_rotation_translation();
                let (.., curr_translation) =
                    joint_matrices.world_matrices()[joint_index].to_scale_rotation_translation();

                let mut parent_position =
                    (parent_translation * BVH_SCALE_RATIO) + pose_translation_offset;
                let mut current_position =
                    (curr_translation * BVH_SCALE_RATIO) + pose_translation_offset;

                parent_position = snapped_player_matrix.transform_point3(parent_position);
                current_position = snapped_player_matrix.transform_point3(current_position);
                let color = match *selected_pose == i {
                    true => palette.green,
                    false => palette.base4.with_alpha(0.8),
                };

                gizmos.line(parent_position, current_position, color);
            }
        }
    }
}

fn spawn_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // spawn the first scene in the file
    let scene: Handle<Scene> = asset_server.load("glb/model_skeleton_mixamo.glb#Scene0");
    info!("Loaded scene: {:?}", scene);
    commands
        .spawn((MainScene, SceneBundle { scene, ..default() }))
        .insert((
            PlayerBundle::default(),
            TrajectoryBundle::new(100),
            MotionPlayerBundle::default(),
        ));
}

fn spawn_light(mut commands: Commands) {
    commands
        .spawn(DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        })
        .insert(Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            f32::to_radians(-45.0),
            f32::to_radians(45.0),
            0.0,
        )));
}

fn spawn_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    palette: Res<ColorPalette>,
) {
    let size = 25.0;
    let mut plane_mesh = Plane3d::default().mesh().size(size, size).build();
    let uvs = plane_mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0).unwrap();

    if let VertexAttributeValues::Float32x2(values) = uvs {
        for uv in values.iter_mut() {
            uv[0] *= size;
            uv[1] *= size;
        }
    };

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(plane_mesh),
            material: materials.add(StandardMaterial {
                base_color: palette.base2,
                base_color_texture: Some(asset_server.load_with_settings(
                    "textures/Grid.png",
                    |s: &mut _| {
                        *s = ImageLoaderSettings {
                            sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                                // rewriting mode to repeat image,
                                address_mode_u: ImageAddressMode::Repeat,
                                address_mode_v: ImageAddressMode::Repeat,
                                ..default()
                            }),
                            ..default()
                        }
                    },
                )),
                reflectance: 0.5,
                metallic: 0.5,
                ..default()
            }),
            ..default()
        },
        GroundPlane,
    ));
}

#[derive(Component)]
pub struct GroundPlane;
