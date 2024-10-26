use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::bvh_manager::bvh_player::BvhPlayer;
use crate::motion_data::motion_data_player::MotionDataPlayerPair;
use crate::motion_data::MotionData;
use crate::motion_matching::MotionMatchingResult;
use crate::GameMode;

use super::groupbox;
use egui_extras::{Column, TableBuilder};

pub fn play_mode_panel(ui: &mut egui::Ui, world: &mut World) {
    ui.heading("Play Mode");
    ui.add_space(10.0);
    data_inspector(ui, world);
}

fn data_inspector(ui: &mut egui::Ui, world: &mut World) {
    let mut params = SystemState::<(
        MotionData,
        ResMut<MotionDataPlayerPair>,
        ResMut<BvhPlayer>,
        Res<State<GameMode>>,
        ResMut<NextState<GameMode>>,
        Res<MotionMatchingResult>,
    )>::new(world);
    let (
        motion_data,
        mut motion_player,
        mut bvh_player,
        game_mode,
        mut next_game_mode,
        motion_matching_result,
    ) = params.get_mut(world);

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

    ui.add_space(10.0);
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

    ui.add_space(10.0);

    ui.label("Motion Matching Result");

    ui.group(|ui| {
        TableBuilder::new(ui)
            .columns(Column::exact(110.0).resizable(true), 4)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading(egui::RichText::new("Chunk Index").size(12.0));

                    ui.separator();
                });
                header.col(|ui| {
                    ui.heading(egui::RichText::new("Chunk offset").size(12.0));
                    ui.separator();
                });
                header.col(|ui| {
                    ui.heading(egui::RichText::new("Trajectory Distance").size(12.0));
                    ui.separator();
                });
                header.col(|ui| {
                    ui.heading(egui::RichText::new("Pose Distance").size(12.0));
                    ui.separator();
                });
            })
            .body(|mut body| {
                for (nearest_trajectory, pose_matching_result) in motion_matching_result
                    .nearest_trajectories
                    .iter()
                    .zip(motion_matching_result.pose_matching_result.iter())
                {
                    if let Some(trajectory) = nearest_trajectory {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(format!("{}", trajectory.chunk_index));
                                ui.separator();
                            });
                            row.col(|ui| {
                                ui.label(format!("{}", trajectory.chunk_offset));
                                ui.separator();
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.3}", trajectory.distance));
                                ui.separator();
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.3}", pose_matching_result));
                                ui.separator();
                            });
                        });
                    }
                }
            });
    });

    ui.add_space(10.0);
}
