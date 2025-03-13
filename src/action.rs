use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<PlayerAction>::default())
            .init_resource::<ActionState<PlayerAction>>()
            .insert_resource(PlayerAction::input_map());
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum PlayerAction {
    #[actionlike(DualAxis)]
    Walk,
    Run,
}

impl PlayerAction {
    /// Define the default bindings to the input
    pub fn input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        // Default gamepad input bindings
        input_map.insert_dual_axis(Self::Walk, GamepadStick::LEFT);
        input_map.insert(Self::Run, GamepadButton::South);

        // Default kbm input bindings
        input_map.insert_dual_axis(Self::Walk, VirtualDPad::wasd());
        input_map.insert(Self::Run, KeyCode::ShiftLeft);

        input_map
    }
}
