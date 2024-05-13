use bevy::{prelude::*, utils::HashSet};
use bevy_egui::{
    egui::{self, Color32},
    EguiContexts, EguiPlugin,
};

use crate::{bvh_asset::BvhAsset, bvh_player::SelectedBvhAsset};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ui)
            .add_plugins(EguiPlugin)
            .insert_resource(ShowDrawArrow { show: true })
            .init_resource::<SelectedFiles>();
    }
}

#[derive(Resource)]
pub struct ShowDrawArrow {
    pub show: bool,
}

#[derive(Resource, Default, Debug)]
pub struct SelectedFiles {
    pub files: HashSet<String>,
}

pub fn bvh_selection_menu(
    ui: &mut egui::Ui,
    asset_server: &AssetServer,
    bvh_assets: &Assets<BvhAsset>,
    selected_bvh_asset: &mut SelectedBvhAsset,
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
                    }
                }
            });
    });
}

pub fn build_button(ui: &mut egui::Ui) {
    if ui.button("Build").clicked() {
        info!("Build button pressed.");
    }
}

pub fn arrow_checkbox(ui: &mut egui::Ui, show_draw_arrow: &mut ShowDrawArrow) {
    ui.checkbox(&mut show_draw_arrow.show, "Show Arrows");
}

pub fn bvh_buider_menu(
    ui: &mut egui::Ui,
    asset_server: &AssetServer,
    bvh_assets: &Assets<BvhAsset>,
    selected_files: &mut SelectedFiles,
) {
    ui.label("Bvh Builder");
    ui.add_space(10.0);
    egui::Frame::default()
        .inner_margin(6.0)
        .outer_margin(4.0)
        .stroke((1.0, Color32::DARK_GRAY))
        .rounding(10.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .auto_shrink(false)
                .show(ui, |ui| {
                    for id in bvh_assets.ids() {
                        let Some(bvh_name) = asset_server.get_path(id) else {
                            continue;
                        };
                        let bvh_name = bvh_name.to_string();

                        let mut is_selected = selected_files.files.contains(&bvh_name);
                        if ui.checkbox(&mut is_selected, &bvh_name).changed() {
                            if is_selected {
                                selected_files.files.insert(bvh_name);
                            } else {
                                selected_files.files.remove(&bvh_name);
                            }
                        }
                    }
                });
        });
}

fn ui(
    mut contexts: EguiContexts,
    mut selected_bvh_asset: ResMut<SelectedBvhAsset>,
    mut show_draw_arrow: ResMut<ShowDrawArrow>,
    mut selected_files: ResMut<SelectedFiles>,
    asset_server: Res<AssetServer>,
    bvh_assets: Res<Assets<BvhAsset>>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("right_panel")
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Properties");
            ui.add_space(10.0);
            arrow_checkbox(ui, &mut show_draw_arrow);
            ui.add_space(10.0);
            bvh_selection_menu(ui, &asset_server, &bvh_assets, &mut selected_bvh_asset);
            ui.add_space(10.0);
            bvh_buider_menu(ui, &asset_server, &bvh_assets, &mut selected_files);
            ui.add_space(10.0);
            build_button(ui);
        });
}
