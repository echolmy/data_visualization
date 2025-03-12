mod events;

use crate::mesh::{self, vtk};
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::*;
use rfd::FileDialog;
use std::path::PathBuf;
pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<events::OpenFileEvent>()
            .add_event::<events::LoadModelEvent>()
            .add_systems(
                Update,
                (initialize_ui_systems, file_dialog_system, load_resource),
            );
        // .add_plugins(ObjPlugin);
    }
}

fn initialize_ui_systems(
    mut contexts: EguiContexts,
    mut open_file_events: EventWriter<events::OpenFileEvent>,
) {
    egui::TopBottomPanel::top("Menu Bar").show(contexts.ctx_mut(), |ui| {
        // The top panel is often a good place for a menu bar:
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Import").clicked() {
                    // send an event
                    open_file_events.send(events::OpenFileEvent);
                }
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
        });
    });
}

fn file_dialog_system(
    mut open_events: EventReader<events::OpenFileEvent>,
    mut load_events: EventWriter<events::LoadModelEvent>,
) {
    for _ in open_events.read() {
        if let Some(file) = FileDialog::new()
            .add_filter("model", &["obj", "glb", "vtk"])
            .set_directory("/")
            .pick_file()
        {
            let filepath = PathBuf::from(file.display().to_string());
            println!("open file: {}", filepath.display());
            load_events.send(events::LoadModelEvent(filepath));
        };
    }
}

fn load_resource(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut load_events: EventReader<events::LoadModelEvent>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut egui_context: EguiContexts,
) {
    let window = window_query.get_single().unwrap();
    for events::LoadModelEvent(path) in load_events.read() {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("obj") => {
                commands.spawn((
                    Mesh3d(asset_server.load(format!("{}", path.to_string_lossy()))),
                    Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, -5.0),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    })),
                ));
            }
            // Some("gltf") | Some("glb") => {
            //     commands.spawn((
            //         SceneRoot(asset_server.load(format!("{}#Scene0", path.to_string_lossy()))),
            //         Transform::from_xyz(0.0, 0.0, 0.0),
            //         Visibility::Visible,
            //     ));
            // }
            // VTK extension:
            // Legacy: .vtk
            Some("vtk") => {
                match mesh::process_vtk_file_legacy(path) {
                    Ok(mesh) => {
                        commands.spawn((
                            Mesh3d(meshes.add(mesh.clone())),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(0.7, 0.7, 0.7),
                                metallic: 0.0,
                                perceptual_roughness: 0.5,
                                reflectance: 0.1,
                                cull_mode: None,
                                ..default()
                            })),
                            Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, -1.0)
                                .with_rotation(Quat::from_euler(
                                    EulerRot::XYZ,
                                    std::f32::consts::PI / 2.0,
                                    std::f32::consts::PI / 4.0,
                                    0.0,
                                )),
                            Visibility::Visible,
                        ));

                        println!("生成的网格顶点数: {:?}", mesh.count_vertices());
                    }
                    Err(err) => {
                        println!("加载VTK文件失败: {:?}", err);
                        // 显示错误消息到UI
                        egui::Window::new("错误").show(egui_context.ctx_mut(), |ui| {
                            ui.label(format!("加载文件失败: {:?}", err));
                        });
                    }
                }
            }
            // XML: .vtu (非结构网格), .vtp (多边形数据), .vts (结构网格),
            //      .vtr (矩形网格), .vti (图像数据)
            Some("vtu" | "vtp" | "vts" | "vtr" | "vti") => {
                // 显示暂不支持的消息
                egui::Window::new("提示").show(egui_context.ctx_mut(), |ui| {
                    ui.label("目前暂不支持该格式，正在开发中...");
                });
            }
            _ => {
                println!("目前不支持其他格式，请选择另一个模型。");
                // 显示不支持的消息
                egui::Window::new("不支持的格式").show(egui_context.ctx_mut(), |ui| {
                    ui.label("不支持此文件格式，请选择.obj或.vtk文件。");
                });
            }
        };
    }
}
