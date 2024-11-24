use bevy::color::palettes::css::WHITE;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::texture::{
    ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy_bvh_anim::prelude::{BvhAsset, JointMatrices};

use crate::bvh_manager::bvh_player::JointMap;
use crate::draw_axes::{ColorPalette, DrawAxes};
use crate::motion::chunk::ChunkIterator;
use crate::motion::motion_player::MotionPlayerBundle;
use crate::motion::{motion_asset, MotionData};
use crate::motion_matching::{NearestPose, NearestTrajectories};
use crate::player::{MovementConfig, PlayerBundle, PlayerMarker};
use crate::trajectory::{TrajectoryBundle, TrajectoryConfig, TrajectoryPoint};
use crate::transform2d::Transform2d;
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
    mut nearest_trajectories_evr: EventReader<NearestTrajectories>,
    trajectory_config: Res<TrajectoryConfig>,
    palette: Res<ColorPalette>,
    mut axes: ResMut<DrawAxes>,
    q_player_transform: Query<&Transform, With<PlayerMarker>>,
    mut nearest_traj: Local<Option<(NearestTrajectories, Mat4)>>,
) {
    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    let Ok(player_transform) = q_player_transform.get_single() else {
        return;
    };
    let curr_player_matrix = player_transform.compute_matrix();

    let num_segments = trajectory_config.num_segments();
    let num_points = trajectory_config.num_points();

    for trajs in nearest_trajectories_evr.read() {
        if trajs.is_empty() {
            continue;
        }
        *nearest_traj = Some((trajs.clone(), curr_player_matrix));
    }

    let Some((trajs, snapped_player_matrix)) = nearest_traj.as_ref() else {
        return;
    };

    for traj in trajs.iter() {
        let chunk = motion_asset.trajectory_data.get_chunk(traj.chunk_index);

        if let Some(chunk) = chunk {
            println!("Original Traj Count: {}", chunk.len());
            println!("Chunk Offset: {}", traj.chunk_offset);
            let num_trajectories = chunk.len() - traj.chunk_offset - num_segments;
            println!("Remaining Traj Count: {}", num_trajectories);

            let data_traj = &chunk[traj.chunk_offset..traj.chunk_offset + num_points];
            // Center point of trajectory
            let data_inv_matrix = data_traj[trajectory_config.history_count].matrix.inverse();

            for point in data_traj {
                let (.., translation) = point.matrix.to_scale_rotation_translation();
                let mut translation =
                    data_inv_matrix.transform_point3(translation) * BVH_SCALE_RATIO;
                translation.y = 0.0;

                let velocity = point.velocity * BVH_SCALE_RATIO;
                let velocity = Vec3::new(velocity.x, 0.0, velocity.y);
                let velocity_magnitude = velocity.length();

                translation = snapped_player_matrix.transform_point3(translation);
                let velocity = snapped_player_matrix.transform_vector3(velocity).xz();
                let angle = f32::atan2(velocity.x, velocity.y);

                axes.draw_forward(
                    Mat4::from_rotation_translation(Quat::from_rotation_y(angle), translation),
                    velocity_magnitude * 0.1,
                    palette.purple,
                );
            }
        }

        // break;
    }
}

fn draw_nearest_pose_armature(
    motion_data: MotionData,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    nearest_pose: Res<NearestPose>,
    mut gizmos: Gizmos,
    palette: Res<ColorPalette>,
) {
    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    const POSE_OFFSET: f32 = 1.0;
    const OFFSET: Vec3 = Vec3::new(-15.0, 20.0, 1.0);
    const SCALING: f32 = 0.1;

    let (camera, camera_transform) = q_camera.single();
    let cam_matrix = camera_transform.compute_matrix();
    let inv_cam_matrix = camera_transform.compute_matrix().inverse();

    let mut joint_matrices = JointMatrices::new(motion_asset.joints());

    for (i, pose) in nearest_pose.nearest_pose.iter().enumerate() {
        // println!("Nearest Pose Count: {}", i);

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

            // let offset = camera_transform.right() * OFFSET.x
            //     + camera_transform.up() * OFFSET.y
            //     + camera_transform.forward() * OFFSET.z;

            parent_position = cam_matrix.project_point3(parent_position + OFFSET);
            current_position = cam_matrix.project_point3(current_position + OFFSET);

            parent_position *= SCALING;
            current_position *= SCALING;

            if let Some(best_pose_i) = nearest_pose.best_post_index {
                // println!("Best Pose Index......{}", best_pose_i);
                let color = match best_pose_i == i {
                    true => palette.green,
                    false => palette.base4.with_alpha(0.8),
                };

                gizmos.line(
                    // Constant scaling factor of the Bvh data.
                    // parent_translation * BVH_SCALE_RATIO,
                    // curr_translation * BVH_SCALE_RATIO,
                    parent_position,
                    current_position,
                    color,
                );
            } else {
                gizmos.line(
                    // Constant scaling factor of the Bvh data.
                    // parent_translation * BVH_SCALE_RATIO,
                    // curr_translation * BVH_SCALE_RATIO,
                    parent_position,
                    current_position,
                    palette.base4.with_alpha(0.8),
                );
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
