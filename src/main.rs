use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(bevy_motion_matching::MotionMatchingAppPlugin)
        .run();
}
