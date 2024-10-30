use bevy::prelude::*;

pub struct DrawAxesPlugin;

impl Plugin for DrawAxesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawAxes>()
            .add_systems(First, clear_axes)
            .add_systems(Last, draw_axes.run_if(resource_changed::<DrawAxes>));
    }
}

fn clear_axes(mut axes: ResMut<DrawAxes>) {
    if axes.is_empty() == false {
        axes.clear();
    }
}

fn draw_axes(mut gizmos: Gizmos, axes: Res<DrawAxes>, palette: Res<ColorPalette>) {
    for axis in axes.iter() {
        let start = axis.mat.transform_point3(Vec3::ZERO);
        gizmos.arrow(
            start,
            start + axis.mat.transform_vector3(Vec3::Z) * axis.size,
            axis.color.unwrap_or(palette.blue),
        );

        if axis.forward_only == false {
            gizmos.arrow(
                start,
                start + axis.mat.transform_vector3(Vec3::X) * axis.size,
                axis.color.unwrap_or(palette.red),
            );
            gizmos.arrow(
                start,
                start + axis.mat.transform_vector3(Vec3::Y) * axis.size,
                axis.color.unwrap_or(palette.green),
            );
        }
    }
}

/// Axes to draw.
///
/// Will be cleaned up every frame.
#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub struct DrawAxes(Vec<DrawAxis>);

impl DrawAxes {
    pub fn draw(&mut self, mat: Mat4, size: f32) {
        self.push(DrawAxis {
            mat,
            size,
            color: None,
            forward_only: false,
        });
    }

    pub fn draw_with_color(&mut self, mat: Mat4, size: f32, color: Color) {
        self.push(DrawAxis {
            mat,
            size,
            color: Some(color),
            forward_only: false,
        });
    }

    pub fn draw_forward(&mut self, mat: Mat4, size: f32, color: Color) {
        self.push(DrawAxis {
            mat,
            size,
            color: Some(color),
            forward_only: true,
        });
    }
}

/// Matrix and size of the axis to be drawn.
#[derive(Default, Debug, Clone, Copy)]
pub struct DrawAxis {
    pub mat: Mat4,
    pub size: f32,
    pub color: Option<Color>,
    pub forward_only: bool,
}

#[derive(Resource)]
pub struct ColorPalette {
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub blue: Color,
    pub purple: Color,
    pub base0: Color,
    pub base1: Color,
    pub base2: Color,
    pub base3: Color,
    pub base4: Color,
    pub base5: Color,
    pub base6: Color,
    pub base7: Color,
    pub base8: Color,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            red: Color::Srgba(Srgba::hex("#FF6188").unwrap()),
            orange: Color::Srgba(Srgba::hex("#FC9867").unwrap()),
            yellow: Color::Srgba(Srgba::hex("#FFD866").unwrap()),
            green: Color::Srgba(Srgba::hex("#A9DC76").unwrap()),
            blue: Color::Srgba(Srgba::hex("#78DCE8").unwrap()),
            purple: Color::Srgba(Srgba::hex("#AB9DF2").unwrap()),
            base0: Color::Srgba(Srgba::hex("#19181A").unwrap()),
            base1: Color::Srgba(Srgba::hex("#221F22").unwrap()),
            base2: Color::Srgba(Srgba::hex("#2D2A2E").unwrap()),
            base3: Color::Srgba(Srgba::hex("#403E41").unwrap()),
            base4: Color::Srgba(Srgba::hex("#5B595C").unwrap()),
            base5: Color::Srgba(Srgba::hex("#727072").unwrap()),
            base6: Color::Srgba(Srgba::hex("#939293").unwrap()),
            base7: Color::Srgba(Srgba::hex("#C1C0C0").unwrap()),
            base8: Color::Srgba(Srgba::hex("#FCFCFA").unwrap()),
        }
    }
}
