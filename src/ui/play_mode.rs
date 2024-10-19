use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::motion_data::MotionData;

use super::groupbox;

pub fn play_mode_panel(ui: &mut egui::Ui, world: &mut World) {
    ui.heading("Play Mode");
    ui.add_space(10.0);
    data_inspector(ui, world);
}

fn data_inspector(ui: &mut egui::Ui, world: &mut World) {
    let mut params = SystemState::<MotionData>::new(world);
    let mut motion_data = params.get_mut(world);

    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    // Overview
    ui.label("Overview");
    groupbox(ui, |ui| {
        ui.label(format!(
            "Chunk Count: {}",
            motion_asset.trajectories.chunk_count()
        ));
        ui.label(format!(
            "Trajectory Count: {}",
            motion_asset.trajectories.matrices().len()
        ));
        ui.label(format!(
            "Trajectory Interval: {}",
            motion_asset.trajectories.interval()
        ));
        ui.label(format!("Pose Count: {}", motion_asset.poses.poses().len()));
        ui.label(format!("Pose Interval: {}", motion_asset.poses.interval()));
    });
}
