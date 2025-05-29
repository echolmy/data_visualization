pub mod events;
use crate::mesh;
use bevy::prelude::*;
use bevy_egui::*;
use rfd::FileDialog;
use std::path::PathBuf;

// 添加一个新的事件类型，用于在模型加载完成后发送
#[derive(Event)]
pub struct ModelLoadedEvent {
    pub position: Vec3,
    pub scale: Vec3,
    pub bounds_min: Option<Vec3>, // 模型包围盒的最小点
    pub bounds_max: Option<Vec3>, // 模型包围盒的最大点
}

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<events::OpenFileEvent>()
            .add_event::<events::LoadModelEvent>()
            .add_event::<events::LoadModelWithConversionEvent>() // 注册新的转换事件
            .add_event::<events::ToggleWireframeEvent>() // 注册线框切换事件
            .add_event::<ModelLoadedEvent>() // 注册新事件
            .add_systems(
                Update,
                (
                    initialize_ui_systems,
                    file_dialog_system,
                    load_resource,
                    load_resource_with_conversion,
                )
                    .after(EguiSet::InitContexts),
            );
        // .add_plugins(ObjPlugin);
    }
}

fn initialize_ui_systems(
    mut contexts: EguiContexts,
    mut open_file_events: EventWriter<events::OpenFileEvent>,
    mut wireframe_toggle_events: EventWriter<events::ToggleWireframeEvent>,
    mut load_conversion_events: EventWriter<events::LoadModelWithConversionEvent>,
    windows: Query<&Window>,
) {
    // 只有在窗口存在时才访问egui上下文
    if windows.iter().next().is_some() {
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

                // 添加View菜单
                egui::menu::menu_button(ui, "View", |ui| {
                    if ui.button("Wireframe").clicked() {
                        wireframe_toggle_events.send(events::ToggleWireframeEvent);
                    }
                });
            });
        });
    }
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

/// 加载资源文件
fn load_resource(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut load_events: EventReader<events::LoadModelEvent>,
    mut model_loaded_events: EventWriter<ModelLoadedEvent>, // 添加事件写入器
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    // 检查窗口是否存在
    let window_exists = windows.iter().next().is_some();

    for events::LoadModelEvent(path) in load_events.read() {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("obj") => {
                let position = Vec3::new(0.0, 0.5, 0.0);
                let scale = Vec3::splat(1.0);

                commands.spawn((
                    Mesh3d(asset_server.load(format!("{}", path.to_string_lossy()))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        unlit: false,
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    })),
                    Transform::from_translation(position).with_scale(scale),
                ));

                // 对于OBJ模型，我们无法直接获取包围盒，所以设置为None
                model_loaded_events.send(ModelLoadedEvent {
                    position,
                    scale,
                    bounds_min: None,
                    bounds_max: None,
                });
            }
            // VTK extension:
            // Legacy: .vtk
            Some("vtk") => {
                match mesh::process_vtk_file_legacy(path) {
                    Ok(mesh) => {
                        let position = Vec3::new(0.0, 0.5, 0.0);
                        let scale = Vec3::ONE;

                        // 计算模型的包围盒
                        let mut bounds_min = None;
                        let mut bounds_max = None;

                        if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                            if let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) =
                                positions
                            {
                                // 初始化包围盒
                                if !positions.is_empty() {
                                    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
                                    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

                                    // 遍历所有顶点，更新包围盒
                                    for pos in positions {
                                        let pos_vec = Vec3::new(pos[0], pos[1], pos[2]);
                                        min = min.min(pos_vec);
                                        max = max.max(pos_vec);
                                    }

                                    bounds_min = Some(min);
                                    bounds_max = Some(max);

                                    println!("Model bounds: min={:?}, max={:?}", min, max);
                                }
                            }
                        }

                        // 创建实体，移除直接添加的Wireframe组件
                        commands.spawn((
                            Mesh3d(meshes.add(mesh.clone())),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(1.0, 1.0, 1.0),
                                metallic: 0.0,
                                perceptual_roughness: 0.5,
                                reflectance: 0.0,
                                cull_mode: None,
                                unlit: true,
                                alpha_mode: AlphaMode::Opaque,
                                ..default()
                            })),
                            Transform::from_translation(position),
                            Visibility::Visible,
                        ));

                        println!("number of vertices: {:?}", mesh.count_vertices());

                        // 发送模型加载完成事件，包含包围盒信息
                        model_loaded_events.send(ModelLoadedEvent {
                            position,
                            scale,
                            bounds_min,
                            bounds_max,
                        });
                    }
                    Err(err) => {
                        println!("load VTK file failed: {:?}", err);
                        // 显示错误消息到UI
                        if window_exists {
                            egui::Window::new("Error").show(egui_context.ctx_mut(), |ui| {
                                ui.label(format!("load file failed: {:?}", err));
                            });
                        }
                    }
                }
            }
            // XML: .vtu (非结构网格), .vtp (多边形数据), .vts (结构网格),
            //      .vtr (矩形网格), .vti (图像数据)
            Some("vtu" | "vtp" | "vts" | "vtr" | "vti") => {
                // 显示暂不支持的消息
                if window_exists {
                    egui::Window::new("Note").show(egui_context.ctx_mut(), |ui| {
                        ui.label("currently not supported this format, developing...");
                    });
                }
            }
            _ => {
                println!("currently not supported other formats, please select another model.");
                // 显示不支持的消息
                if window_exists {
                    egui::Window::new("Not supported format").show(egui_context.ctx_mut(), |ui| {
                        ui.label(
                            "not supported this file format, please select .obj or .vtk file.",
                        );
                    });
                }
            }
        }
    }
}

/// 加载资源文件，支持一阶到二阶转换
fn load_resource_with_conversion(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut load_events: EventReader<events::LoadModelWithConversionEvent>,
    mut model_loaded_events: EventWriter<ModelLoadedEvent>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    // 检查窗口是否存在
    let window_exists = windows.iter().next().is_some();

    for events::LoadModelWithConversionEvent {
        path,
        convert_to_second_order,
    } in load_events.read()
    {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("vtk") => {
                match mesh::process_vtk_file_legacy_with_conversion(path, *convert_to_second_order)
                {
                    Ok(mesh) => {
                        let position = Vec3::new(0.0, 0.5, 0.0);
                        let scale = Vec3::ONE;

                        // 计算模型的包围盒
                        let mut bounds_min = None;
                        let mut bounds_max = None;

                        if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                            if let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) =
                                positions
                            {
                                // 初始化包围盒
                                if !positions.is_empty() {
                                    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
                                    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

                                    // 遍历所有顶点，更新包围盒
                                    for pos in positions {
                                        let pos_vec = Vec3::new(pos[0], pos[1], pos[2]);
                                        min = min.min(pos_vec);
                                        max = max.max(pos_vec);
                                    }

                                    bounds_min = Some(min);
                                    bounds_max = Some(max);

                                    println!("Model bounds: min={:?}, max={:?}", min, max);
                                }
                            }
                        }

                        // 创建实体
                        commands.spawn((
                            Mesh3d(meshes.add(mesh.clone())),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(1.0, 1.0, 1.0),
                                metallic: 0.0,
                                perceptual_roughness: 0.5,
                                reflectance: 0.0,
                                cull_mode: None,
                                unlit: true,
                                alpha_mode: AlphaMode::Opaque,
                                ..default()
                            })),
                            Transform::from_translation(position),
                            Visibility::Visible,
                        ));

                        println!("number of vertices: {:?}", mesh.count_vertices());
                        if *convert_to_second_order {
                            println!("模型已转换为二阶");
                        }

                        // 发送模型加载完成事件，包含包围盒信息
                        model_loaded_events.send(ModelLoadedEvent {
                            position,
                            scale,
                            bounds_min,
                            bounds_max,
                        });
                    }
                    Err(err) => {
                        println!("load VTK file failed: {:?}", err);
                        // 显示错误消息到UI
                        if window_exists {
                            egui::Window::new("Error").show(egui_context.ctx_mut(), |ui| {
                                ui.label(format!("load file failed: {:?}", err));
                            });
                        }
                    }
                }
            }
            _ => {
                println!("转换功能目前只支持VTK文件格式");
                if window_exists {
                    egui::Window::new("转换不支持").show(egui_context.ctx_mut(), |ui| {
                        ui.label("一阶到二阶转换功能目前只支持VTK文件格式");
                    });
                }
            }
        }
    }
}
