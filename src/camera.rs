use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        tonemapping::{DebandDither, Tonemapping},
    },
    prelude::*,
};
use bevy_third_person_camera::{camera::Offset, ThirdPersonCamera, ThirdPersonCameraPlugin};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ThirdPersonCameraPlugin)
            .insert_resource(Msaa::default())
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, camera_lerp);
    }
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        ThirdPersonCamera {
            offset_enabled: true,
            offset: Offset::new(0.5, 0.8),
            offset_toggle_key: KeyCode::KeyE,
            offset_toggle_speed: 0.3,
            cursor_lock_toggle_enabled: true,
            cursor_lock_active: true,
            cursor_lock_key: KeyCode::Space,
            ..Default::default()
        },
        Transform::default(),
    ));

    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            dither: DebandDither::Enabled,
            tonemapping: Tonemapping::AcesFitted,
            ..default()
        })
        .insert(BloomSettings::default());
}

fn camera_lerp(
    mut q_camera: Query<&mut Transform, (With<Camera3d>, Without<ThirdPersonCamera>)>,
    q_third_person: Query<&Transform, (With<ThirdPersonCamera>, Without<Camera3d>)>,
    time: Res<Time>,
) {
    const SPEED: f32 = 15.0;

    let Ok(third_person_transform) = q_third_person.get_single() else {
        return;
    };

    let Ok(mut camera_transform) = q_camera.get_single_mut() else {
        return;
    };

    camera_transform.translation = Vec3::lerp(
        camera_transform.translation,
        third_person_transform.translation,
        time.delta_seconds() * SPEED,
    );

    camera_transform.rotation = Quat::slerp(
        camera_transform.rotation,
        third_person_transform.rotation,
        time.delta_seconds() * SPEED,
    );
}
