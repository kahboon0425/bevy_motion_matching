use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::egui::Color32;
use egui_plot::{Line, Plot, PlotPoints};

use crate::bvh_manager::bvh_player::BvhPlayer;
use crate::motion_data::chunk::ChunkIterator;
use crate::motion_data::motion_data_player::MotionDataPlayerPair;
use crate::motion_data::MotionData;
use crate::motion_matching::MotionMatchingResult;
use crate::trajectory::TrajectoryPlot;
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
        Res<TrajectoryPlot>,
    )>::new(world);
    let (
        motion_data,
        mut motion_player,
        mut bvh_player,
        game_mode,
        mut next_game_mode,
        motion_matching_result,
        trajectory_points,
    ) = params.get_mut(world);

    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    // Overview
    ui.label("Overview");
    groupbox(ui, |ui| {
        ui.label(format!(
            "Chunk Count: {}",
            motion_asset.trajectory_data.offsets().num_chunks()
        ));
        ui.label(format!(
            "Trajectory Count: {}",
            motion_asset.trajectory_data.items().len()
        ));
        ui.label(format!(
            "Trajectory Interval: {}",
            motion_asset.trajectory_data.config().interval_time
        ));
        ui.label(format!(
            "Pose Count: {}",
            motion_asset.pose_data.items().len()
        ));
        ui.label(format!(
            "Pose Interval: {}",
            motion_asset.pose_data.interval()
        ));
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
    groupbox(ui, |ui| {
        ui.label("Trajectories Matching Visualization");

        let motion_trajs = &motion_asset.trajectory_data;

        let chunk_index = motion_matching_result.best_pose_result.chunk_index;
        let chunk_offset = motion_matching_result.best_pose_result.chunk_offset;

        let Some(trajs) = motion_trajs.get_chunk(chunk_index) else {
            return;
        };
        let trajectory = &trajs[chunk_offset..chunk_offset + 7];

        // Center point of trajectory
        let inv_matrix = trajectory[3].matrix.inverse();

        let best_trajectory_points = trajectory
            .iter()
            .map(|trajectory| {
                inv_matrix.transform_point3(trajectory.matrix.to_scale_rotation_translation().2)
            })
            // Rescale?
            .map(|v| (v.xz() * 0.01).as_dvec2().to_array())
            .collect::<Vec<_>>();

        let plot_points =
            PlotPoints::from_iter(trajectory_points.trajectories_points.iter().cloned());

        let trajectory_line = Line::new(plot_points)
            .color(Color32::from_rgb(255, 0, 0))
            .name("Trajectory");

        let best_pose_line = Line::new(PlotPoints::from_iter(best_trajectory_points))
            .color(Color32::from_rgb(0, 255, 0))
            .name("Best Pose");
        // let plot_points_2 = PlotPoints::from_iter(vec![
        //     [0.0, 0.0], // Start point
        //     [1.0, 1.0], // End point
        // ]);

        // let trajectory_line_2 = Line::new(plot_points_2)
        //     .color(Color32::from_rgb(255, 0, 255))
        //     .name("Trajectory");

        Plot::new("trajectory_plot")
            .width(500.0)
            .height(300.0)
            .show(ui, |plot_ui| {
                plot_ui.line(best_pose_line);
                plot_ui.line(trajectory_line);
                // plot_ui.line(trajectory_line_y);
            });
    });

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
                        let is_best_pose = trajectory.chunk_index
                            == motion_matching_result.best_pose_result.chunk_index
                            && trajectory.chunk_offset
                                == motion_matching_result.best_pose_result.chunk_offset
                            && trajectory
                                .distance
                                .eq(&motion_matching_result.best_pose_result.trajectory_distance)
                            && pose_matching_result
                                .eq(&motion_matching_result.best_pose_result.pose_distance);

                        let row_color = if is_best_pose {
                            Some(Color32::RED)
                        } else {
                            Some(Color32::GRAY)
                        };
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.visuals_mut().override_text_color = row_color;

                                ui.label(format!("{}", trajectory.chunk_index));
                                ui.separator();
                            });
                            row.col(|ui| {
                                ui.visuals_mut().override_text_color = row_color;
                                ui.label(format!("{}", trajectory.chunk_offset));
                                ui.separator();
                            });
                            row.col(|ui| {
                                ui.visuals_mut().override_text_color = row_color;
                                ui.label(format!("{}", trajectory.distance));
                                ui.separator();
                            });
                            row.col(|ui| {
                                ui.visuals_mut().override_text_color = row_color;
                                ui.label(format!("{}", pose_matching_result));
                                ui.separator();
                            });
                        });
                    }
                }
            });
    });

    ui.add_space(10.0);

    ui.label("Best Match Pose");
    groupbox(ui, |ui| {
        ui.label(format!(
            "Chunk Index: {}",
            motion_matching_result.best_pose_result.chunk_index
        ));
        ui.label(format!(
            "Chunk Offset: {}",
            motion_matching_result.best_pose_result.chunk_offset
        ));
        ui.label(format!(
            "Trajectory Distance: {}",
            motion_matching_result.best_pose_result.trajectory_distance
        ));
        ui.label(format!(
            "Pose Distance: {}",
            motion_matching_result.best_pose_result.pose_distance
        ));
    });

    ui.add_space(10.0);

    ui.label(format!(
        "Trajactory Matching Time: {} ms",
        motion_matching_result.traj_matching_time,
    ));

    ui.label(format!(
        "Pose Matching Time: {} ms",
        motion_matching_result.pose_matching_time,
    ));

    ui.label("Memory Usage");
    ui.add_space(10.0);
}
