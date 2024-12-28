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

// fn query_meshes(
//     meshes: Query<(Entity, &Handle<Mesh>)>,
//     mesh_assets: Res<Assets<Mesh>>,
// ) {
//     for (entity, mesh_handle) in meshes.iter() {
//         if let Some(mesh) = mesh_assets.get(mesh_handle) {
//             println!("找到实体 {:?} 的mesh", entity);
//
//             // 获取顶点位置
//             if let Some(vertices) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
//                 println!("\n顶点位置数据:");
//                 println!("顶点数量: {}", vertices.len());
//                 match vertices {
//                     bevy::render::mesh::VertexAttributeValues::Float32x3(positions) => {
//                         for (i, position) in positions.iter().enumerate() {
//                             println!("顶点 {}: ({}, {}, {})", i, position[0], position[1], position[2]);
//                         }
//                     }
//                     _ => println!("顶点格式不是 Float32x3"),
//                 }
//             }
//
//             // 获取索引数据
//             if let Some(indices) = mesh.indices() {
//                 println!("\n索引数据:");
//                 println!("索引数量: {}", indices.len());
//                 match indices {
//                     bevy::render::mesh::Indices::U32(indices) => {
//                         // 每三个索引构成一个三角形
//                         for i in (0..indices.len()).step_by(3) {
//                             println!("三角形 {}: 顶点索引 [{}, {}, {}]",
//                                      i/3,
//                                      indices[i],
//                                      indices[i+1],
//                                      indices[i+2]
//                             );
//                         }
//                     }
//                     _ => println!("索引格式不是 U32"),
//                 }
//             }
//
//             // 获取法线数据
//             if let Some(normals) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
//                 println!("\n法线数据:");
//                 match normals {
//                     bevy::render::mesh::VertexAttributeValues::Float32x3(normals) => {
//                         for (i, normal) in normals.iter().enumerate() {
//                             println!("顶点 {} 的法线: ({}, {}, {})", i, normal[0], normal[1], normal[2]);
//                         }
//                     }
//                     _ => println!("法线格式不是 Float32x3"),
//                 }
//             } else {
//                 println!("\n该mesh没有法线数据");
//             }
//         }
//     }
// }

// fn update_transform(mut query: Query<&mut Transform, With<ui::Model>>) {
//     for mut transform in &mut query {
//         transform.rotate_y(0.01);
//         // transform.rotate_z(0.01);
//         // transform.rotate_x(0.01);
//     }
// }
