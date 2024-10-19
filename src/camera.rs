use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        tonemapping::{DebandDither, Tonemapping},
    },
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
};

use crate::{
    scene_loader::MainScene,
    ui::{MouseInUi, UiSystemSet},
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Msaa>()
            .init_resource::<CameraFocus>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                PreUpdate,
                pan_orbit_camera
                    .run_if(any_with_component::<PanOrbitState>)
                    .after(UiSystemSet),
            );
    }
}

/// Bundle to spawn our custom camera easily.
#[derive(Bundle, Default)]
pub struct PanOrbitCameraBundle {
    pub camera: Camera3dBundle,
    pub state: PanOrbitState,
    pub settings: PanOrbitSettings,
}

/// The internal state of the pan-orbit controller.
#[derive(Component)]
pub struct PanOrbitState {
    pub center: Vec3,
    /// Offset from [`PanOrbitState::center`].
    pub offset: Vec3,
    pub radius: f32,
    pub upside_down: bool,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for PanOrbitState {
    fn default() -> Self {
        PanOrbitState {
            center: Vec3::ZERO,
            offset: Vec3::Y,
            radius: 1.0,
            upside_down: false,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

/// The configuration of the pan-orbit controller.
#[derive(Component)]
pub struct PanOrbitSettings {
    /// World units per pixel of mouse motion.
    pub pan_sensitivity: f32,
    /// Radians per pixel of mouse motion.
    pub orbit_sensitivity: f32,
    /// Exponent per pixel of mouse motion.
    pub zoom_sensitivity: f32,
    /// Key to hold for panning.
    pub pan_key: Option<MouseButton>,
    /// Key to hold for orbiting.
    pub orbit_key: Option<KeyCode>,
    /// Key to hold for zooming.
    pub zoom_key: Option<KeyCode>,
    /// Key to press for focusing.
    pub focus_key: Option<KeyCode>,
    /// For devices with a notched scroll wheel, like desktop mice.
    pub scroll_line_sensitivity: f32,
    /// For devices with smooth scrolling, like touchpads.
    pub scroll_pixel_sensitivity: f32,
}

impl Default for PanOrbitSettings {
    fn default() -> Self {
        PanOrbitSettings {
            // 1000 pixels per world unit
            pan_sensitivity: 0.001,
            // 0.2 degree per pixel
            orbit_sensitivity: 0.2f32.to_radians(),
            zoom_sensitivity: 0.01,
            pan_key: Some(MouseButton::Middle),
            orbit_key: Some(KeyCode::AltLeft),
            zoom_key: Some(KeyCode::ShiftLeft),
            focus_key: Some(KeyCode::KeyF),
            // 1 "line" == 16 "pixels of motion"
            scroll_line_sensitivity: 16.0,
            scroll_pixel_sensitivity: 1.0,
        }
    }
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct CameraFocus(Option<Entity>);

impl CameraFocus {
    pub fn get(&self) -> Option<Entity> {
        self.0
    }

    pub fn set(&mut self, entity: Entity) {
        self.0 = Some(entity);
    }

    pub fn clear(&mut self) {
        self.0 = None;
    }
}

fn spawn_camera(mut commands: Commands) {
    let mut camera = PanOrbitCameraBundle::default();
    // Position our camera using our component,
    // not Transform (it would get overwritten)
    camera.state.radius = 5.0;
    camera.state.pitch = -15.0f32.to_radians();
    camera.state.yaw = 30.0f32.to_radians();
    camera.camera = Camera3dBundle {
        camera: Camera {
            hdr: true,
            ..default()
        },
        deband_dither: DebandDither::Enabled,
        tonemapping: Tonemapping::AcesFitted,
        ..default()
    };
    commands.spawn(camera).insert(BloomSettings::default());
}

fn pan_orbit_camera(
    mut q_camera: Query<(&PanOrbitSettings, &mut PanOrbitState, &mut Transform)>,
    q_global_transforms: Query<&GlobalTransform>,
    q_main_scene: Query<Entity, With<MainScene>>,
    mut evr_motion: EventReader<MouseMotion>,
    mut evr_scroll: EventReader<MouseWheel>,
    kbd: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut camera_focus: ResMut<CameraFocus>,
    mouse_in_ui: Res<MouseInUi>,
) {
    // First, accumulate the total amount of
    // mouse motion and scroll, from all pending events:
    let mut total_motion: Vec2 = evr_motion.read().map(|ev| ev.delta).sum();

    // Reverse Y (Bevy's Worldspace coordinate system is Y-Up,
    // but events are in window/ui coordinates, which are Y-Down)
    total_motion.y = -total_motion.y;

    let mut total_scroll_lines = Vec2::ZERO;
    let mut total_scroll_pixels = Vec2::ZERO;
    if mouse_in_ui.get() == false {
        for ev in evr_scroll.read() {
            match ev.unit {
                MouseScrollUnit::Line => {
                    total_scroll_lines.x += ev.x;
                    total_scroll_lines.y -= ev.y;
                }
                MouseScrollUnit::Pixel => {
                    total_scroll_pixels.x += ev.x;
                    total_scroll_pixels.y -= ev.y;
                }
            }
        }
    }

    let left_clicked = mouse.pressed(MouseButton::Left);

    for (settings, mut state, mut transform) in &mut q_camera {
        // Camera focus
        if settings
            .focus_key
            .map(|key| kbd.just_pressed(key))
            .unwrap_or(false)
        {
            if camera_focus.get().is_some() {
                camera_focus.clear();
            } else if let Ok(entity) = q_main_scene.get_single() {
                camera_focus.set(entity);
            }
        }

        // Check how much of each thing we need to apply.
        // Accumulate values from motion and scroll,
        // based on our configuration settings.

        let mut total_pan = Vec2::ZERO;
        if settings
            .pan_key
            .map(|key| mouse.pressed(key))
            .unwrap_or(false)
        {
            total_pan -= total_motion * settings.pan_sensitivity;
        }

        let mut total_orbit = Vec2::ZERO;
        if settings
            .orbit_key
            .map(|key| kbd.pressed(key))
            .unwrap_or(false)
            && left_clicked
        {
            total_orbit -= total_motion * settings.orbit_sensitivity;
        }

        let mut total_zoom = Vec2::ZERO;
        if settings
            .zoom_key
            .map(|key| kbd.pressed(key))
            .unwrap_or(false)
            && left_clicked
        {
            total_zoom -= total_motion * settings.zoom_sensitivity;
        }
        total_zoom -=
            total_scroll_lines * settings.scroll_line_sensitivity * settings.zoom_sensitivity;
        total_zoom -=
            total_scroll_pixels * settings.scroll_pixel_sensitivity * settings.zoom_sensitivity;

        // Upon starting a new orbit maneuver (key is just pressed),
        // check if we are starting it upside-down
        if settings
            .orbit_key
            .map(|key| kbd.just_pressed(key))
            .unwrap_or(false)
            && left_clicked
        {
            state.upside_down = state.pitch < -FRAC_PI_2 || state.pitch > FRAC_PI_2;
        }

        // If we are upside down, reverse the X orbiting
        if state.upside_down {
            total_orbit.x = -total_orbit.x;
        }

        // Now we can actually do the things!

        let mut any = false;

        // To ZOOM, we need to multiply our radius.
        if total_zoom != Vec2::ZERO {
            any = true;
            // in order for zoom to feel intuitive,
            // everything needs to be exponential
            // (done via multiplication)
            // not linear
            // (done via addition)

            // so we compute the exponential of our
            // accumulated value and multiply by that
            state.radius *= (-total_zoom.y).exp();
        }

        // To ORBIT, we change our pitch and yaw values
        if total_orbit != Vec2::ZERO {
            any = true;
            state.yaw += total_orbit.x;
            state.pitch -= total_orbit.y;
            // wrap around, to stay between +- 180 degrees
            if state.yaw > PI {
                state.yaw -= TAU; // 2 * PI
            }
            if state.yaw < -PI {
                state.yaw += TAU; // 2 * PI
            }
            if state.pitch > PI {
                state.pitch -= TAU; // 2 * PI
            }
            if state.pitch < -PI {
                state.pitch += TAU; // 2 * PI
            }
        }

        // To PAN, we can get the UP and RIGHT direction
        // vectors from the camera's transform, and use
        // them to move the center point. Multiply by the
        // radius to make the pan adapt to the current zoom.
        let mut pan_offset = Vec3::ZERO;
        if total_pan != Vec2::ZERO {
            any = true;
            let radius = state.radius;
            pan_offset += transform.right() * total_pan.x * radius;
            pan_offset += transform.up() * total_pan.y * radius;
        }

        match camera_focus.get() {
            Some(entity) => {
                any = true;
                if let Ok(transform) = q_global_transforms.get(entity) {
                    state.offset += pan_offset;
                    state.center = transform.translation();
                }
            }
            None => {
                state.center += pan_offset;
            }
        }

        // Finally, compute the new camera transform.
        // (if we changed anything, or if the pan-orbit
        // controller was just added and thus we are running
        // for the first time and need to initialize)
        if any || state.is_added() {
            // YXZ Euler Rotation performs yaw/pitch/roll.
            transform.rotation = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
            // To position the camera, get the backward direction vector
            // and place the camera at the desired radius from the center.
            transform.translation = state.center + state.offset + transform.back() * state.radius;
        }
    }
}
