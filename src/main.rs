mod camera;
mod mesh;
mod ui;

use bevy::prelude::*;
use bevy_egui::*;
use bevy_obj::ObjPlugin;
use camera::CameraPlugin;
use ui::UIPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(ObjPlugin)
        .add_plugins(UIPlugin)
        .add_plugins(CameraPlugin)
        .add_systems(Startup, setup)
        // .add_systems(Update, update_transform)
        .run();
}

/// set up a simple 3D scene
fn setup(mut commands: Commands) {
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(Vec3::new(5.0, 5.0, 0.0)),
    ));
}
