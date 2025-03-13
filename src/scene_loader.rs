use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;

use crate::draw_axes::ColorPalette;
use crate::motion::motion_player::MotionPlayerBundle;
use crate::player::PlayerBundle;
use crate::trajectory::TrajectoryBundle;

/// Load glb file and setup the scene.
pub struct SceneLoaderPlugin;

impl Plugin for SceneLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_scene, spawn_light, spawn_ground));
    }
}

#[derive(Component)]
pub struct MainScene;

fn spawn_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // spawn the first scene in the file
    let scene: Handle<Scene> = asset_server.load("glb/model_skeleton_mixamo.glb#Scene0");
    info!("Loaded scene: {:?}", scene);
    commands.spawn((MainScene, SceneRoot(scene))).insert((
        PlayerBundle::default(),
        TrajectoryBundle::new(100),
        MotionPlayerBundle::default(),
    ));
}

fn spawn_light(mut commands: Commands) {
    commands
        .spawn(DirectionalLight {
            shadows_enabled: true,
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
        Mesh3d(meshes.add(plane_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
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
        })),
        GroundPlane,
    ));
}

#[derive(Component)]
pub struct GroundPlane;
