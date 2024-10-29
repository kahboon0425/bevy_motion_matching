use bevy::prelude::*;

pub(super) struct Transform2dPlugin;

impl Plugin for Transform2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            apply_transform2d.before(TransformSystem::TransformPropagate),
        );

        app.register_type::<Transform2d>();
    }
}

fn apply_transform2d(
    mut q_transform2ds: Query<(&mut Transform, &Transform2d), Changed<Transform2d>>,
) {
    for (mut transform, transform2d) in q_transform2ds.iter_mut() {
        transform.translation.x = transform2d.translation.x;
        transform.translation.z = transform2d.translation.y;

        transform.rotation =
            Quat::from_rotation_y(f32::atan2(transform2d.direction.x, transform2d.direction.y));
    }
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Transform2d {
    pub translation: Vec2,
    direction: Vec2,
}

impl Default for Transform2d {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            direction: Vec2::Y,
        }
    }
}

impl Transform2d {
    pub fn set_direction(&mut self, direction: Vec2) {
        self.direction = direction.normalize_or_zero();
    }

    pub fn forward(&self) -> Vec2 {
        self.direction
    }

    pub fn right(&self) -> Vec2 {
        Vec2::new(self.direction.y, -self.direction.x)
    }
}
