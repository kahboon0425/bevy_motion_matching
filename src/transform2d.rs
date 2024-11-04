use bevy::prelude::*;

pub struct Transform2dPlugin;

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

        transform.rotation = Quat::from_rotation_y(transform2d.angle);
    }
}

#[derive(Component, Reflect, Default, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct Transform2d {
    pub translation: Vec2,
    pub angle: f32,
}

impl Transform2d {
    pub fn set_direction(&mut self, direction: Vec2) {
        self.angle = f32::atan2(direction.x, direction.y);
    }

    pub fn translation3d(&self) -> Vec3 {
        Vec3::new(self.translation.x, 0.0, self.translation.y)
    }

    pub fn direction3d(&self) -> Vec3 {
        let forward = self.forward();
        Vec3::new(forward.x, 0.0, forward.y)
    }

    pub fn forward(&self) -> Vec2 {
        Vec2::new(f32::sin(self.angle), f32::cos(self.angle))
    }

    pub fn right(&self) -> Vec2 {
        let forward = self.forward();
        Vec2::new(forward.y, -forward.x)
    }
}
