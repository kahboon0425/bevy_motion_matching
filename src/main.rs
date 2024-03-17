use std::fs;
use std::io::BufReader;

use bevy::asset::LoadState;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::Window;
use bevy::DefaultPlugins;
use bvh_anim::{self, errors::LoadError, Bvh};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (spawn_camera, setup, store_bvh))
        .add_systems(
            Update,
            (
                match_bones,
                // spawn_gltf_objects,
                keyboard_input,
            ),
        )
        // .add_systems(Update, spawn_gltf_objects)
        .add_systems(Update, pan_orbit_camera)
        .run();
}

#[derive(Resource)]
pub struct BvhToCharacter {
    pub scene_handle: Handle<Scene>,
    pub loaded: bool,
}

#[derive(Component)]
pub struct MainCharacter;

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
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

pub fn check_character(
    q_character: Query<(Entity, &Children), With<MainCharacter>>,
    q_children: Query<&Children>,
) {
    let Ok((_characater_entity, children)) = q_character.get_single() else {
        return;
    };

    fn recurse_loop_children(children: &Children, q_children: &Query<&Children>) {
        for child in children {
            // process the current child
            println!("{:?}", child);
            // * get the actual value (entity ID) that the reference is pointing to
            if let Ok(_new_children) = q_children.get(*child) {
                // recursively process the children of the current child                recurse_loop_children(new_children, q_children);
            }
        }
    }

    recurse_loop_children(children, &q_children);
}

fn load_bvh() -> Result<Vec<Bvh>, LoadError> {
    let animation_file_path: &str = "./assets/walking-animation-dataset/";

    let mut loaded_bvhs: Vec<Bvh> = Vec::new();

    let mut count: usize = 0;
    if let Ok(entries) = fs::read_dir(animation_file_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(filename) = entry.file_name().to_str() {
                    println!("Loading animation file: {}", filename);

                    let filename: String = animation_file_path.to_owned() + filename;

                    let bvh_file: fs::File = fs::File::open(&filename).unwrap();
                    let bvh_reader: BufReader<fs::File> = BufReader::new(bvh_file);

                    let bvh: Bvh = bvh_anim::from_reader(bvh_reader)?;

                    loaded_bvhs.push(bvh);

                    if count >= 2 {
                        break;
                    }
                    count += 1;
                }
            }
        }

        if loaded_bvhs.is_empty() {
            println!("No BVH files found");
        }
    } else {
        println!("Failed to read directory");
    }

    Ok(loaded_bvhs)
}

#[derive(Resource)]
pub struct BvhData(pub Option<Vec<Bvh>>);

pub fn store_bvh(mut commands: Commands) {
    match load_bvh() {
        Ok(bvhs) => {
            commands.insert_resource(BvhData(Some(bvhs)));
        }
        Err(err) => {
            commands.insert_resource(BvhData(None));
            println!("{:#?}", err);
        }
    }
}

#[derive(Component)]
pub struct BoneIndex(pub usize);

#[derive(Component)]
pub struct BoneRotation(pub Quat);

pub fn match_bones(
    mut commands: Commands,
    mut q_names: Query<(Entity, &Name, &mut Transform)>,
    bvh_data: Res<BvhData>,
    mut bvh_to_character: ResMut<BvhToCharacter>,
    server: Res<AssetServer>,
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

    if let Some(bvh_vec) = &bvh_data.0 {
        let bvh: Bvh = bvh_vec[1].clone();
        let frame: &bvh_anim::Frame = bvh.frames().last().unwrap();

        println!("{:#?}", frame);

        for (entity, name, mut transform) in q_names.iter_mut() {
            let bone_name = &name.as_str()[6..];

            let mut joint_index: usize = 0;

            for joint in bvh.joints() {
                if bone_name == joint.data().name() {
                    if bone_name == "Hips" {
                        continue;
                    }

                    println!("{:#?} = {:#?}", bone_name, joint.data().name());

                    commands.entity(entity).insert(BoneIndex(joint_index));

                    let offset_y = joint.data().offset().x;
                    let offset_x = joint.data().offset().y;
                    let offset_z = joint.data().offset().z;

                    let rotation_0 = frame[&joint.data().channels()[0]];
                    let rotation_1 = frame[&joint.data().channels()[1]];
                    let rotation_2 = frame[&joint.data().channels()[2]];

                    let rotation = Quat::from_euler(
                        EulerRot::ZYX,
                        rotation_0.to_radians(),
                        rotation_1.to_radians(),
                        rotation_2.to_radians(),
                    );

                    println!("origin transform: {:?}", transform.translation);
                    println!("bvh offset: {}, {}, {}", offset_x, offset_y, offset_z);

                    // transform.translation = Vec3::new(offset_x, offset_y, offset_z);
                    transform.rotation = rotation;

                    // Update the rotation of the entity for each frame
                    commands.entity(entity).insert(BoneRotation(rotation));
                    println!("Bone Name: {}, Rotation: {:?}", bone_name, rotation);
                }

                joint_index += 1;
            }
        }
    } else {
        println!("BVH data not available");
    }
}

pub fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    q_bone: Query<(Entity, &BoneIndex, &Name, &Transform)>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let target_bone_index = 5;
        for (entity, bone_index, bone_name, transform) in q_bone.iter() {
            if bone_index.0 == target_bone_index {
                println!("{:#?}: {:#?}", bone_index.0, bone_name);
            }
        }
    }
}

/// Tags an entity as capable of panning and orbiting.
#[derive(Component)]
struct PanOrbitCamera {
    /// The "focus point" to orbit around. It is automatically updated when panning the camera
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: 5.0,
            upside_down: false,
        }
    }
}

/// Pan the camera with middle mouse click, zoom with scroll wheel, orbit with right mouse click.
fn pan_orbit_camera(
    windows: Query<&Window>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<Input<MouseButton>>,
    mut query: Query<(&mut PanOrbitCamera, &mut Transform, &Projection)>,
) {
    // change input mapping for orbit and panning here
    let orbit_button = MouseButton::Right;
    let pan_button = MouseButton::Middle;

    let mut pan = Vec2::ZERO;
    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;
    let mut orbit_button_changed = false;

    if input_mouse.pressed(orbit_button) {
        for ev in ev_motion.read() {
            rotation_move += ev.delta;
        }
    } else if input_mouse.pressed(pan_button) {
        // Pan only if we're not rotating at the moment
        for ev in ev_motion.read() {
            pan += ev.delta;
        }
    }
    for ev in ev_scroll.read() {
        scroll += ev.y;
    }
    if input_mouse.just_released(orbit_button) || input_mouse.just_pressed(orbit_button) {
        orbit_button_changed = true;
    }

    for (mut pan_orbit, mut transform, projection) in query.iter_mut() {
        if orbit_button_changed {
            // only check for upside down when orbiting started or ended this frame
            // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it correct
            let up = transform.rotation * Vec3::Y;
            pan_orbit.upside_down = up.y <= 0.0;
        }

        let mut any = false;
        if rotation_move.length_squared() > 0.0 {
            any = true;
            let window = get_primary_window_size(&windows);
            let delta_x = {
                let delta = rotation_move.x / window.x * std::f32::consts::PI * 2.0;
                if pan_orbit.upside_down {
                    -delta
                } else {
                    delta
                }
            };
            let delta_y = rotation_move.y / window.y * std::f32::consts::PI;
            let yaw = Quat::from_rotation_y(-delta_x);
            let pitch = Quat::from_rotation_x(-delta_y);
            transform.rotation = yaw * transform.rotation; // rotate around global y axis
            transform.rotation = transform.rotation * pitch; // rotate around local x axis
        } else if pan.length_squared() > 0.0 {
            any = true;
            // make panning distance independent of resolution and FOV,
            let window = get_primary_window_size(&windows);
            if let Projection::Perspective(projection) = projection {
                pan *= Vec2::new(projection.fov * projection.aspect_ratio, projection.fov) / window;
            }
            // translate by local axes
            let right = transform.rotation * Vec3::X * -pan.x;
            let up = transform.rotation * Vec3::Y * pan.y;
            // make panning proportional to distance away from focus point
            let translation = (right + up) * pan_orbit.radius;
            pan_orbit.focus += translation;
        } else if scroll.abs() > 0.0 {
            any = true;
            pan_orbit.radius -= scroll * pan_orbit.radius * 0.2;
            // dont allow zoom to reach zero or you get stuck
            pan_orbit.radius = f32::max(pan_orbit.radius, 0.05);
        }

        if any {
            // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
            // parent = x and y rotation
            // child = z-offset
            let rot_matrix = Mat3::from_quat(transform.rotation);
            transform.translation =
                pan_orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
        }
    }

    // consume any remaining events, so they don't pile up if we don't need them
    // (and also to avoid Bevy warning us about not checking events every frame update)
    ev_motion.clear();
}

fn get_primary_window_size(windows: &Query<&Window>) -> Vec2 {
    let window = windows.get_single().unwrap();
    let window = Vec2::new(window.width() as f32, window.height() as f32);
    window
}

/// Spawn a camera like this
fn spawn_camera(mut commands: Commands) {
    let translation = Vec3::new(-2.0, 2.5, 5.0);
    let radius = translation.length();

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        PanOrbitCamera {
            radius,
            ..Default::default()
        },
    ));
}
