pub mod events;
use crate::mesh;
use crate::mesh::vtk::VtkMeshExtractor; // 添加VtkMeshExtractor trait导入
use bevy::prelude::*;
use bevy_egui::*;
use rfd::FileDialog;
use std::path::PathBuf;
use vtkio; // 添加vtkio导入

// 添加一个新的事件类型，用于在模型加载完成后发送
#[derive(Event)]
pub struct ModelLoadedEvent {
    pub position: Vec3,
    pub scale: Vec3,
    pub bounds_min: Option<Vec3>, // 模型包围盒的最小点
    pub bounds_max: Option<Vec3>, // 模型包围盒的最大点
}

// 存储当前模型的几何数据
#[derive(Resource, Default)]
pub struct CurrentModelData {
    pub geometry: Option<mesh::vtk::GeometryData>,
    pub is_higher_order: bool,
    pub current_order: u32,
}

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<events::OpenFileEvent>()
            .add_event::<events::LoadModelEvent>()
            .add_event::<events::ToggleWireframeEvent>() // 注册线框切换事件
            .add_event::<events::ConvertToHigherOrderEvent>() // 注册高阶转换事件
            .add_event::<ModelLoadedEvent>() // 注册新事件
            .init_resource::<CurrentModelData>() // 注册当前模型数据资源
            .add_systems(
                Update,
                (
                    initialize_ui_systems,
                    file_dialog_system,
                    load_resource,
                    handle_higher_order_conversion, // 添加处理转换的系统
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
    mut convert_events: EventWriter<events::ConvertToHigherOrderEvent>, // 添加转换事件写入器
    current_model: Res<CurrentModelData>,                               // 添加当前模型数据访问
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

                // 添加Mesh菜单
                egui::menu::menu_button(ui, "Mesh", |ui| {
                    // 只有在有模型加载且为一阶时才显示转换选项
                    if current_model.geometry.is_some() && !current_model.is_higher_order {
                        if ui.button("Convert to Second Order").clicked() {
                            convert_events.send(events::ConvertToHigherOrderEvent { order: 2 });
                        }
                    } else if current_model.geometry.is_some() && current_model.is_higher_order {
                        ui.label(format!(
                            "Current mesh is {} order",
                            current_model.current_order
                        ));
                    } else {
                        ui.label("Load a model first");
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
    mut current_model: ResMut<CurrentModelData>,            // 添加当前模型数据
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
                // OBJ文件不保存几何数据到CurrentModelData
                current_model.geometry = None;
                current_model.is_higher_order = false;
                current_model.current_order = 1;

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
                // 首先解析几何数据
                let vtk = match vtkio::Vtk::import(PathBuf::from(format!(
                    "{}",
                    path.to_string_lossy()
                ))) {
                    Ok(vtk) => vtk,
                    Err(err) => {
                        println!("load VTK file failed: {:?}", err);
                        if window_exists {
                            egui::Window::new("Error").show(egui_context.ctx_mut(), |ui| {
                                ui.label(format!("load file failed: {:?}", err));
                            });
                        }
                        continue;
                    }
                };

                let geometry = match vtk.data {
                    vtkio::model::DataSet::UnstructuredGrid { meta: _, pieces } => {
                        let extractor = mesh::vtk::UnstructuredGridExtractor;
                        match extractor.process_legacy(pieces) {
                            Ok(geo) => geo,
                            Err(err) => {
                                println!("Failed to extract UnstructuredGrid: {:?}", err);
                                continue;
                            }
                        }
                    }
                    vtkio::model::DataSet::PolyData { meta: _, pieces } => {
                        let extractor = mesh::vtk::PolyDataExtractor;
                        match extractor.process_legacy(pieces) {
                            Ok(geo) => geo,
                            Err(err) => {
                                println!("Failed to extract PolyData: {:?}", err);
                                continue;
                            }
                        }
                    }
                    _ => {
                        println!("Unsupported VTK data type");
                        continue;
                    }
                };

                // 保存几何数据到CurrentModelData
                current_model.geometry = Some(geometry.clone());
                current_model.is_higher_order = false;
                current_model.current_order = 1;

                // 现在创建可渲染的网格
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

/// 处理高阶网格转换事件
fn handle_higher_order_conversion(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut convert_events: EventReader<events::ConvertToHigherOrderEvent>,
    mut current_model: ResMut<CurrentModelData>,
    mut model_loaded_events: EventWriter<ModelLoadedEvent>,
    // 查询现有的模型实体以便替换
    model_entities: Query<Entity, With<Mesh3d>>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    let window_exists = windows.iter().next().is_some();

    for convert_event in convert_events.read() {
        if let Some(ref geometry) = current_model.geometry {
            println!("Converting model to order {} mesh", convert_event.order);

            match mesh::convert_to_higher_order(geometry, convert_event.order) {
                Ok(higher_order_geometry) => {
                    match mesh::process_higher_order_geometry(higher_order_geometry.clone()) {
                        Ok(new_mesh) => {
                            // 删除现有的模型实体
                            for entity in model_entities.iter() {
                                commands.entity(entity).despawn();
                            }

                            let position = Vec3::new(0.0, 0.5, 0.0);
                            let scale = Vec3::ONE;

                            // 创建新的模型实体
                            commands.spawn((
                                Mesh3d(meshes.add(new_mesh.clone())),
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

                            println!("Converted mesh vertices: {}", new_mesh.count_vertices());

                            // 更新当前模型数据
                            current_model.geometry = Some(higher_order_geometry);
                            current_model.is_higher_order = true;
                            current_model.current_order = convert_event.order;

                            // 发送模型加载完成事件
                            model_loaded_events.send(ModelLoadedEvent {
                                position,
                                scale,
                                bounds_min: None, // 这里可以计算新的包围盒
                                bounds_max: None,
                            });

                            if window_exists {
                                egui::Window::new("Conversion Success").show(
                                    egui_context.ctx_mut(),
                                    |ui| {
                                        ui.label(format!(
                                            "Successfully converted to order {} mesh",
                                            convert_event.order
                                        ));
                                    },
                                );
                            }
                        }
                        Err(err) => {
                            println!("Failed to process higher order geometry: {:?}", err);
                            if window_exists {
                                egui::Window::new("Conversion Error").show(
                                    egui_context.ctx_mut(),
                                    |ui| {
                                        ui.label(format!(
                                            "Failed to process higher order geometry: {:?}",
                                            err
                                        ));
                                    },
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("Failed to convert to higher order: {:?}", err);
                    if window_exists {
                        egui::Window::new("Conversion Error").show(egui_context.ctx_mut(), |ui| {
                            ui.label(format!("Failed to convert to higher order: {:?}", err));
                        });
                    }
                }
            }
        } else {
            println!("No model loaded to convert");
            if window_exists {
                egui::Window::new("No Model").show(egui_context.ctx_mut(), |ui| {
                    ui.label("Please load a model first before converting");
                });
            }
        }
    }
}
