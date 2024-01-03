use bevy::gltf::Gltf;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::Window;
use bevy::DefaultPlugins;
use bvh_anim;
use std::error::Error;
use std::fs;
use std::io::BufReader;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(PreStartup, load_character_gltf)
        .add_systems(Startup, (spawn_camera, setup, store_bvh))
        .add_systems(Update, (match_bones, spawn_gltf_objects, keyboard_input))
        // .add_systems(Update, spawn_gltf_objects)
        .add_systems(Update, pan_orbit_camera)
        .run();
}

/// Helper resource for tracking our asset
#[derive(Resource)]
struct CharacterGltf(Handle<Gltf>);

fn load_character_gltf(mut commands: Commands, ass: Res<AssetServer>) {
    let gltf: Handle<Gltf> = ass.load("glb/model_skeleton.glb");
    commands.insert_resource(CharacterGltf(gltf));
}

fn spawn_gltf_objects(
    _commands: Commands,
    character_gltf: Res<CharacterGltf>,
    assets_gltf: Res<Assets<Gltf>>,
) {
    // println!("============================");
    // if the GLTF has loaded, we can navigate its contents
    if let Some(gltf) = assets_gltf.get(&character_gltf.0) {
        let mut _count: u32 = 0;
        // spawn the first scene in the file
        // commands.spawn(SceneBundle {
        //     scene: gltf.scenes[0].clone(),
        //     ..Default::default()
        // });
        let animation_file_path = "./assets/walking-animation-dataset/walk1_subject1.bvh";
        let bvh_file = fs::File::open(animation_file_path).unwrap();
        let bvh_reader = BufReader::new(bvh_file);
        let _bvh = bvh_anim::from_reader(bvh_reader).unwrap();
        // for joint in bvh.joints() {
        // println!("{:#?}", joint.data().name());
        // }
        for _key in gltf.named_nodes.keys() {
            // println!("{}", key);
            _count += 1;
        }

        // println!("{:#?}", gltf);

        // println!("{}", count);

        // spawn the scene named "YellowCar"
        // commands.spawn(SceneBundle {
        //     scene: gltf.named_scenes["YellowCar"].clone(),
        //     transform: Transform::from_xyz(1.0, 2.0, 3.0),
        //     ..Default::default()
        // });

        // PERF: the `.clone()`s are just for asset handles, don't worry :)
    }
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
    let scene0: Handle<Scene> = asset_server.load("./glb/model_skeleton.glb#Scene0");
    // let scene0: Handle<Scene> = asset_server.load("./glb/simple_skeleton.glb#Scene0");
    println!("Loaded asset: {:?}", scene0);
    commands
        .spawn(SceneBundle {
            scene: scene0,
            ..default()
        })
        .insert(MainCharacter);
}

pub fn check_character(
    // mut commands: Commands,
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

pub fn load_bvh() -> Result<Vec<bvh_anim::Bvh>, Box<dyn Error>> {
    let animation_file_path = "./assets/walking-animation-dataset/";

    let mut loaded_bvhs = Vec::new();

    if let Ok(entries) = fs::read_dir(animation_file_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(file_name) = entry.file_name().to_str() {
                    println!("Animation File: {}", file_name);
                    let animation_file_name = animation_file_path.to_owned() + file_name;
                    let bvh_file = fs::File::open(&animation_file_name)?;
                    let bvh_reader = BufReader::new(bvh_file);
                    let bvh = bvh_anim::from_reader(bvh_reader)?;

                    loaded_bvhs.push(bvh);
                }
            }
        }

        if loaded_bvhs.is_empty() {
            Err("No BVH files found".into())
        } else {
            Ok(loaded_bvhs)
        }
    } else {
        Err("Failed to read directory".into())
    }
}

#[derive(Resource)]
pub struct BvhData(pub Option<Vec<bvh_anim::Bvh>>);

pub fn store_bvh(mut commands: Commands) {
    if let Ok(bvhs) = load_bvh() {
        commands.insert_resource(BvhData(Some(bvhs)));
    } else {
        println!("Error loading BVH during startup");
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
) {
    // match load_bvh() {
    //     Ok(bvh) => {
    if let Some(bvh_vec) = &bvh_data.0 {
        for bvh in bvh_vec {
            for frame in bvh.frames() {
                // println!("{:#?}", frame);

                for (entity, name, mut transform) in q_names.iter_mut() {
                    // println!("{}", name);
                    let bone_name = &name.as_str()[6..];
                    // println!("{:#?}", bone_name);

                    let mut joint_index: usize = 0;
                    // let mut count: usize = 0;

                    for joint in bvh.joints() {
                        // count += 1;

                        // println!("{:#?}", joint);
                        if bone_name == joint.data().name() {
                            // println!("{:#?} = {:#?}", bone_name, joint.data().name());

                            commands.entity(entity).insert(BoneIndex(joint_index));

                            // println!("{:#?}", joint.data().name());
                            // println!("{:#?}", joint.data());
                            // println!("{:#?}", joint.data().offset());
                            let offset_x = joint.data().offset().x;
                            let offset_y = joint.data().offset().y;
                            let offset_z = joint.data().offset().z;

                            // let rotation_index_0 = joint.data().channels()[0].motion_index();
                            // println!("{:#?}", rotation_index_0);
                            // let rotation_index_1 = joint.data().channels()[1].motion_index();
                            // let rotation_index_2 = joint.data().channels()[2].motion_index();

                            // println!("{:#?}", rotation_x);

                            // println!("{:#?}", joint.data().offset().x);
                            // let transform = Transform::from_xyz(offset_x, offset_y, offset_z);
                            // commands.entity(entity).insert(transform);

                            transform.translation = Vec3::new(offset_x, offset_y, offset_z);

                            // println!("{:#?}", frame);
                            // println!("{:#?}", frame[&joint.data().channels()[0]]);
                            // let test =
                            //     frame.get(&bvh.joints().next().unwrap().data().channels()[0]);

                            // println!("------------------------------------------ {:#?}", test);
                            let rotation_x = frame[&joint.data().channels()[0]];
                            let rotation_y = frame[&joint.data().channels()[1]];
                            let rotation_z = frame[&joint.data().channels()[2]];

                            let quat_x = Quat::from_rotation_x(rotation_x.to_radians());
                            let quat_y = Quat::from_rotation_y(rotation_y.to_radians());
                            let quat_z = Quat::from_rotation_z(rotation_z.to_radians());

                            // Combine the rotations along different axes
                            let rotation = quat_x * quat_y * quat_z;

                            // Update the rotation of the entity for each frame
                            commands.entity(entity).insert(BoneRotation(rotation));
                            // println!("Bone Name: {}, Rotation: {:?}", bone_name, rotation);
                        }

                        joint_index += 1;
                    }
                }

                // println!("Joint Count{}", count);
            }
        }
        // println!("..........");
    } else {
        println!("BVH data not available");
    }

    // Err(err) => {
    //     println!("Error loading BVH: {}", err);
    // }
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
