use bevy::{prelude::*, utils::HashSet};
use bevy_egui::{
    egui::{self},
    EguiContexts, EguiPlugin,
};

use crate::animation_loader::{AnimationSelectEvent, BvhFile};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ui)
            .add_plugins(EguiPlugin)
            .insert_resource(ShowDrawArrow { show: true })
            .insert_resource(SelectedFiles::default());
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
    file_name: &ResMut<BvhFile>,
    mut event_writer: EventWriter<AnimationSelectEvent>,
) {
    ui.horizontal(|ui| {
        ui.label("Choose Animation File:");
        egui::ComboBox::from_label("").show_ui(ui, |ui| {
            for file in file_name.0.iter() {
                if ui.selectable_label(false, file).clicked() {
                    event_writer.send(AnimationSelectEvent(file.to_string()));
                }
            }
        });
    });
}

pub fn build_button(ui: &mut egui::Ui) {
    if ui.button("Build").clicked() {
        println!("Build Buttonnnnnnnnnnnnnnnnnnnn");
    }
}

pub fn arrow_checkbox(ui: &mut egui::Ui, mut show_draw_arrow: ResMut<ShowDrawArrow>) {
    ui.checkbox(&mut show_draw_arrow.show, "Show Arrows");
}

pub fn multiple_files_selection_menu(
    ui: &mut egui::Ui,
    file_name: &ResMut<BvhFile>,
    mut selected_files: ResMut<SelectedFiles>,
) {
    ui.vertical(|ui| {
        ui.label("Select Multiple Animation Files:");
        for file in file_name.0.iter() {
            let mut is_selected = selected_files.files.contains(file);
            if ui.checkbox(&mut is_selected, file).changed() {
                if is_selected {
                    selected_files.files.insert(file.clone());
                } else {
                    selected_files.files.remove(file);
                }
            }
        }
    });
    println!("Selected Files: {:?}", selected_files);
}

pub fn ui(
    mut contexts: EguiContexts,
    file_name: ResMut<BvhFile>,
    animation_file_selection_event: EventWriter<AnimationSelectEvent>,
    show_draw_arrow: ResMut<ShowDrawArrow>,
    selected_files: ResMut<SelectedFiles>,
) {
    let ctx = contexts.ctx_mut();
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Properties");
            animation_files_menu(ui, &file_name, animation_file_selection_event);
            build_button(ui);
            arrow_checkbox(ui, show_draw_arrow);
            multiple_files_selection_menu(ui, &file_name, selected_files);
        });
}
