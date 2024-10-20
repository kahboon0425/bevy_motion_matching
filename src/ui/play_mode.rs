use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::bvh_manager::bvh_player::BvhPlayer;
use crate::motion_data::motion_data_player::MotionDataPlayer;
use crate::motion_data::MotionData;
use crate::GameMode;

use super::groupbox;

pub fn play_mode_panel(ui: &mut egui::Ui, world: &mut World) {
    ui.heading("Play Mode");
    ui.add_space(10.0);
    data_inspector(ui, world);
}

fn data_inspector(ui: &mut egui::Ui, world: &mut World) {
    let mut params = SystemState::<(
        MotionData,
        ResMut<MotionDataPlayer>,
        ResMut<BvhPlayer>,
        Res<State<GameMode>>,
        ResMut<NextState<GameMode>>,
    )>::new(world);
    let (motion_data, mut motion_player, mut bvh_player, game_mode, mut next_game_mode) =
        params.get_mut(world);

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

    let button_text = match **game_mode {
        GameMode::Play => "Disable Play Mode",
        _ => "Enable Play Mode",
    };

    if ui.button(button_text).clicked() {
        match **game_mode {
            GameMode::Play => next_game_mode.set(GameMode::None),
            _ => next_game_mode.set(GameMode::Play),
        }

        motion_player.is_playing = !motion_player.is_playing;
        // Stop the bvh preview player.
        if motion_player.is_playing {
            bvh_player.is_playing = false;
        }
    }
}
