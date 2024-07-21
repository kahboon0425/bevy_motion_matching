use bevy::{ecs::system::SystemState, prelude::*};
use bevy_bvh_anim::prelude::*;
use bevy_egui::egui;

use crate::{bvh_library::BvhLibrary, bvh_player::SelectedBvhAsset, scene_loader::MainScene};

#[derive(Resource, Default)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_time: f32,
    pub duration: f32,
}

#[derive(Resource)]
pub struct DrawBvhTrail(bool);

impl DrawBvhTrail {
    pub fn get(&self) -> bool {
        self.0
    }
}

impl Default for DrawBvhTrail {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Resource)]
pub struct DrawTrajectory(bool);

impl DrawTrajectory {
    pub fn get(&self) -> bool {
        self.0
    }
}

impl Default for DrawTrajectory {
    fn default() -> Self {
        Self(true)
    }
}

pub fn config_panel(ui: &mut egui::Ui, world: &mut World) {
    ui.heading("Configurations");
    ui.add_space(10.0);
    bvh_map_label(ui, world);
    bvh_playback(ui, world);
    ui.add_space(10.0);
    show_character_checkbox(ui, world);
    draw_bvh_trail_checkbox(ui, world);
    draw_trajectory_checkbox(ui, world);
}

fn bvh_playback(ui: &mut egui::Ui, world: &mut World) {
    let mut params = SystemState::<(
        Res<AssetServer>,
        Res<Assets<BvhAsset>>,
        ResMut<SelectedBvhAsset>,
        ResMut<PlaybackState>,
    )>::new(world);
    let (asset_server, bvh_assets, mut selected_bvh_asset, mut playback_state) =
        params.get_mut(world);

    ui.horizontal(|ui| {
        ui.label("Choose Bvh File:");

        let mut selected_name = String::new();
        if let Some(path) = asset_server.get_path(selected_bvh_asset.0) {
            selected_name = path.to_string();
        }

        egui::ComboBox::from_label("")
            .selected_text(selected_name)
            .show_ui(ui, |ui| {
                for id in bvh_assets.ids() {
                    let Some(bvh_name) = asset_server.get_path(id) else {
                        continue;
                    };
                    if ui.selectable_label(false, bvh_name.to_string()).clicked() {
                        selected_bvh_asset.0 = id;
                        if let Some(bvh) = bvh_assets.get(id).map(|asset| asset.get()) {
                            playback_state.duration =
                                bvh.frame_time().as_secs_f32() * bvh.num_frames() as f32;
                        }
                    }
                }
            });
    });

    ui.add_space(5.0);
    ui.horizontal(|ui| {
        let button_icon = match playback_state.is_playing {
            true => "Pause",
            false => "Play",
        };

        if ui.button(button_icon).clicked() {
            playback_state.is_playing = !playback_state.is_playing;
        }

        let playback_duration = playback_state.duration;
        ui.add(egui::Slider::new(
            &mut playback_state.current_time,
            0.0..=playback_duration,
        ));
    });
}

fn bvh_map_label(ui: &mut egui::Ui, world: &mut World) {
    let bvh_library = world.resource::<BvhLibrary>();
    ui.horizontal(|ui| {
        ui.label("Bvh Map: ");
        if let Some(map_path) = bvh_library.get_map().and_then(|m| m.path()) {
            ui.label(map_path.to_string());
        }
    });
}

fn show_character_checkbox(ui: &mut egui::Ui, world: &mut World) {
    let mut q_main_scene = world.query_filtered::<&mut Visibility, With<MainScene>>();
    let Ok(mut main_scene_vis) = q_main_scene.get_single_mut(world) else {
        return;
    };

    let mut is_main_scene_visible = matches!(*main_scene_vis, Visibility::Hidden) == false;
    ui.checkbox(&mut is_main_scene_visible, "Show Character");
    match is_main_scene_visible {
        true => *main_scene_vis = Visibility::Inherited,
        false => *main_scene_vis = Visibility::Hidden,
    }
}

fn draw_bvh_trail_checkbox(ui: &mut egui::Ui, world: &mut World) {
    let mut show_draw_arrow = world.resource_mut::<DrawBvhTrail>();
    ui.checkbox(&mut show_draw_arrow.0, "Show Bvh Trail");
}

fn draw_trajectory_checkbox(ui: &mut egui::Ui, world: &mut World) {
    let mut show_draw_arrow = world.resource_mut::<DrawTrajectory>();
    ui.checkbox(&mut show_draw_arrow.0, "Show Trajectory Arrows");
}
