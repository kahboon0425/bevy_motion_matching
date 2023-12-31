use bevy::prelude::*;
use bevy::DefaultPlugins;
// use bvh_anim;
// use std::error::Error;
// use std::fs;
// use std::io;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            // shadows_enabled: true,
            ..Default::default()
        },
        ..default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(100.0, 100.0, 150.0)
            .looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
        ..default()
    });

    // spawn the first scene in the file
    let scene0 = asset_server.load("./fbx/model_skeleton.glb#Scene0");
    println!("Loaded asset: {:?}", scene0);
    commands.spawn(SceneBundle {
        scene: scene0,
        transform: Transform::from_xyz(1.0, 1.0, 1.0).with_scale(Vec3::new(0.5, 0.5, 0.5)),
        ..Default::default()
    });
}
