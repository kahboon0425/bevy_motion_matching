use bevy::prelude::*;
use bevy_third_person_camera::ThirdPersonCameraTarget;

/// Load scene from glb file.
pub struct SceneLoaderPlugin;

impl Plugin for SceneLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_scene);
    }
}

#[derive(Component)]
pub struct MainScene;

pub fn spawn_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            // shadows_enabled: true,
            ..Default::default()
        },
        ..default()
    });

    // spawn the first scene in the file
    let scene: Handle<Scene> = asset_server.load("glb/model_skeleton.glb#Scene0");
    info!("Loaded scene: {:?}", scene);
    commands
        .spawn(SceneBundle { scene, ..default() })
        .insert(MainScene)
        .insert(ThirdPersonCameraTarget);
}
