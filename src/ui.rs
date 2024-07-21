use std::io::Write;

use bevy::{prelude::*, utils::HashSet};
use bevy_bvh_anim::prelude::*;
use bevy_egui::{
    egui::{self, Color32},
    EguiContexts,
};

#[cfg(not(feature = "debug"))]
use bevy_egui::EguiPlugin;

#[cfg(feature = "debug")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::{
    bvh_library::BvhLibrary, bvh_player::SelectedBvhAsset, motion_data_asset::MotionDataAsset,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.add_plugins(WorldInspectorPlugin::new());
        #[cfg(not(feature = "debug"))]
        app.add_plugins(EguiPlugin);

        app.init_resource::<MouseInUi>()
            .init_resource::<ShowDrawArrow>()
            .init_resource::<BuildConfig>()
            .init_resource::<PlaybackState>()
            .add_systems(PreUpdate, reset_mouse_in_ui)
            .add_systems(Update, right_panel.in_set(UiSystemSet));
    }
}

#[derive(SystemSet, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct UiSystemSet;

#[derive(Resource, Default, Debug)]
pub struct MouseInUi(bool);

impl MouseInUi {
    pub fn get(&self) -> bool {
        self.0
    }

    pub fn set_is_inside(&mut self) {
        self.0 = true;
    }
}

#[derive(Resource)]
pub struct ShowDrawArrow(bool);

impl ShowDrawArrow {
    pub fn get(&self) -> bool {
        self.0
    }
}

impl Default for ShowDrawArrow {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Resource, Default, Debug)]
pub struct BuildConfig {
    pub bvh_assets: HashSet<AssetId<BvhAsset>>,
}

#[derive(Resource, Default)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_time: f32,
    pub duration: f32,
}

fn reset_mouse_in_ui(mut mouse_in_ui: ResMut<MouseInUi>) {
    mouse_in_ui.0 = false;
}

fn bvh_playback_menu(
    ui: &mut egui::Ui,
    asset_server: &AssetServer,
    bvh_assets: &Assets<BvhAsset>,
    selected_bvh_asset: &mut SelectedBvhAsset,
    playback_state: &mut PlaybackState,
) {
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

    ui.add_space(10.0);
    ui.label("Playback State");
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

fn arrow_checkbox(ui: &mut egui::Ui, show_draw_arrow: &mut ShowDrawArrow) {
    ui.checkbox(&mut show_draw_arrow.0, "Show Arrows");
}

fn bvh_map_label(ui: &mut egui::Ui, bvh_library: &Res<BvhLibrary>) {
    ui.horizontal(|ui| {
        ui.label("Bvh Map: ");
        if let Some(map_path) = bvh_library.get_map().and_then(|m| m.path()) {
            ui.label(map_path.to_string());
        }
    });
}

fn _bvh_map_config(ui: &mut egui::Ui, bvh_library: &Res<BvhLibrary>, bvh_asset: &Assets<BvhAsset>) {
    ui.vertical(|ui| {
        let Some(asset) = bvh_library.get_map().and_then(|id| bvh_asset.get(id)) else {
            return;
        };

        let bvh = asset.get();

        scrollbox(ui, 300.0, |ui| {
            for joint in bvh.joints() {
                let joint_data = joint.data();
                ui.horizontal(|ui| {
                    ui.label(joint_data.name()[6..].to_str().unwrap());
                    // TODO: Show their position relative to root
                });
            }
        });

        if ui.button("Save Map").clicked() {
            // Save configuration
        }
    });
}

fn motion_data_asset_buider_menu(
    ui: &mut egui::Ui,
    asset_server: &AssetServer,
    bvh_assets: &Assets<BvhAsset>,
    build_config: &mut BuildConfig,
) {
    ui.label("Bvh Builder");
    ui.add_space(10.0);
    scrollbox(ui, 200.0, |ui| {
        for id in bvh_assets.ids() {
            let Some(bvh_name) = asset_server.get_path(id) else {
                continue;
            };

            let mut is_selected = build_config.bvh_assets.contains(&id);
            if ui
                .checkbox(&mut is_selected, bvh_name.to_string())
                .changed()
            {
                if is_selected {
                    build_config.bvh_assets.insert(id);
                } else {
                    build_config.bvh_assets.remove(&id);
                }
            }
        }
    });
}

fn scrollbox<R>(ui: &mut egui::Ui, height: f32, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    egui::Frame::default()
        .inner_margin(6.0)
        .outer_margin(4.0)
        .stroke((1.0, Color32::DARK_GRAY))
        .rounding(10.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(height)
                .auto_shrink(false)
                .show(ui, add_contents)
        });
}

fn build_motion_data_asset_button(
    ui: &mut egui::Ui,
    build_config: &BuildConfig,
    bvh_library: &BvhLibrary,
    bvh_assets: &Assets<BvhAsset>,
) {
    if ui.button("Build").clicked() {
        // TODO: Add this into BuildConfig?
        const TRAJECTORY_INTERVAL: f32 = 0.1667;

        let Some(bvh_map) = bvh_library
            .get_map()
            .and_then(|handle| bvh_assets.get(handle))
            .map(|asset| asset.get())
        else {
            return;
        };

        let mut motion_data_asset = MotionDataAsset::new(bvh_map, TRAJECTORY_INTERVAL);

        for id in build_config.bvh_assets.iter() {
            let Some(bvh) = bvh_assets.get(*id).map(|asset| asset.get()) else {
                return;
            };

            motion_data_asset.append_frames(bvh);
        }

        // TODO(perf): Serialize into binary instead
        let convert_to_json = serde_json::to_string(&motion_data_asset).unwrap();

        let mut motion_library = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            // TODO: specify a file name and possibly a location
            .open("assets/motion_data/motion_data.json")
            .unwrap();

        motion_library
            .write_all(convert_to_json.as_bytes())
            .unwrap();
    }
}

#[derive(Default, Clone, Copy)]
enum RightPanelPage {
    #[default]
    Config,
    Builder,
    PlayMode,
}

#[allow(clippy::too_many_arguments)]
fn right_panel(
    mut contexts: EguiContexts,
    mut selected_bvh_asset: ResMut<SelectedBvhAsset>,
    mut show_draw_arrow: ResMut<ShowDrawArrow>,
    mut build_configs: ResMut<BuildConfig>,
    asset_server: Res<AssetServer>,
    bvh_assets: Res<Assets<BvhAsset>>,
    bvh_library: Res<BvhLibrary>,
    mut playback_state: ResMut<PlaybackState>,
    mut page: Local<RightPanelPage>,
    mut mouse_in_ui: ResMut<MouseInUi>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("left_panel")
        .resizable(true)
        .show(ctx, |ui| {
            if ui.rect_contains_pointer(ui.min_rect()) {
                mouse_in_ui.set_is_inside();
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Config").clicked() {
                    *page = RightPanelPage::Config;
                }
                if ui.button("Builder").clicked() {
                    *page = RightPanelPage::Builder;
                }
                if ui.button("Play Mode").clicked() {
                    *page = RightPanelPage::PlayMode;
                }
            });

            egui::ScrollArea::vertical().show(ui, |ui| match *page {
                RightPanelPage::Config => {
                    ui.heading("Configurations");
                    ui.add_space(10.0);
                    bvh_map_label(ui, &bvh_library);
                    bvh_playback_menu(
                        ui,
                        &asset_server,
                        &bvh_assets,
                        &mut selected_bvh_asset,
                        &mut playback_state,
                    );
                    ui.add_space(10.0);
                    // bvh_map_config(ui, &bvh_library, &bvh_assets);
                }
                RightPanelPage::Builder => {
                    ui.heading("Builder");
                    ui.add_space(10.0);
                    motion_data_asset_buider_menu(
                        ui,
                        &asset_server,
                        &bvh_assets,
                        &mut build_configs,
                    );
                    ui.add_space(10.0);
                    build_motion_data_asset_button(ui, &build_configs, &bvh_library, &bvh_assets);
                }
                RightPanelPage::PlayMode => {
                    ui.heading("Play Mode");
                    ui.add_space(10.0);
                    arrow_checkbox(ui, &mut show_draw_arrow);
                }
            })
        });

    if ctx.is_pointer_over_area() {
        mouse_in_ui.set_is_inside();
    }
}
