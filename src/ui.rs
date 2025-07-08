pub mod color_bar;
pub mod events;
use crate::mesh;
use crate::mesh::vtk::VtkMeshExtractor;
use bevy::prelude::*;
use bevy_egui::*;
use rfd::FileDialog;
use std::path::PathBuf;
use vtkio;

// 重新导出颜色条相关的公共接口
pub use color_bar::ColorBarConfig;

/// 标记组件，用于标识用户导入的模型网格
/// 只有带有此组件的网格才会受到颜色映射的影响
#[derive(Component)]
pub struct UserModelMesh;

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
    pub geometry: Option<mesh::GeometryData>,
}

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<events::LoadModelEvent>()
            .add_event::<events::ToggleWireframeEvent>() // register toggle wireframe event
            .add_event::<events::SubdivideMeshEvent>() // register subdivide mesh event
            .add_event::<events::GenerateWaveEvent>() // register generate wave event
            .add_event::<events::GenerateWaveShaderEvent>() // register generate wave shader event
            .add_event::<events::ClearAllMeshesEvent>() // register clear all meshes event
            .add_event::<ModelLoadedEvent>() // register model loaded event
            .init_resource::<CurrentModelData>() // register current model data resource
            .init_resource::<ColorBarConfig>() // register color bar config resource
            .add_systems(
                Update,
                (
                    initialize_ui_systems,
                    check_pending_file_load, // add pending file load check system
                    load_resource,
                    handle_subdivision,            // add handle subdivision system
                    handle_wave_generation,        // add handle wave generation system
                    handle_wave_shader_generation, // add handle wave shader generation system
                    handle_clear_all_meshes,       // add handle clear all meshes system
                    color_bar::apply_color_map_changes, // add color map change handling system
                )
                    .after(EguiSet::InitContexts),
            );
        // .add_plugins(ObjPlugin);
    }
}

fn initialize_ui_systems(
    mut contexts: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>, // 添加键盘输入
    _load_events: EventWriter<events::LoadModelEvent>, // 不再直接使用，通过临时文件传递
    mut wireframe_toggle_events: EventWriter<events::ToggleWireframeEvent>,
    mut subdivide_events: EventWriter<events::SubdivideMeshEvent>, // 添加细分事件写入器
    mut wave_events: EventWriter<events::GenerateWaveEvent>,       // 添加波形生成事件写入器
    mut wave_shader_events: EventWriter<events::GenerateWaveShaderEvent>, // 添加GPU shader波形生成事件写入器
    mut clear_events: EventWriter<events::ClearAllMeshesEvent>, // 添加清除所有mesh事件写入器
    current_model: Res<CurrentModelData>,                       // 添加当前模型数据访问
    mut color_bar_config: ResMut<ColorBarConfig>,               // 添加颜色条配置访问
    windows: Query<&Window>,
) {
    // 处理键盘快捷键
    if keyboard_input.just_pressed(KeyCode::Delete) {
        clear_events.send(events::ClearAllMeshesEvent);
    }

    // 只有在窗口存在时才访问egui上下文
    if windows.iter().next().is_some() {
        egui::TopBottomPanel::top("Menu Bar").show(contexts.ctx_mut(), |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Import").clicked() {
                        // 使用异步文件对话框避免主线程阻塞
                        std::thread::spawn(move || {
                            if let Some(file) = FileDialog::new()
                                .add_filter("model", &["obj", "glb", "vtk", "vtu"])
                                .set_directory(
                                    &std::env::var("HOME").unwrap_or_else(|_| "/".to_string()),
                                )
                                .pick_file()
                            {
                                println!("Selected file: {}", file.display());
                                // 通过文件系统传递路径（临时解决方案）
                                let temp_file = std::env::temp_dir().join("pending_file_load.txt");
                                if let Err(e) =
                                    std::fs::write(&temp_file, file.to_string_lossy().as_bytes())
                                {
                                    eprintln!("Failed to write pending file: {}", e);
                                }
                            }
                        });
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

                    ui.separator();

                    // 颜色条控制
                    let color_bar_text = if color_bar_config.visible {
                        "hide color bar"
                    } else {
                        "show color bar"
                    };
                    if ui.button(color_bar_text).clicked() {
                        color_bar_config.visible = !color_bar_config.visible;
                    }

                    ui.separator();

                    if ui.button("Clear User Meshes (Delete)").clicked() {
                        clear_events.send(events::ClearAllMeshesEvent);
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

                    ui.separator();

                    // 波形生成选项
                    ui.label("Generate:");
                    if ui.button("Create Wave Surface (CPU)").clicked() {
                        wave_events.send(events::GenerateWaveEvent);
                    }

                    if ui.button("Create Wave Surface (GPU Shader)").clicked() {
                        wave_shader_events.send(events::GenerateWaveShaderEvent);
                    }
                });
            });
        });

        // 在TopBottomPanel之后立即显示SidePanel，确保正确的布局顺序
        if color_bar_config.visible {
            color_bar::render_color_bar_inline(contexts, color_bar_config);
        }
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
    mut color_bar_config: ResMut<ColorBarConfig>,           // 添加颜色条配置
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
                        metallic: 0.2,                 // 轻微金属感，增强反射
                        perceptual_roughness: 0.4,     // 略微光滑表面，更好的光照反应
                        reflectance: 0.5,              // 适中反射率
                        unlit: false,                  // 确保PBR光照开启
                        alpha_mode: AlphaMode::Opaque, // 改为Opaque获得更好性能
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

                // 自动更新颜色条数值范围
                color_bar::update_color_bar_range_from_geometry(&geometry, &mut color_bar_config);

                // 4. 使用已解析的geometry直接创建可渲染的mesh
                let mut mesh = mesh::create_mesh_from_geometry(&geometry);

                // 5. 应用当前选择的颜色映射表（覆盖VTK文件中的默认颜色）
                if let Err(e) =
                    color_bar::apply_custom_color_mapping(&geometry, &mut mesh, &color_bar_config)
                {
                    println!("Failed to apply initial color mapping: {:?}", e);
                }

                let position = Vec3::new(0.0, 0.5, 0.0);
                let scale = Vec3::ONE;

                // 6. 计算模型包围盒
                let mut bounds_min = None;
                let mut bounds_max = None;

                if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    if let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) =
                        positions
                    {
                        // 7. 初始化包围盒
                        if !positions.is_empty() {
                            let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
                            let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

                            // 8. 遍历所有顶点，更新包围盒
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

                // 9. 创建实体
                commands.spawn((
                    Mesh3d(meshes.add(mesh.clone())),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 1.0, 1.0),
                        metallic: 0.2,             // 轻微金属感，增强反射
                        perceptual_roughness: 0.4, // 略微光滑表面，更好的光照反应
                        reflectance: 0.5,          // 适中反射率
                        cull_mode: None,
                        unlit: false, // 启用PBR光照！
                        alpha_mode: AlphaMode::Opaque,
                        // 启用顶点颜色混合，保持颜色映射功能
                        ..default()
                    })),
                    Transform::from_translation(position),
                    Visibility::Visible,
                    UserModelMesh, // 标记为用户导入的模型网格
                ));

                println!("number of vertices: {:?}", mesh.count_vertices());

                // 10. 发送模型加载完成事件，包含包围盒信息
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
    _commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    mut subdivide_events: EventReader<events::SubdivideMeshEvent>,
    mut current_model: ResMut<CurrentModelData>,
    mut model_entities: Query<&mut Mesh3d, With<UserModelMesh>>,
    color_bar_config: Res<ColorBarConfig>, // 添加颜色条配置访问
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    let window_exists = windows.iter().next().is_some();

    for _subdivide_event in subdivide_events.read() {
        if let Some(ref geometry) = current_model.geometry {
            match mesh::subdivision::subdivide_mesh(geometry) {
                Ok(subdivided_geometry) => {
                    // 使用通用的网格创建函数处理细分后的几何数据
                    let mut new_mesh = mesh::create_mesh_from_geometry(&subdivided_geometry);

                    // 应用当前选择的颜色映射表到细分后的网格
                    if let Err(e) = color_bar::apply_custom_color_mapping(
                        &subdivided_geometry,
                        &mut new_mesh,
                        &color_bar_config,
                    ) {
                        println!("Failed to apply color mapping to subdivided mesh: {:?}", e);
                    }

                    // 找到用户模型实体并更新其mesh，应该总是只有一个
                    if let Ok(mut mesh3d) = model_entities.get_single_mut() {
                        *mesh3d = Mesh3d(meshes.add(new_mesh.clone()));
                        println!(
                            "Updated existing user model mesh, now has {} vertices",
                            new_mesh.count_vertices()
                        );
                    } else {
                        println!("Error: No user model entity found for subdivision! This should not happen.");
                        if window_exists {
                            egui::Window::new("Subdivision Error").show(
                                egui_context.ctx_mut(),
                                |ui| {
                                    ui.label("Error: No user model found for subdivision");
                                    ui.label("This should not happen - please report this bug");
                                },
                            );
                        }
                        return; // 早退出，不更新模型数据
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

/// 处理波形生成事件
fn handle_wave_generation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut wave_events: EventReader<events::GenerateWaveEvent>,
    mut current_model: ResMut<CurrentModelData>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    use crate::mesh::wave::{generate_wave_surface, PlaneWave};

    let window_exists = windows.iter().next().is_some();

    for _wave_event in wave_events.read() {
        // 创建默认波形参数
        let wave = PlaneWave::default();

        // 生成波形网格
        let wave_mesh = generate_wave_surface(
            &wave, 10.0, // 宽度
            10.0, // 深度
            50,   // 宽度分辨率
            50,   // 深度分辨率
        );

        let position = Vec3::new(0.0, 0.0, 0.0);

        // 创建波形实体
        commands.spawn((
            Mesh3d(meshes.add(wave_mesh.clone())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.6, 1.0), // 蓝色，像水一样
                metallic: 0.1,
                perceptual_roughness: 0.3,
                reflectance: 0.8,
                cull_mode: None,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_translation(position),
            Visibility::Visible,
        ));

        // 清除当前模型数据，因为这是新生成的波形
        current_model.geometry = None;

        println!(
            "Generated wave surface with {} vertices",
            wave_mesh.count_vertices()
        );

        if window_exists {
            egui::Window::new("Wave Generated").show(egui_context.ctx_mut(), |ui| {
                ui.label("Successfully generated wave surface!");
                ui.label("Parameters:");
                ui.label("  • Amplitude: 1.0");
                ui.label("  • Wave vector: (0.5, 0.3)");
                ui.label("  • Frequency: 2.0");
                ui.label("  • Resolution: 50x50");
            });
        }
    }
}

/// 处理GPU Shader波形生成事件
fn handle_wave_shader_generation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut wave_materials: ResMut<Assets<crate::render::WaveMaterial>>,
    mut wave_shader_events: EventReader<events::GenerateWaveShaderEvent>,
    mut current_model: ResMut<CurrentModelData>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    use crate::render::{create_flat_plane_mesh, WaveMaterial};

    let window_exists = windows.iter().next().is_some();

    for _wave_shader_event in wave_shader_events.read() {
        // 创建平面网格用于shader变形
        let plane_mesh = create_flat_plane_mesh(
            50,                                // width resolution
            50,                                // height resolution
            bevy::math::Vec2::new(10.0, 10.0), // size
        );

        // 创建波浪材质
        let wave_material = WaveMaterial::new(
            1.0,
            0.0,
            0.5,
            0.5,
            1.0,
            0.0,
            bevy::math::Vec3::new(0.2, 0.2, 0.8),
        );

        let position = Vec3::new(0.0, 0.0, 0.0); // 放在原点位置

        // 创建使用shader材质的波形实体
        commands.spawn((
            Mesh3d(meshes.add(plane_mesh.clone())),
            MeshMaterial3d(wave_materials.add(wave_material)),
            Transform::from_translation(position),
        ));

        // 清除当前模型数据，因为这是新生成的波形
        current_model.geometry = None;

        println!(
            "Generated GPU shader wave surface with {} vertices",
            plane_mesh.count_vertices()
        );

        if window_exists {
            egui::Window::new("GPU Wave Generated").show(egui_context.ctx_mut(), |ui| {
                ui.label("Successfully generated GPU shader wave surface!");
                ui.label("Features:");
                ui.label("  • Real-time GPU wave calculation");
                ui.label("  • Animated wave motion");
                ui.label("  • Dynamic lighting with normals");
                ui.label("Parameters:");
                ui.label("  • Amplitude: 3.0");
                ui.label("  • Wave vector: (1.0, 1.0)");
                ui.label("  • Frequency: 2.0");
                ui.label("  • Resolution: 50x50");
            });
        }
    }
}

/// 处理清除所有mesh事件
fn handle_clear_all_meshes(
    mut commands: Commands,
    mut clear_events: EventReader<events::ClearAllMeshesEvent>,
    // 查询所有用户导入的网格实体
    mesh_entities: Query<Entity, With<UserModelMesh>>,
    mut current_model: ResMut<CurrentModelData>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    let window_exists = windows.iter().next().is_some();

    for _clear_event in clear_events.read() {
        let mesh_count = mesh_entities.iter().count();

        if mesh_count > 0 {
            // 遍历所有用户导入的mesh实体并删除它们（保留坐标系和网格）
            for entity in mesh_entities.iter() {
                commands.entity(entity).despawn();
            }

            // 清除当前模型数据
            current_model.geometry = None;

            println!("清除了 {} 个用户mesh实体（保留坐标系和网格）", mesh_count);

            if window_exists {
                egui::Window::new("清除完成").show(egui_context.ctx_mut(), |ui| {
                    ui.label(format!("成功清除了 {} 个mesh", mesh_count));
                    ui.label("坐标系和网格已保留");
                });
            }
        } else {
            println!("场景中没有用户mesh需要清除");
            if window_exists {
                egui::Window::new("提示").show(egui_context.ctx_mut(), |ui| {
                    ui.label("场景中没有用户mesh需要清除");
                    ui.label("坐标系和网格将保持不变");
                });
            }
        }
    }
}

/// 检查是否有待处理的文件加载请求
fn check_pending_file_load(mut load_events: EventWriter<events::LoadModelEvent>) {
    let temp_file = std::env::temp_dir().join("pending_file_load.txt");

    if temp_file.exists() {
        if let Ok(file_path_str) = std::fs::read_to_string(&temp_file) {
            let file_path = PathBuf::from(file_path_str.trim());
            if file_path.exists() {
                println!(
                    "Loading file from background thread: {}",
                    file_path.display()
                );
                load_events.send(events::LoadModelEvent(file_path));
            }
        }

        // 清理临时文件
        let _ = std::fs::remove_file(&temp_file);
    }
}
