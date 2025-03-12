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
        // .add_systems(Update, debug_mesh_colors)
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

// fn debug_mesh_colors(meshes: Res<Assets<Mesh>>, query: Query<&Mesh3d>) {
//     println!("Starting debug"); // 检查函数是否执行

//     if let Some(handle) = query.iter().next() {
//         println!("Found mesh handle"); // 检查是否找到了mesh handle

//         if let Some(mesh) = meshes.get(&handle.0) {
//             println!("Got mesh"); // 检查是否能获取到mesh

//             if let Some(colors) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
//                 println!("Found colors"); // 检查是否有颜色属性
//                 println!("Colors: {:?}", colors);
//             } else {
//                 println!("No color attribute found"); // 如果没有颜色属性
//             }
//         } else {
//             println!("Could not get mesh from handle");
//         }
//     } else {
//         println!("No mesh found in query"); // 如果查询不到mesh
//     }
// }
