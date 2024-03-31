use bevy::prelude::*;

pub struct CharacterLoaderPlugin;

impl Plugin for CharacterLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_character);
    }
}
#[derive(Resource)]
pub struct BvhToCharacter {
    pub scene_handle: Handle<Scene>,
    pub loaded: bool,
}

#[derive(Component)]
pub struct MainCharacter;

pub fn spawn_character(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            // shadows_enabled: true,
            ..Default::default()
        },
        ..default()
    });

    // spawn the first scene in the file
    let scene: Handle<Scene> = asset_server.load("./glb/model_skeleton_origin.glb#Scene0");
    println!("Loaded asset: {:?}", scene);
    commands
        .spawn(SceneBundle {
            scene: scene.clone(),
            ..default()
        })
        .insert(MainCharacter);

    commands.insert_resource(BvhToCharacter {
        loaded: false,
        scene_handle: scene,
    });
}
