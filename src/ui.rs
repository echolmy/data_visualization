pub mod events;
use crate::mesh;
use crate::mesh::vtk::VtkMeshExtractor; // 添加VtkMeshExtractor trait导入
use bevy::prelude::*;
use bevy_egui::*;
use rfd::FileDialog;
use std::path::PathBuf;
use vtkio; // 添加vtkio导入

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
}

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<events::OpenFileEvent>()
            .add_event::<events::LoadModelEvent>()
            .add_event::<events::ToggleWireframeEvent>() // 注册线框切换事件
            .add_event::<events::SubdivideMeshEvent>() // 注册细分事件
            .add_event::<ModelLoadedEvent>() // 注册新事件
            .init_resource::<CurrentModelData>() // 注册当前模型数据资源
            .add_systems(
                Update,
                (
                    initialize_ui_systems,
                    file_dialog_system,
                    load_resource,
                    handle_subdivision, // 添加处理细分的系统
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
    mut subdivide_events: EventWriter<events::SubdivideMeshEvent>, // 添加细分事件写入器
    current_model: Res<CurrentModelData>,                          // 添加当前模型数据访问
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
                    // 细分选项 - 只要有模型就可以细分
                    if current_model.geometry.is_some() {
                        ui.label("Subdivision:");

                        if ui.button("Subdivide").clicked() {
                            subdivide_events.send(events::SubdivideMeshEvent);
                        }
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
            .add_filter("model", &["obj", "glb", "vtk", "vtu"])
            .set_directory("/")
            .pick_file()
        {
            let filepath = PathBuf::from(file.display().to_string());
            println!("open file: {}", filepath.display());
            load_events.send(events::LoadModelEvent(filepath));
        };
    }
}

/// 加载并处理3D模型资源文件
///
/// # 参数
/// * `commands` - Bevy命令系统，用于生成实体
/// * `asset_server` - 资源服务器，用于加载资源
/// * `meshes` - 网格资源管理器
/// * `materials` - 材质资源管理器
/// * `load_events` - 加载模型事件读取器
/// * `model_loaded_events` - 模型加载完成事件写入器
/// * `current_model` - 当前模型数据
/// * `egui_context` - Egui上下文
/// * `windows` - 窗口查询
///
/// # 功能
/// * 支持加载OBJ和VTK格式的3D模型文件
/// * 对于OBJ文件，直接通过资源服务器加载
/// * 对于VTK文件，解析几何数据并创建可渲染的网格
/// * 更新当前模型状态并发送加载完成事件
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

                model_loaded_events.send(ModelLoadedEvent {
                    position,
                    scale,
                    bounds_min: None,
                    bounds_max: None,
                });
            }
            // VTK extension:
            // Legacy: .vtk
            Some("vtk" | "vtu") => {
                // 1. 导入VTK文件
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

                // 打印VTK文件的基本信息
                mesh::print_vtk_info(&vtk);

                // 2. 解析VTK文件获取几何数据
                let geometry = match vtk.data {
                    // 2.1 处理UnstructuredGrid
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
                    // 2.2 处理PolyData
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
                    // 2.3 TODO: 支持其他数据类型
                    _ => {
                        println!("Unsupported VTK data type");
                        continue;
                    }
                };

                println!(
                    "Extracted geometry data attributes: {:?}",
                    &geometry.attributes
                );

                // 打印几何数据的基本信息
                mesh::print_geometry_info(&geometry);

                // 3. 保存几何数据到CurrentModelData
                current_model.geometry = Some(geometry.clone());

                // 4. 使用已解析的geometry直接创建可渲染的mesh
                let mesh = mesh::create_mesh_from_geometry(&geometry);

                let position = Vec3::new(0.0, 0.5, 0.0);
                let scale = Vec3::ONE;

                // 5. 计算模型包围盒
                let mut bounds_min = None;
                let mut bounds_max = None;

                if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    if let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) =
                        positions
                    {
                        // 6. 初始化包围盒
                        if !positions.is_empty() {
                            let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
                            let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

                            // 7. 遍历所有顶点，更新包围盒
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

                // 8. 创建实体
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

                // 9. 发送模型加载完成事件，包含包围盒信息
                model_loaded_events.send(ModelLoadedEvent {
                    position,
                    scale,
                    bounds_min,
                    bounds_max,
                });
            }
            // XML: .vtp (多边形数据), .vts (结构网格),
            //      .vtr (矩形网格), .vti (图像数据)
            Some("vtp" | "vts" | "vtr" | "vti") => {
                // 11. show the message that this format is not supported
                if window_exists {
                    egui::Window::new("Note").show(egui_context.ctx_mut(), |ui| {
                        ui.label("currently not supported this format, developing...");
                    });
                }
            }
            _ => {
                println!("currently not supported other formats, please select another model.");
                // 12. show the message that this format is not supported
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

/// 处理网格细分事件
fn handle_subdivision(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut subdivide_events: EventReader<events::SubdivideMeshEvent>,
    mut current_model: ResMut<CurrentModelData>,
    mut model_entities: Query<&mut Mesh3d>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    let window_exists = windows.iter().next().is_some();

    for _subdivide_event in subdivide_events.read() {
        if let Some(ref geometry) = current_model.geometry {
            match mesh::subdivision::subdivide_mesh(geometry) {
                Ok(subdivided_geometry) => {
                    // 使用通用的网格创建函数处理细分后的几何数据
                    let new_mesh = mesh::create_mesh_from_geometry(&subdivided_geometry);

                    // 找到第一个模型实体并更新其mesh
                    if let Ok(mut mesh3d) = model_entities.get_single_mut() {
                        *mesh3d = Mesh3d(meshes.add(new_mesh.clone()));
                        println!(
                            "Updated existing mesh, now has {} vertices",
                            new_mesh.count_vertices()
                        );
                    } else {
                        // 如果没有找到现有实体，则创建新的（降级处理）
                        let position = Vec3::new(0.0, 0.5, 0.0);
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
                    }

                    // 更新当前模型数据 - 细分后的网格仍然是线性网格，可以继续细分
                    current_model.geometry = Some(subdivided_geometry);

                    if window_exists {
                        egui::Window::new("Subdivision Success").show(
                            egui_context.ctx_mut(),
                            |ui| {
                                ui.label("Successfully completed one subdivision");
                            },
                        );
                    }
                }
                Err(err) => {
                    println!("Subdivision failed: {:?}", err);
                    if window_exists {
                        egui::Window::new("Subdivision Error").show(egui_context.ctx_mut(), |ui| {
                            ui.label(format!("Subdivision failed: {:?}", err));
                        });
                    }
                }
            }
        } else {
            println!("No model loaded for subdivision");
            if window_exists {
                egui::Window::new("No Model").show(egui_context.ctx_mut(), |ui| {
                    ui.label("Please load a model first before subdivision");
                });
            }
        }
    }
}
