pub mod color_bar;
pub mod events;
use crate::animation::TimeSeriesEvent;
use crate::mesh;
use crate::mesh::vtk::VtkMeshExtractor;
use bevy::prelude::*;
use bevy_egui::*;
pub use color_bar::ColorBarConfig;
use rfd::FileDialog;
use std::path::PathBuf;
use vtkio;

/// Marker component to identify imported models
#[derive(Component)]
pub struct UserModelMesh;

#[derive(Event)]
pub struct ModelLoadedEvent {
    pub position: Vec3,
    pub scale: Vec3,
    pub bounds_min: Option<Vec3>, // Minimum point of model bounding box
    pub bounds_max: Option<Vec3>, // Maximum point of model bounding box
}

// Store current model's geometry data
#[derive(Resource, Default)]
pub struct CurrentModelData {
    pub geometry: Option<mesh::GeometryData>,
}

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<events::LoadModelEvent>()
            .add_event::<events::ToggleWireframeEvent>()
            .add_event::<events::SubdivideMeshEvent>()
            .add_event::<events::GenerateWaveEvent>()
            .add_event::<events::GenerateWaveShaderEvent>()
            .add_event::<events::ClearAllMeshesEvent>()
            .add_event::<events::GenerateLODEvent>()
            .add_event::<ModelLoadedEvent>()
            .init_resource::<CurrentModelData>()
            .init_resource::<ColorBarConfig>()
            .add_systems(
                Update,
                (
                    initialize_ui_systems,
                    check_pending_file_load,
                    load_resource,
                    handle_subdivision,
                    handle_wave_generation,
                    handle_wave_shader_generation,
                    handle_clear_all_meshes,
                    handle_lod_generation,
                    color_bar::apply_color_map_changes,
                )
                    .after(EguiSet::InitContexts),
            );
        // .add_plugins(ObjPlugin);
    }
}

fn initialize_ui_systems(
    mut contexts: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    _load_events: EventWriter<events::LoadModelEvent>,
    mut wireframe_toggle_events: EventWriter<events::ToggleWireframeEvent>,
    mut subdivide_events: EventWriter<events::SubdivideMeshEvent>,
    mut wave_events: EventWriter<events::GenerateWaveEvent>,
    mut wave_shader_events: EventWriter<events::GenerateWaveShaderEvent>,
    mut clear_events: EventWriter<events::ClearAllMeshesEvent>,
    mut lod_events: EventWriter<events::GenerateLODEvent>,
    mut time_series_events: EventWriter<TimeSeriesEvent>,
    current_model: Res<CurrentModelData>,
    animation_asset: Res<crate::animation::TimeSeriesAsset>,
    mut color_bar_config: ResMut<ColorBarConfig>,
    windows: Query<&Window>,
) {
    // Handle keyboard shortcuts
    if keyboard_input.just_pressed(KeyCode::Delete) {
        clear_events.send(events::ClearAllMeshesEvent);
    }

    // Only access egui context when window exists
    if windows.iter().next().is_some() {
        egui::TopBottomPanel::top("Menu Bar").show(contexts.ctx_mut(), |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Import").clicked() {
                        // Use async file dialog to avoid main thread blocking
                        std::thread::spawn(move || {
                            if let Some(file) = FileDialog::new()
                                .add_filter("model", &["obj", "glb", "vtk", "vtu"])
                                .set_directory(
                                    &std::env::var("HOME").unwrap_or_else(|_| "/".to_string()),
                                )
                                .pick_file()
                            {
                                println!("Selected file: {}", file.display());

                                let temp_file = std::env::temp_dir().join("pending_file_load.txt");
                                if let Err(e) =
                                    std::fs::write(&temp_file, file.to_string_lossy().as_bytes())
                                {
                                    eprintln!("Failed to write pending file: {}", e);
                                }
                            }
                        });
                    }

                    ui.separator();

                    if ui.button("Import Time Series").clicked() {
                        // Select time series folder
                        std::thread::spawn(move || {
                            if let Some(folder) = FileDialog::new()
                                .set_directory(
                                    &std::env::var("HOME").unwrap_or_else(|_| "/".to_string()),
                                )
                                .pick_folder()
                            {
                                println!("Selected time series folder: {}", folder.display());
                                // Scan VTK files in the folder
                                let mut vtk_files = Vec::new();
                                if let Ok(entries) = std::fs::read_dir(&folder) {
                                    for entry in entries {
                                        if let Ok(entry) = entry {
                                            let path = entry.path();
                                            if path.extension().and_then(|ext| ext.to_str())
                                                == Some("vtu")
                                            {
                                                vtk_files.push(path);
                                            }
                                        }
                                    }
                                }

                                // Sort by numerical order (ensure correct time sequence)
                                vtk_files.sort_by(|a, b| {
                                    // Extract numeric part from filename for comparison
                                    let extract_number = |path: &std::path::Path| -> Option<u32> {
                                        let file_stem = path.file_stem()?.to_str()?;
                                        // Find the number after the last underscore
                                        if let Some(pos) = file_stem.rfind('_') {
                                            file_stem[pos + 1..].parse().ok()
                                        } else {
                                            // If no underscore, try to parse the whole filename as number
                                            file_stem.parse().ok()
                                        }
                                    };

                                    match (extract_number(a), extract_number(b)) {
                                        (Some(num_a), Some(num_b)) => num_a.cmp(&num_b),
                                        (Some(_), None) => std::cmp::Ordering::Less,
                                        (None, Some(_)) => std::cmp::Ordering::Greater,
                                        (None, None) => a.cmp(b),
                                    }
                                });

                                println!("Found {} VTK files in time series", vtk_files.len());
                                if vtk_files.len() > 0 {
                                    println!("First file: {}", vtk_files[0].display());
                                    println!(
                                        "Last file: {}",
                                        vtk_files[vtk_files.len() - 1].display()
                                    );
                                }

                                if !vtk_files.is_empty() {
                                    let temp_file =
                                        std::env::temp_dir().join("pending_time_series.txt");
                                    let file_list = vtk_files
                                        .iter()
                                        .map(|p| p.to_string_lossy().to_string())
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    if let Err(e) = std::fs::write(&temp_file, file_list) {
                                        eprintln!("Failed to write pending time series: {}", e);
                                    }
                                } else {
                                    eprintln!("No VTK files found in selected folder");
                                }
                            }
                        });
                    }

                    ui.separator();

                    if ui.button("Quit").clicked() {
                        std::process::exit(0);
                    }
                });

                // Add View menu
                egui::menu::menu_button(ui, "View", |ui| {
                    if ui.button("Wireframe").clicked() {
                        wireframe_toggle_events.send(events::ToggleWireframeEvent);
                    }

                    ui.separator();

                    // Color bar control
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

                    ui.separator();

                    // Debug information
                    ui.label("Debug Info:");
                    if animation_asset.is_loaded && animation_asset.get_total_time_steps() > 1 {
                        ui.label(format!(
                            "Time series loaded: {} frames",
                            animation_asset.get_total_time_steps()
                        ));
                        if let Some(mesh_entity) = animation_asset.mesh_entity {
                            ui.label(format!("Mesh entity: {:?}", mesh_entity));
                        } else {
                            ui.label("No mesh entity");
                        }
                    } else if animation_asset.is_loaded
                        && animation_asset.get_total_time_steps() == 1
                    {
                        ui.label("Single file loaded (not a time series)");
                    } else {
                        ui.label("No time series loaded");
                    }
                });

                // Add Mesh menu
                egui::menu::menu_button(ui, "Mesh", |ui| {
                    // Subdivision options
                    if current_model.geometry.is_some() {
                        ui.label("Operations:");

                        if ui.button("Subdivide").clicked() {
                            subdivide_events.send(events::SubdivideMeshEvent);
                        }

                        if ui.button("Generate LOD").clicked() {
                            lod_events.send(events::GenerateLODEvent);
                        }
                    } else {
                        ui.label("Load a model first");
                    }

                    ui.separator();

                    // Wave generation
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

        if color_bar_config.visible {
            color_bar::render_color_bar_inline(&mut contexts, color_bar_config);
        }

        // Add time series animation control panel
        if animation_asset.is_loaded && animation_asset.get_total_time_steps() > 1 {
            egui::TopBottomPanel::bottom("time_series_animation")
                .resizable(false)
                .min_height(120.0)
                .show(contexts.ctx_mut(), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading("Time Series Animation Control");
                            ui.separator();
                            if animation_asset.is_step2_complete {
                                ui.colored_label(egui::Color32::GREEN, "✓ Animation Ready");
                            } else {
                                ui.colored_label(egui::Color32::YELLOW, "● Loading...");
                            }
                        });

                        ui.separator();

                        // Only show animation controls when step 2 is complete
                        if animation_asset.is_step2_complete {
                            // Playback control buttons
                            ui.horizontal(|ui| {
                                // Play/Pause button
                                if animation_asset.is_playing {
                                    if ui.button("⏸ Pause").clicked() {
                                        time_series_events.send(TimeSeriesEvent::Pause);
                                    }
                                } else {
                                    if ui.button("▶ Play").clicked() {
                                        time_series_events.send(TimeSeriesEvent::Play);
                                    }
                                }

                                // Stop button
                                if ui.button("⏹ Stop").clicked() {
                                    time_series_events.send(TimeSeriesEvent::Stop);
                                }

                                ui.separator();

                                // Single step control
                                if ui.button("⏮ Prev Frame").clicked() {
                                    time_series_events.send(TimeSeriesEvent::PrevTimeStep);
                                }
                                if ui.button("⏭ Next Frame").clicked() {
                                    time_series_events.send(TimeSeriesEvent::NextTimeStep);
                                }

                                ui.separator();

                                // Loop playback toggle
                                let loop_text = if animation_asset.loop_animation {
                                    "🔄 Loop On"
                                } else {
                                    "🔄 Loop Off"
                                };
                                if ui.button(loop_text).clicked() {
                                    time_series_events.send(TimeSeriesEvent::ToggleLoop);
                                }
                            });

                            // Time step progress bar
                            ui.horizontal(|ui| {
                                ui.label("Time Step:");
                                let total_steps = animation_asset.get_total_time_steps();
                                let mut current_step = animation_asset.current_time_step;

                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut current_step,
                                            0..=(total_steps.saturating_sub(1)),
                                        )
                                        .text("Frame")
                                        .show_value(true),
                                    )
                                    .changed()
                                {
                                    time_series_events
                                        .send(TimeSeriesEvent::SetTimeStep(current_step));
                                }

                                ui.label(format!("{}/{}", current_step + 1, total_steps));
                            });

                            // FPS control
                            ui.horizontal(|ui| {
                                ui.label("Playback Speed:");
                                let mut fps = animation_asset.fps;
                                if ui
                                    .add(
                                        egui::Slider::new(&mut fps, 0.1..=30.0)
                                            .text("FPS")
                                            .show_value(true),
                                    )
                                    .changed()
                                {
                                    time_series_events.send(TimeSeriesEvent::SetFPS(fps));
                                }
                            });

                            // Current file information
                            if let Some(current_data) = animation_asset.get_current_time_step_data()
                            {
                                ui.horizontal(|ui| {
                                    ui.label("Current File:");
                                    if let Some(file_name) = current_data.file_path.file_name() {
                                        if let Some(name_str) = file_name.to_str() {
                                            ui.monospace(name_str);
                                        }
                                    }
                                });
                            }
                        } else {
                            // Step 2 loading status
                            ui.horizontal(|ui| {
                                ui.label("Status: Loading time series data...");
                                ui.label(format!(
                                    "Loaded: {}/{}",
                                    animation_asset.time_steps.len(),
                                    animation_asset.all_file_paths.len()
                                ));
                            });
                        }
                    });
                });
        }
    }
}

// Load and process 3D model resource files
fn load_resource(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut load_events: EventReader<events::LoadModelEvent>,
    mut model_loaded_events: EventWriter<ModelLoadedEvent>,
    mut current_model: ResMut<CurrentModelData>,
    color_bar_config: ResMut<ColorBarConfig>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
    mesh_entities: Query<Entity, With<UserModelMesh>>,
) {
    // Check if window exists
    let window_exists = windows.iter().next().is_some();

    for events::LoadModelEvent(path) in load_events.read() {
        // Clear existing user models from scene before importing new model
        let cleared_count =
            clear_existing_models_silent(&mut commands, &mesh_entities, &mut current_model);
        if cleared_count > 0 {
            println!(
                "Cleared {} existing models before importing new model",
                cleared_count
            );
        }

        match path.extension().and_then(|ext| ext.to_str()) {
            Some("obj") => {
                let position = Vec3::new(0.0, 0.5, 0.0);
                let scale = Vec3::splat(1.0);

                commands.spawn((
                    Mesh3d(asset_server.load(format!("{}", path.to_string_lossy()))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        metallic: 0.2,
                        perceptual_roughness: 0.4,
                        reflectance: 0.5,
                        unlit: false,
                        alpha_mode: AlphaMode::Opaque,
                        ..default()
                    })),
                    Transform::from_translation(position).with_scale(scale),
                ));

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
                // 1. Import VTK file
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

                // Print VTK information
                // mesh::print_vtk_info(&vtk);

                // 2. Parse VTK file to get geometry data
                let geometry = match vtk.data {
                    // 2.1 Process UnstructuredGrid
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
                    // 2.2 Process PolyData
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
                    // 2.3 TODO: Support other data types
                    _ => {
                        println!("Unsupported VTK data type");
                        continue;
                    }
                };

                println!(
                    "Extracted geometry data attributes: {:?}",
                    &geometry.attributes
                );

                // Print geometry info for debugging
                // mesh::print_geometry_info(&geometry);

                // 3. Save geometry data to CurrentModelData
                current_model.geometry = Some(geometry.clone());

                // color_bar::update_color_bar_range_from_geometry(&geometry, &mut color_bar_config);

                // 4. Use parsed geometry to directly create mesh
                let mut mesh = mesh::create_mesh_from_geometry(&geometry);

                // 5. Apply selected color mapping
                if let Err(e) =
                    color_bar::apply_custom_color_mapping(&geometry, &mut mesh, &color_bar_config)
                {
                    println!("Failed to apply initial color mapping: {:?}", e);
                }

                let position = Vec3::new(0.0, 0.5, 0.0);
                let scale = Vec3::ONE;

                // 6. Calculate model bounding box
                let mut bounds_min = None;
                let mut bounds_max = None;

                if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    if let bevy::render::mesh::VertexAttributeValues::Float32x3(positions) =
                        positions
                    {
                        // 7. Initialize bounding box
                        if !positions.is_empty() {
                            let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
                            let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

                            // 8. Iterate through all vertices, update bounding box
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

                // 9. Create entity
                commands.spawn((
                    Mesh3d(meshes.add(mesh.clone())),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 1.0, 1.0),
                        metallic: 0.2,
                        perceptual_roughness: 0.4,
                        reflectance: 0.5,
                        cull_mode: None,
                        unlit: false,
                        alpha_mode: AlphaMode::Opaque,
                        ..default()
                    })),
                    Transform::from_translation(position),
                    Visibility::Visible,
                    UserModelMesh,
                ));

                println!("number of vertices: {:?}", mesh.count_vertices());

                // 10. Send model loaded complete event
                model_loaded_events.send(ModelLoadedEvent {
                    position,
                    scale,
                    bounds_min,
                    bounds_max,
                });
            }
            // XML: .vtp (polygon data), .vts (structured grid),
            //      .vtr (rectilinear grid), .vti (image data)
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

/// Handle mesh subdivision events
fn handle_subdivision(
    _commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    mut subdivide_events: EventReader<events::SubdivideMeshEvent>,
    mut current_model: ResMut<CurrentModelData>,
    mut model_entities: Query<&mut Mesh3d, With<UserModelMesh>>,
    color_bar_config: Res<ColorBarConfig>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    let window_exists = windows.iter().next().is_some();

    for _subdivide_event in subdivide_events.read() {
        if let Some(ref geometry) = current_model.geometry {
            match mesh::subdivision::subdivide_mesh(geometry) {
                Ok(subdivided_geometry) => {
                    // Create subdivided geometry data
                    let mut new_mesh = mesh::create_mesh_from_geometry(&subdivided_geometry);

                    // Apply currently selected color mapping to subdivided mesh
                    if let Err(e) = color_bar::apply_custom_color_mapping(
                        &subdivided_geometry,
                        &mut new_mesh,
                        &color_bar_config,
                    ) {
                        println!("Failed to apply color mapping to subdivided mesh: {:?}", e);
                    }

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
                        return;
                    }

                    // Update current model data
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

/// Handle wave generation
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
        // Create default wave parameters
        let wave = PlaneWave::default();

        // Generate wave mesh
        let wave_mesh = generate_wave_surface(
            &wave, 10.0, // Width
            10.0, // Depth
            50,   // Width resolution
            50,   // Depth resolution
        );

        let position = Vec3::new(0.0, 0.0, 0.0);

        // Create wave entity
        commands.spawn((
            Mesh3d(meshes.add(wave_mesh.clone())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.6, 1.0),
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

        // Clear current model data
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

/// Handle GPU Shader wave generation
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
        // Create flat plane mesh for shader deformation
        let plane_mesh = create_flat_plane_mesh(
            50,                                // width resolution
            50,                                // height resolution
            bevy::math::Vec2::new(10.0, 10.0), // size
        );

        // Create wave material
        let wave_material = WaveMaterial::new(
            1.0,
            0.0,
            0.5,
            0.5,
            1.0,
            0.0,
            bevy::math::Vec3::new(0.2, 0.2, 0.8),
        );

        let position = Vec3::new(0.0, 0.0, 0.0);

        // Create wave entity using shader material
        commands.spawn((
            Mesh3d(meshes.add(plane_mesh.clone())),
            MeshMaterial3d(wave_materials.add(wave_material)),
            Transform::from_translation(position),
        ));

        // Clear current model data
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

/// Clear existing models
pub fn clear_existing_models_silent(
    commands: &mut Commands,
    mesh_entities: &Query<Entity, With<UserModelMesh>>,
    current_model: &mut ResMut<CurrentModelData>,
) -> usize {
    let mesh_count = mesh_entities.iter().count();

    if mesh_count > 0 {
        // Iterate through all user imported mesh entities and delete them
        for entity in mesh_entities.iter() {
            commands.entity(entity).despawn();
        }

        // Clear current model data
        current_model.geometry = None;
    }

    mesh_count
}

fn handle_clear_all_meshes(
    mut commands: Commands,
    mut clear_events: EventReader<events::ClearAllMeshesEvent>,
    mesh_entities: Query<Entity, With<UserModelMesh>>,
    mut current_model: ResMut<CurrentModelData>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    let window_exists = windows.iter().next().is_some();

    for _clear_event in clear_events.read() {
        let mesh_count = mesh_entities.iter().count();

        if mesh_count > 0 {
            for entity in mesh_entities.iter() {
                commands.entity(entity).despawn();
            }

            // Clear current model data
            current_model.geometry = None;

            println!("Cleared {} user mesh entities", mesh_count);

            if window_exists {
                egui::Window::new("Clear Complete").show(egui_context.ctx_mut(), |ui| {
                    ui.label(format!("Successfully cleared {} meshes", mesh_count));
                });
            }
        } else {
            println!("No user meshes in scene to clear");
            if window_exists {
                egui::Window::new("Notice").show(egui_context.ctx_mut(), |ui| {
                    ui.label("No user meshes in scene to clear");
                });
            }
        }
    }
}

/// Handle LOD generation events
fn handle_lod_generation(
    mut commands: Commands,
    mut lod_events: EventReader<events::GenerateLODEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    current_model: Res<CurrentModelData>,
    model_entities: Query<Entity, (With<UserModelMesh>, Without<crate::lod::LODManager>)>,
    mut egui_context: EguiContexts,
    windows: Query<&Window>,
) {
    let window_exists = windows.iter().next().is_some();

    for _lod_event in lod_events.read() {
        if let Some(ref geometry) = current_model.geometry {
            // Add LOD manager to all user model entities
            let mut entities_processed = 0;
            for entity in model_entities.iter() {
                match crate::lod::LODManager::new(geometry.clone(), &mut meshes) {
                    Ok(lod_manager) => {
                        commands.entity(entity).insert(lod_manager);
                        entities_processed += 1;
                        println!("Successfully created LOD manager for entity {:?}", entity);
                    }
                    Err(e) => {
                        println!(
                            "Failed to create LOD manager for entity {:?}: {:?}",
                            entity, e
                        );
                    }
                }
            }

            if entities_processed > 0 {
                println!(
                    "Successfully generated LOD for {} entities",
                    entities_processed
                );
                if window_exists {
                    egui::Window::new("LOD Generation Complete").show(
                        egui_context.ctx_mut(),
                        |ui| {
                            ui.label(format!(
                                "Successfully generated LOD for {} models",
                                entities_processed
                            ));
                            ui.label("LOD will automatically switch based on camera distance");
                        },
                    );
                }
            } else {
                println!("No model entities found that can generate LOD");
                if window_exists {
                    egui::Window::new("Notice").show(egui_context.ctx_mut(), |ui| {
                        ui.label("No models found that can generate LOD");
                        ui.label("Please import a model first, or LOD already exists");
                    });
                }
            }
        } else {
            println!("Currently no geometry data, cannot generate LOD");
            if window_exists {
                egui::Window::new("Error").show(egui_context.ctx_mut(), |ui| {
                    ui.label("Currently no model data");
                    ui.label("Please import a VTK file first");
                });
            }
        }
    }
}

/// Check for pending file load requests
fn check_pending_file_load(
    mut load_events: EventWriter<events::LoadModelEvent>,
    mut time_series_events: EventWriter<TimeSeriesEvent>,
) {
    // Check for regular file loading
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
        let _ = std::fs::remove_file(&temp_file);
    }

    // Check for time series file loading
    let time_series_file = std::env::temp_dir().join("pending_time_series.txt");
    if time_series_file.exists() {
        if let Ok(file_list_str) = std::fs::read_to_string(&time_series_file) {
            let file_paths: Vec<PathBuf> = file_list_str
                .lines()
                .filter_map(|line| {
                    let path = PathBuf::from(line.trim());
                    if path.exists() {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect();

            if !file_paths.is_empty() {
                println!("Loading time series with {} files", file_paths.len());
                time_series_events.send(TimeSeriesEvent::LoadSeries(file_paths));
            }
        }
        let _ = std::fs::remove_file(&time_series_file);
    }
}
