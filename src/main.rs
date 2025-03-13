mod camera;
mod environment;
mod mesh;
mod ui;

use bevy::prelude::*;
use bevy_egui::*;
use bevy_obj::ObjPlugin;
use camera::CameraPlugin;
use environment::EnvironmentPlugin;
use std::sync::atomic::{AtomicBool, Ordering};
use ui::UIPlugin;

// 定义组件
#[derive(Component)]
pub struct Mesh3d(pub Handle<Mesh>);

#[derive(Component)]
pub struct MeshMaterial3d(pub Handle<StandardMaterial>);

// 用于跟踪是否已经打印过调试信息
static DEBUG_PRINTED: AtomicBool = AtomicBool::new(false);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(ObjPlugin)
        .add_plugins(UIPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(EnvironmentPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, debug_mesh_colors)
        .run();
}

/// set up a simple 3D scene
fn setup(mut commands: Commands) {
    // 这里可能需要添加一些模型加载或其他初始化代码
}

fn debug_mesh_colors(meshes: Res<Assets<Mesh>>, query: Query<&Mesh3d>) {
    // 如果已经打印过调试信息，则不再打印
    if DEBUG_PRINTED.load(Ordering::Relaxed) {
        return;
    }

    // 如果没有找到mesh，则不打印调试信息
    if query.iter().next().is_none() {
        return;
    }

    println!("Starting debug"); // 检查函数是否执行

    if let Some(handle) = query.iter().next() {
        println!("Found mesh handle"); // 检查是否找到了mesh handle

        if let Some(mesh) = meshes.get(&handle.0) {
            println!("Got mesh"); // 检查是否能获取到mesh

            if let Some(colors) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
                println!("Found colors"); // 检查是否有颜色属性
                println!("Colors: {:?}", colors);

                // 标记为已经打印过调试信息
                DEBUG_PRINTED.store(true, Ordering::Relaxed);
            } else {
                println!("No color attribute found"); // 如果没有颜色属性

                // 标记为已经打印过调试信息
                DEBUG_PRINTED.store(true, Ordering::Relaxed);
            }
        } else {
            println!("Could not get mesh from handle");
        }
    }
}
