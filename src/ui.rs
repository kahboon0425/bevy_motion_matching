use bevy::{ecs::system::SystemState, prelude::*};
use bevy_egui::{
    egui::{self, Color32},
    EguiContexts,
};

#[cfg(not(feature = "debug"))]
use bevy_egui::EguiPlugin;

#[cfg(feature = "debug")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use play_mode::RunPresetDirection;

use crate::player::ResetPlayer;

pub mod builder;
pub mod config;
pub mod play_mode;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        app.add_plugins(WorldInspectorPlugin::new());
        #[cfg(not(feature = "debug"))]
        app.add_plugins(EguiPlugin);

        app.init_resource::<MouseInUi>()
            .init_resource::<config::BvhTrailConfig>()
            .init_resource::<config::DrawMainArmature>()
            .init_resource::<play_mode::DrawNearestTrajectory>()
            .init_resource::<play_mode::DrawNearestPoseArmature>()
            .insert_resource(RunPresetDirection(false))
            .init_resource::<config::DrawTrajectory>()
            .init_resource::<builder::BuildConfigs>()
            .init_resource::<play_mode::MotionMatchingResult>()
            .add_systems(PreUpdate, reset_mouse_in_ui)
            .add_systems(Update, right_panel.in_set(UiSystemSet));
    }
}

#[derive(SystemSet, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct UiSystemSet;

#[derive(Resource, Default, Debug)]
pub struct MouseInUi(bool);

impl MouseInUi {
    pub fn get(&self) -> bool {
        self.0
    }

    pub fn set_is_inside(&mut self) {
        self.0 = true;
    }
}

fn reset_mouse_in_ui(mut mouse_in_ui: ResMut<MouseInUi>) {
    mouse_in_ui.0 = false;
}

#[derive(Default, Clone, Copy)]
enum RightPanelPage {
    #[default]
    Config,
    Builder,
    PlayMode,
}

fn right_panel(
    world: &mut World,
    params: &mut SystemState<(EguiContexts, Local<RightPanelPage>)>,
    reset_player: &mut SystemState<EventWriter<ResetPlayer>>,
) {
    let (mut contexts, mut page) = params.get_mut(world);

    let ctx = contexts.ctx_mut().clone();

    egui::SidePanel::left("left_panel")
        .resizable(true)
        .show(&ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Config").clicked() {
                    *page = RightPanelPage::Config;
                }
                if ui.button("Builder").clicked() {
                    *page = RightPanelPage::Builder;
                }
                if ui.button("Play Mode").clicked() {
                    *page = RightPanelPage::PlayMode;
                }
            });

            if ui.button("Reset Player").clicked() {
                let mut evw_reset_player = reset_player.get_mut(world);
                evw_reset_player.send(ResetPlayer);
            }

            egui::ScrollArea::vertical().show(ui, |ui| match *page {
                RightPanelPage::Config => config::config_panel(ui, world),
                RightPanelPage::Builder => builder::builder_panel(ui, world),
                RightPanelPage::PlayMode => {
                    // play_mode::play_mode_panel(ui, world)
                    play_mode::play_mode_panel(ui, world)
                }
            });
        });

    if ctx.is_pointer_over_area() {
        let mut mouse_in_ui = world.resource_mut::<MouseInUi>();
        mouse_in_ui.set_is_inside();
    }
}

fn scrollbox<R>(ui: &mut egui::Ui, height: f32, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    groupbox(ui, |ui| {
        egui::ScrollArea::vertical()
            .max_height(height)
            .auto_shrink(false)
            .show(ui, add_contents)
    });
}

fn groupbox<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    egui::Frame::default()
        .inner_margin(6.0)
        // .outer_margin(4.0)
        .stroke((1.0, Color32::DARK_GRAY))
        .corner_radius(5.0)
        .show(ui, add_contents);
}
