use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::egui::Color32;
use egui_plot::{Arrows, Legend, Line, Plot, PlotPoints};

use crate::motion::chunk::ChunkIterator;
use crate::motion::MotionData;
use crate::motion_matching::MatchTrajectory;
use crate::testing::generate_testing_data;
use crate::trajectory::TrajectoryConfig;
use crate::trajectory::TrajectoryPlot;
use crate::{GameMode, Method, BVH_SCALE_RATIO};

use super::groupbox;
use egui_extras::{Column, TableBuilder};

pub fn play_mode_panel(ui: &mut egui::Ui, world: &mut World) {
    ui.heading("Play Mode");
    ui.add_space(10.0);
    data_inspector(ui, world);
    generate_testing_data(ui, world);
    draw_nearest_pose_armature_checkbox(ui, world);
    draw_nearest_trajectory_checkbox(ui, world);
    run_preset_direction(ui, world);
    motion_matching_method(ui, world);
    trajectory_matching_visualization(ui, world);
    motion_matching_result(ui, world);
}

fn data_inspector(ui: &mut egui::Ui, world: &mut World) {
    let mut params = SystemState::<(
        MotionData,
        ResMut<NextState<GameMode>>,
        Res<State<GameMode>>,
    )>::new(world);

    let (motion_data, mut next_game_mode, game_mode) = params.get_mut(world);

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
}

fn draw_nearest_pose_armature_checkbox(ui: &mut egui::Ui, world: &mut World) {
    let mut draw_main_armature = world.resource_mut::<DrawNearestPoseArmature>();
    ui.checkbox(&mut draw_main_armature, "Show Nearest Pose Armature");
}

fn draw_nearest_trajectory_checkbox(ui: &mut egui::Ui, world: &mut World) {
    let mut draw_trajectory = world.resource_mut::<DrawNearestTrajectory>();
    ui.checkbox(&mut draw_trajectory, "Show Nearest Trajectory Arrows");
}

fn run_preset_direction(ui: &mut egui::Ui, world: &mut World) {
    let mut run_preset_movement = world.resource_mut::<RunPresetDirection>();
    ui.checkbox(&mut run_preset_movement, "Run Preset Movement");
    ui.add_space(10.0);
}

fn motion_matching_method(ui: &mut egui::Ui, world: &mut World) {
    let mut params = SystemState::<(
        ResMut<MotionMatchingResult>,
        Res<State<Method>>,
        ResMut<NextState<Method>>,
    )>::new(world);

    let (mut motion_matching_result, method_state, mut next_method_state) = params.get_mut(world);

    ui.horizontal(|ui| {
        ui.label("Method:");

        let methods = ["BruteForceKNN", "KdTree", "KMeans"];

        // cast enum into a usize
        let mut selected_index = *method_state.get() as usize;

        egui::ComboBox::from_label("")
            .selected_text(methods[selected_index].to_string())
            .show_index(ui, &mut selected_index, methods.len(), |i| methods[i]);

        let new_method = match selected_index {
            0 => Method::BruteForceKNN,
            1 => Method::KdTree,
            2 => Method::KMeans,
            _ => Method::BruteForceKNN,
        };

        if *method_state.get() != new_method {
            motion_matching_result.matching_result = MatchingResult::default();
        }

        next_method_state.set(new_method);
    });
    ui.add_space(10.0);
}

fn trajectory_matching_visualization(ui: &mut egui::Ui, world: &mut World) {
    let mut params = SystemState::<(
        MotionData,
        Res<MotionMatchingResult>,
        Res<TrajectoryPlot>,
        Res<TrajectoryConfig>,
    )>::new(world);

    let (motion_data, motion_matching_result, traj_plot, traj_config) = params.get_mut(world);

    let Some(motion_asset) = motion_data.get() else {
        return;
    };

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
                data_inv_matrix
                    .transform_point3(traj.matrix.to_scale_rotation_translation().2)
                    .xz()
            })
            .map(|mut v| {
                // x axis is reversed in bevy.
                v.x = -v.x;
                v *= BVH_SCALE_RATIO;
                v.as_dvec2().to_array()
            })
            .collect::<Vec<_>>();

        // Asset data's trajectory.
        let data_traj_arrows = Arrows::new(
            PlotPoints::from_iter(data_traj[..data_traj.len() - 2].iter().cloned()),
            PlotPoints::from_iter(data_traj[1..].iter().cloned()),
        )
        .color(Color32::LIGHT_YELLOW)
        .name("Data Trajectory (Matched)");

        // Entity's trajectory.
        if traj_plot.len() >= 2 {
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
                        Line::new(PlotPoints::from_iter([[0.0, 0.0], [0.2, 0.0]]))
                            .color(Color32::RED),
                    );
                    // y-axis
                    plot_ui.line(
                        Line::new(PlotPoints::from_iter([[0.0, 0.0], [0.0, 0.2]]))
                            .color(Color32::GREEN),
                    );
                    plot_ui.arrows(data_traj_arrows);
                    plot_ui.arrows(traj_arrows);
                });
        }
    });
    ui.add_space(10.0);
}

fn motion_matching_result(ui: &mut egui::Ui, world: &mut World) {
    let mut params = SystemState::<(Res<MotionMatchingResult>, MotionData)>::new(world);

    let (motion_matching_result, motion_data) = params.get_mut(world);

    let Some(motion_asset) = motion_data.get() else {
        return;
    };

    ui.label("Motion Matching Result");

    ui.group(|ui| {
        TableBuilder::new(ui)
            .column(Column::initial(150.0))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading(egui::RichText::new("File Name").size(12.0).strong());
                    ui.separator();
                });
                header.col(|ui| {
                    ui.heading(egui::RichText::new("Traj Idx").size(12.0).strong());
                    ui.separator();
                });
                header.col(|ui| {
                    ui.heading(egui::RichText::new("Traj Dist").size(12.0).strong());
                    ui.separator();
                });
                header.col(|ui| {
                    ui.heading(egui::RichText::new("Pose Dist").size(12.0).strong());
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
                            let x = &motion_asset.animation_file[trajectory.chunk_index];
                            ui.visuals_mut().override_text_color = row_color;
                            ui.label(x.to_string());
                            ui.separator();
                        });
                        row.col(|ui| {
                            ui.visuals_mut().override_text_color = row_color;
                            ui.label(format!("{}", trajectory.chunk_offset));
                            ui.separator();
                        });
                        row.col(|ui| {
                            ui.visuals_mut().override_text_color = row_color;
                            ui.label(format!("{:.3}", trajectory.distance));
                            ui.separator();
                        });
                        row.col(|ui| {
                            ui.visuals_mut().override_text_color = row_color;
                            ui.label(format!("{:.3}", pose_dist));
                            ui.separator();
                        });
                    });
                }
            });
    });
    ui.add_space(10.0);

    let result = motion_matching_result.matching_result;
    ui.label(format!(
        "Average Trajactory Matching Time: {:.3} ms",
        result.avg_time,
    ));
    ui.label(format!("Average Memory Usage: {:.3} MB", result.avg_memory,));
}

#[derive(Resource, Deref, DerefMut)]
pub struct DrawNearestPoseArmature(bool);

impl Default for DrawNearestPoseArmature {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct DrawNearestTrajectory(bool);

impl Default for DrawNearestTrajectory {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct RunPresetDirection(pub bool);

#[derive(Default, Resource)]
pub struct MotionMatchingResult {
    /// Match trajectories and pose distances.
    pub trajectories_poses: Vec<(MatchTrajectory, f32)>,
    pub selected_trajectory: usize,
    pub matching_result: MatchingResult,
    // pub pose_matching_time: String,
}

#[derive(Default, Component, Copy, Clone)]
pub struct MatchingResult {
    pub avg_time: f64,
    pub avg_memory: f64,
    pub runs: usize,
}
