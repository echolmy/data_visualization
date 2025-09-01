mod animation;
mod camera;
mod environment;
mod lod;
mod mesh;
mod model_transform;
mod render;
mod ui;

use animation::TimeSeriesAnimationPlugin;
use bevy::pbr::wireframe::WireframePlugin;
use bevy::{pbr::MaterialPlugin, prelude::*};
use bevy_egui::*;
use bevy_obj::ObjPlugin;
use camera::CameraPlugin;
use environment::EnvironmentPlugin;
use lod::LODPlugin;
use model_transform::ModelTransformPlugin;
use render::{animate_wave_shader, create_wireframe_config, toggle_wireframe, WaveMaterial};
// use std::sync::atomic::{AtomicBool, Ordering};
use ui::UIPlugin;

#[derive(Component)]
pub struct Mesh3d(pub Handle<Mesh>);

#[derive(Component)]
pub struct MeshMaterial3d<M: Material>(pub Handle<M>);

// static DEBUG_PRINTED: AtomicBool = AtomicBool::new(false);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(ObjPlugin)
        .add_plugins(UIPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(EnvironmentPlugin)
        .add_plugins(ModelTransformPlugin)
        .add_plugins(LODPlugin)
        .add_plugins(TimeSeriesAnimationPlugin)
        .add_plugins(WireframePlugin)
        .insert_resource(create_wireframe_config())
        .add_systems(Update, toggle_wireframe)
        .add_plugins(MaterialPlugin::<WaveMaterial>::default())
        .add_systems(Update, animate_wave_shader)
        .run();
}
