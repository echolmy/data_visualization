mod camera;
mod environment;
mod mesh;
mod model_transform;
mod render;
mod ui;

use bevy::pbr::wireframe::WireframePlugin;
use bevy::{pbr::MaterialPlugin, prelude::*};
use bevy_egui::*;
use bevy_obj::ObjPlugin;
use camera::CameraPlugin;
use environment::EnvironmentPlugin;
use model_transform::ModelTransformPlugin;
use render::{animate_wave_shader, create_wireframe_config, toggle_wireframe, WaveMaterial};
use std::sync::atomic::{AtomicBool, Ordering};
use ui::UIPlugin;

// 定义组件
#[derive(Component)]
pub struct Mesh3d(pub Handle<Mesh>);

#[derive(Component)]
pub struct MeshMaterial3d<M: Material>(pub Handle<M>);

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
        .add_plugins(ModelTransformPlugin) // 添加模型变换功能
        // 添加线框渲染功能
        .add_plugins(WireframePlugin)
        .insert_resource(create_wireframe_config())
        .add_systems(Update, toggle_wireframe)
        // 添加波浪材质插件和动画系统
        .add_plugins(MaterialPlugin::<WaveMaterial>::default())
        .add_systems(Update, animate_wave_shader)
        .add_systems(Update, debug_mesh_colors)
        .run();
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
