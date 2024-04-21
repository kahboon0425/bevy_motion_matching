use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

use crate::animation_loader::{AnimationSelectEvent, BvhFile};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ui).add_plugins(EguiPlugin);
    }
}

fn ui(
    mut egui_context: EguiContexts,
    file_name: ResMut<BvhFile>,
    mut event_writer: EventWriter<AnimationSelectEvent>,
) {
    egui::Window::new("Properties").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Choose Animation File");

            egui::ComboBox::from_label("").show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                for file in file_name.0.iter() {
                    println!("Number of file: {}", file_name.0.len());
                    // println!("File nameeee: {}", file);
                    if ui.selectable_label(false, file).clicked() {
                        println!("FILE CLICKED: {}", file);
                        event_writer.send(AnimationSelectEvent(file.to_string()));
                    };
                }
            });
        });
    });
}
