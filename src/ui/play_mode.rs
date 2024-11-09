use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::egui::Color32;
use egui_plot::{Arrows, Legend, Line, Plot, PlotPoints};

use crate::motion::MotionData;
use crate::motion_matching::MatchTrajectory;
use crate::trajectory::TrajectoryPlot;
use crate::{motion::chunk::ChunkIterator, trajectory::TrajectoryConfig};
use crate::{GameMode, BVH_SCALE_RATIO};

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
        ResMut<NextState<GameMode>>,
        Res<State<GameMode>>,
        Res<MotionMatchingResult>,
        Res<TrajectoryPlot>,
        Res<TrajectoryConfig>,
    )>::new(world);

    let (
        motion_data,
        mut next_game_mode,
        game_mode,
        motion_matching_result,
        traj_plot,
        traj_config,
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
            motion_asset.pose_data.interval_time()
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
    }

    ui.add_space(10.0);
    groupbox(ui, |ui| {
        ui.label("Trajectory Matching Visualization");

        let motion_trajs = &motion_asset.trajectory_data;

        let Some(selected_traj) = motion_matching_result
            .trajectories_poses
            .get(motion_matching_result.selected_trajectory)
            .map(|(traj, _)| traj)
        else {
            return;
        };

        let Some(trajs) = motion_trajs.get_chunk(selected_traj.chunk_index) else {
            return;
        };

        let chunk_offset = selected_traj.chunk_offset;
        let data_traj = &trajs[chunk_offset..chunk_offset + traj_config.num_points()];

        // Center point of trajectory
        let data_inv_matrix = data_traj[traj_config.history_count].matrix.inverse();

        let data_traj = data_traj
            .iter()
            .map(|traj| {
                data_inv_matrix.transform_point3(traj.matrix.to_scale_rotation_translation().2)
            })
            // Rescale?
            .map(|v| (v.xz() * BVH_SCALE_RATIO).as_dvec2().to_array())
            .collect::<Vec<_>>();

        // Asset data's trajectory.
        let data_traj_arrows = Arrows::new(
            PlotPoints::from_iter(data_traj[..data_traj.len() - 2].iter().cloned()),
            PlotPoints::from_iter(data_traj[1..].iter().cloned()),
        )
        .color(Color32::LIGHT_YELLOW)
        .name("Data Trajectory (Matched)");

        // Entity's trajectory.
        let traj_arrows = Arrows::new(
            PlotPoints::from_iter(traj_plot[..traj_plot.len() - 2].iter().cloned()),
            PlotPoints::from_iter(traj_plot[1..].iter().cloned()),
        )
        .color(Color32::LIGHT_BLUE)
        .name("Trajectory");

        // Plot the graph.
        Plot::new("trajectory_match_viz")
            .width(300.0)
            .height(300.0)
            .legend(Legend::default())
            .center_x_axis(true)
            .center_y_axis(true)
            .data_aspect(1.0)
            .show(ui, |plot_ui| {
                // x-axis
                plot_ui.line(
                    Line::new(PlotPoints::from_iter([[0.0, 0.0], [0.2, 0.0]])).color(Color32::RED),
                );
                // y-axis
                plot_ui.line(
                    Line::new(PlotPoints::from_iter([[0.0, 0.0], [0.0, 0.2]]))
                        .color(Color32::GREEN),
                );
                plot_ui.arrows(data_traj_arrows);
                plot_ui.arrows(traj_arrows);
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
                for (i, (trajectory, pose_dist)) in
                    motion_matching_result.trajectories_poses.iter().enumerate()
                {
                    let row_color = match i == motion_matching_result.selected_trajectory {
                        true => Some(Color32::GREEN),
                        false => Some(Color32::GRAY),
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
                            ui.label(format!("{}", pose_dist));
                            ui.separator();
                        });
                    });
                }
            });
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

#[derive(Default, Resource)]
pub struct MotionMatchingResult {
    /// Match trajectories and pose distances.
    pub trajectories_poses: Vec<(MatchTrajectory, f32)>,
    pub selected_trajectory: usize,
    pub traj_matching_time: String,
    pub pose_matching_time: String,
}
