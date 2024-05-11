use bevy::{prelude::*, utils::HashSet};
use bevy_egui::{
    egui::{self},
    EguiContexts, EguiPlugin,
};

use crate::{animation_player::SelectedBvhAsset, bvh_library::BvhLibrary};

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

pub fn animation_files_menu(
    ui: &mut egui::Ui,
    bvh_library: &BvhLibrary,
    selected_bvh_asset: &mut SelectedBvhAsset,
) {
    ui.horizontal(|ui| {
        ui.label("Choose Animation File:");
        egui::ComboBox::from_label("").show_ui(ui, |ui| {
            for filename in bvh_library.get_filenames() {
                if ui.selectable_label(false, filename).clicked() {
                    if let Some(handle) = bvh_library.get_handle(filename) {
                        selected_bvh_asset.0 = handle;
                    }
                }
            }
        });
    });
}

pub fn build_button(ui: &mut egui::Ui) {
    if ui.button("Build").clicked() {
        info!("build button pressed.");
    }
}

pub fn arrow_checkbox(ui: &mut egui::Ui, show_draw_arrow: &mut ShowDrawArrow) {
    ui.checkbox(&mut show_draw_arrow.show, "Show Arrows");
}

pub fn multiple_files_selection_menu(
    ui: &mut egui::Ui,
    bvh_library: &BvhLibrary,
    selected_files: &mut SelectedFiles,
) {
    ui.vertical(|ui| {
        ui.label("Select Multiple Animation Files:");
        for filename in bvh_library.get_filenames() {
            let mut is_selected = selected_files.files.contains(filename);
            if ui.checkbox(&mut is_selected, filename).changed() {
                if is_selected {
                    selected_files.files.insert(filename.clone());
                } else {
                    selected_files.files.remove(filename);
                }
            }
        }
    });
    println!("Selected Files: {:?}", selected_files);
}

fn ui(
    mut contexts: EguiContexts,
    bvh_library: Res<BvhLibrary>,
    mut selected_bvh_asset: ResMut<SelectedBvhAsset>,
    mut show_draw_arrow: ResMut<ShowDrawArrow>,
    mut selected_files: ResMut<SelectedFiles>,
) {
    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Properties");
            animation_files_menu(ui, &bvh_library, &mut selected_bvh_asset);
            build_button(ui);
            arrow_checkbox(ui, &mut show_draw_arrow);
            multiple_files_selection_menu(ui, &bvh_library, &mut selected_files);
        });
}
