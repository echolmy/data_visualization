// Time series animation system
use crate::mesh::vtk::VtkMeshExtractor;
use bevy::prelude::*;
use std::path::PathBuf;

/// Scalar data for each frame in the time series
#[derive(Clone, Debug)]
pub struct TimeStepData {
    pub scalars: Vec<f32>, // Scalar values for this time step
    #[allow(dead_code)]
    pub time_step: usize, // Time step index
    pub file_path: PathBuf, // Source file path
}

/// Time series asset - Contains static mesh and scalar data for all time steps
#[derive(Resource)]
pub struct TimeSeriesAsset {
    // Step 1: Static model import
    pub pending_first_file: Option<PathBuf>, // First file waiting for import
    pub is_step1_ready: bool,                // Whether step 1 is ready
    pub is_step1_complete: bool,             // Check if step 1 is complete

    // Step 2: Time series data
    pub all_file_paths: Vec<PathBuf>,  // All file paths
    pub time_steps: Vec<TimeStepData>, // Scalar data for all time steps
    pub is_step2_complete: bool,       // Check if step 2 is complete

    // Static mesh data
    pub mesh_entity: Option<Entity>,

    // Geometry data
    pub vertices: Vec<Vec3>, // Static vertex positions
    pub indices: Vec<u32>,   // Static indices

    // Animation control
    pub current_time_step: usize, // Current time step
    pub is_loaded: bool,          // Whether fully loaded
    pub is_playing: bool,         // Whether currently playing
    pub fps: f32,                 // Playback frame rate
    pub timer: Timer,             // Playback timer
    pub loop_animation: bool,     // Whether to loop animation
    pub colors_need_update: bool, // Flag for color update needed
}

/// Time series animation events
#[derive(Event)]
pub enum TimeSeriesEvent {
    LoadSeries(Vec<PathBuf>), // Load time series files
    // Animation control events
    Play,               // Play animation
    Pause,              // Pause animation
    Stop,               // Stop animation
    SetTimeStep(usize), // Set to specific time step
    NextTimeStep,       // Next time step
    PrevTimeStep,       // Previous time step
    SetFPS(f32),        // Set playback frame rate
    ToggleLoop,         // Toggle loop playback
}

impl Default for TimeSeriesAsset {
    fn default() -> Self {
        Self {
            pending_first_file: None,
            is_step1_ready: false,
            is_step1_complete: false,

            all_file_paths: Vec::new(),
            time_steps: Vec::new(),
            is_step2_complete: false,

            mesh_entity: None,
            vertices: Vec::new(),
            indices: Vec::new(),

            current_time_step: 0,
            is_loaded: false,
            is_playing: false,
            fps: 10.0,
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            loop_animation: true,
            colors_need_update: false,
        }
    }
}

impl TimeSeriesAsset {
    /// Start loading time series - Step 1: Import frame 0 as static state
    pub fn start_loading(&mut self, file_paths: Vec<PathBuf>) {
        println!(
            "Loading time series - Step 1: Import frame 0 as static model: {} files available",
            file_paths.len()
        );

        // Reset state
        *self = Self::default();

        // Store all file paths for step 2
        self.all_file_paths = file_paths.clone();

        // Step 1: Only process the first file
        if let Some(first_file) = file_paths.first() {
            println!(
                "Step 1: Loading frame 0 as static model: {}",
                first_file.display()
            );

            // Mark ready to import first file
            self.pending_first_file = Some(first_file.clone());
            self.is_step1_ready = true;
        }
    }

    /// Get current time step data (for UI display)
    pub fn get_current_time_step_data(&self) -> Option<&TimeStepData> {
        self.time_steps.get(self.current_time_step)
    }

    /// Play animation
    pub fn play(&mut self) {
        if self.is_step2_complete && !self.time_steps.is_empty() {
            self.is_playing = true;
            self.colors_need_update = true; // Ensure color update when starting playback
            println!(
                "Started playing time series animation with {} frames",
                self.time_steps.len()
            );
        }
    }

    /// Pause animation
    pub fn pause(&mut self) {
        self.is_playing = false;
        println!("Paused animation at frame {}", self.current_time_step);
    }

    /// Stop animation and return to first frame
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_time_step = 0;
        println!("Stopped animation and returned to frame 0");
    }

    /// Set to specific time step
    pub fn set_time_step(&mut self, step: usize) {
        if step < self.time_steps.len() && step != self.current_time_step {
            self.current_time_step = step;
            self.colors_need_update = true;
            println!("Set to frame {}", step);
        }
    }

    /// Next time step
    pub fn next_time_step(&mut self) {
        if !self.time_steps.is_empty() {
            let old_step = self.current_time_step;
            if self.current_time_step < self.time_steps.len() - 1 {
                self.current_time_step += 1;
            } else if self.loop_animation {
                self.current_time_step = 0;
            } else {
                self.is_playing = false;
            }

            if old_step != self.current_time_step {
                self.colors_need_update = true;
            }
        }
    }

    /// Previous time step
    pub fn prev_time_step(&mut self) {
        if !self.time_steps.is_empty() {
            let old_step = self.current_time_step;
            if self.current_time_step > 0 {
                self.current_time_step -= 1;
            } else if self.loop_animation {
                self.current_time_step = self.time_steps.len() - 1;
            }

            if old_step != self.current_time_step {
                self.colors_need_update = true;
            }
        }
    }

    /// Set playback frame rate
    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps.clamp(0.1, 60.0);
        self.timer = Timer::from_seconds(1.0 / self.fps, TimerMode::Repeating);
        println!("Set playback frame rate to {}fps", self.fps);
    }

    /// Get total time steps
    pub fn get_total_time_steps(&self) -> usize {
        self.time_steps.len()
    }
}

/// Time series animation plugin
pub struct TimeSeriesAnimationPlugin;

impl Plugin for TimeSeriesAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeSeriesAsset>()
            .add_event::<TimeSeriesEvent>()
            .add_systems(
                Update,
                (
                    handle_time_series_events,
                    trigger_first_frame_import, // Step 1: Trigger single file import
                    detect_step1_completion,    // Detect step 1 completion
                    load_all_time_series_data,  // Step 2: Load all time series data
                    update_animation_timer,     // Animation timer
                    update_animation_colors,    // Animation color update
                )
                    .chain(), // Ensure systems execute in order
            );
    }
}

/// Handle time series events
fn handle_time_series_events(
    mut events: EventReader<TimeSeriesEvent>,
    mut time_series_asset: ResMut<TimeSeriesAsset>,
    mut commands: Commands,
    mesh_entities: Query<Entity, With<crate::ui::UserModelMesh>>,
    mut current_model: ResMut<crate::ui::CurrentModelData>,
) {
    for event in events.read() {
        match event {
            TimeSeriesEvent::LoadSeries(file_paths) => {
                // Clear existing user models in the scene before loading time series
                let _cleared_count = crate::ui::clear_existing_models_silent(
                    &mut commands,
                    &mesh_entities,
                    &mut current_model,
                );

                time_series_asset.start_loading(file_paths.clone());
            }
            TimeSeriesEvent::Play => {
                time_series_asset.play();
            }
            TimeSeriesEvent::Pause => {
                time_series_asset.pause();
            }
            TimeSeriesEvent::Stop => {
                time_series_asset.stop();
            }
            TimeSeriesEvent::SetTimeStep(step) => {
                time_series_asset.set_time_step(*step);
            }
            TimeSeriesEvent::NextTimeStep => {
                time_series_asset.next_time_step();
            }
            TimeSeriesEvent::PrevTimeStep => {
                time_series_asset.prev_time_step();
            }
            TimeSeriesEvent::SetFPS(fps) => {
                time_series_asset.set_fps(*fps);
            }
            TimeSeriesEvent::ToggleLoop => {
                time_series_asset.loop_animation = !time_series_asset.loop_animation;
                println!("Loop playback: {}", time_series_asset.loop_animation);
            }
        }
    }
}

/// Animation timer update system
fn update_animation_timer(time: Res<Time>, mut time_series_asset: ResMut<TimeSeriesAsset>) {
    if time_series_asset.is_playing && time_series_asset.is_step2_complete {
        time_series_asset.timer.tick(time.delta());
        if time_series_asset.timer.finished() {
            time_series_asset.next_time_step();
        }
    }
}

/// Animation color update system - Update mesh vertex colors based on current time step
fn update_animation_colors(
    mut time_series_asset: ResMut<TimeSeriesAsset>,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_query: Query<&Mesh3d, With<crate::ui::UserModelMesh>>,
    color_bar_config: Res<crate::ui::ColorBarConfig>,
) {
    // Only process when time series is fully loaded and colors need update
    if !time_series_asset.is_step2_complete
        || time_series_asset.time_steps.is_empty()
        || !time_series_asset.colors_need_update
    {
        return;
    }

    // Get scalar data for current time step
    let current_data = match time_series_asset.get_current_time_step_data() {
        Some(data) => data,
        None => return,
    };

    // Find model mesh and update colors
    let mesh_count = mesh_query.iter().count();
    if mesh_count > 0 {
        for mesh3d in mesh_query.iter() {
            if let Some(mesh) = meshes.get_mut(&mesh3d.0) {
                // Update vertex colors
                apply_scalar_colors_to_mesh(mesh, &current_data.scalars, &color_bar_config);
                println!(
                    "Updated mesh colors for time step {} with {} scalars",
                    time_series_asset.current_time_step,
                    current_data.scalars.len()
                );
            }
        }
        time_series_asset.colors_need_update = false;
    }
}

/// Apply scalar values to mesh vertex colors
fn apply_scalar_colors_to_mesh(
    mesh: &mut Mesh,
    scalars: &[f32],
    color_bar_config: &crate::ui::ColorBarConfig,
) {
    use crate::mesh::color_maps::get_color_map;

    // Get vertex count
    let vertex_count = mesh.count_vertices();

    // Ensure scalar data count matches vertex count
    if scalars.len() != vertex_count {
        println!(
            "Warning: Scalar data count ({}) does not match vertex count ({})",
            scalars.len(),
            vertex_count
        );
        return;
    }

    // Calculate scalar value range
    let (min_val, max_val) = scalars
        .iter()
        .fold((f32::MAX, f32::MIN), |(min, max), &val| {
            (min.min(val), max.max(val))
        });

    // Use range from ColorBarConfig
    let (range_min, range_max) = if color_bar_config.max_value > color_bar_config.min_value {
        (color_bar_config.min_value, color_bar_config.max_value)
    } else {
        (min_val, max_val)
    };

    // Get currently selected color map
    let color_map = get_color_map(&color_bar_config.color_map_name);

    // Generate color data
    let colors: Vec<[f32; 4]> = scalars
        .iter()
        .map(|&scalar| {
            // Normalize scalar value to [0, 1] range
            let normalized = if range_max > range_min {
                ((scalar - range_min) / (range_max - range_min)).clamp(0.0, 1.0)
            } else {
                0.5 // Use middle value if range is 0
            };

            // Get color using color map
            color_map.get_interpolated_color(normalized)
        })
        .collect();

    // Update mesh vertex color attribute
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
}

/// Step 1: Trigger import of frame 0 as single file
fn trigger_first_frame_import(
    mut time_series_asset: ResMut<TimeSeriesAsset>,
    mut events: EventWriter<crate::ui::events::LoadModelEvent>,
) {
    // If step 1 is ready
    if time_series_asset.is_step1_ready && !time_series_asset.is_step1_complete {
        if let Some(first_file_path) = &time_series_asset.pending_first_file {
            println!(
                "Step 1: Triggering single file import for frame 0: {}",
                first_file_path.display()
            );

            // Send single file import event
            events.send(crate::ui::events::LoadModelEvent(first_file_path.clone()));

            // Mark step 1 as started processing
            time_series_asset.is_step1_ready = false;
            println!("Step 1: Single file import event sent, waiting for completion...");
        }
    }
}

/// Step 1 completion detection: Detect if single file import is complete
fn detect_step1_completion(
    mut time_series_asset: ResMut<TimeSeriesAsset>,
    current_model: Res<crate::ui::CurrentModelData>,
    query: Query<Entity, With<crate::ui::UserModelMesh>>,
) {
    // If step 1 is not complete yet, check if it's already completed
    if !time_series_asset.is_step1_complete {
        // Check if there are model entities and geometry data
        let has_model_entity = !query.is_empty();
        let has_geometry_data = current_model.geometry.is_some();

        if has_model_entity && has_geometry_data {
            println!("Step 1 completed: Static model (frame 0) successfully imported");
            time_series_asset.is_step1_complete = true;

            // Get geometry information
            if let Some(ref geometry) = current_model.geometry {
                time_series_asset.vertices = geometry
                    .vertices
                    .iter()
                    .map(|v| Vec3::new(v[0], v[1], v[2]))
                    .collect();
                time_series_asset.indices = geometry.indices.clone();

                // println!(
                //     "Step 1: Extracted {} vertices, {} indices from imported model",
                //     time_series_asset.vertices.len(),
                //     time_series_asset.indices.len()
                // );
            }

            // Start step 2: Parse all time series files
            // println!("Starting Step 2: Loading all time series scalar data...");
        }
    }
}

/// Step 2: Parse scalar data from all time series files
fn load_all_time_series_data(mut time_series_asset: ResMut<TimeSeriesAsset>) {
    // If step 1 is complete and step 2 is not complete yet
    if time_series_asset.is_step1_complete && !time_series_asset.is_step2_complete {
        println!(
            "Step 2: Loading scalar data from {} files",
            time_series_asset.all_file_paths.len()
        );

        let mut loaded_count = 0;
        let file_paths = time_series_asset.all_file_paths.clone();

        for (index, file_path) in file_paths.iter().enumerate() {
            if let Ok((_, _, scalars)) = load_full_mesh_data(file_path) {
                time_series_asset.time_steps.push(TimeStepData {
                    scalars,
                    time_step: index,
                    file_path: file_path.clone(),
                });
                loaded_count += 1;
            } else {
                eprintln!("Failed to load scalar data from: {}", file_path.display());
            }
        }

        time_series_asset.is_step2_complete = true;
        time_series_asset.is_loaded = true;

        println!(
            "Step 2 completed: Loaded scalar data from {}/{} files",
            loaded_count,
            file_paths.len()
        );
        println!(
            "Time series fully loaded: {} time steps available",
            time_series_asset.time_steps.len()
        );
    }
}

/// Load complete data from file
fn load_full_mesh_data(
    path: &PathBuf,
) -> Result<(Vec<Vec3>, Vec<u32>, Vec<f32>), Box<dyn std::error::Error>> {
    println!("Loading full mesh data from: {}", path.display());
    let vtk = vtkio::Vtk::import(path)?;

    let geometry = match &vtk.data {
        vtkio::model::DataSet::UnstructuredGrid { pieces, .. } => {
            let extractor = crate::mesh::vtk::UnstructuredGridExtractor;
            extractor.process_legacy(pieces.clone())?
        }
        _ => {
            return Err("Only UnstructuredGrid format is supported".into());
        }
    };

    // Extract vertices
    let vertices: Vec<Vec3> = geometry
        .vertices
        .iter()
        .map(|v| Vec3::new(v[0], v[1], v[2]))
        .collect();

    let indices = geometry.indices.clone();

    // Extract scalar data
    let scalars = if let Some(attributes) = &geometry.attributes {
        attributes
            .iter()
            .find_map(|((_, location), attr)| match attr {
                crate::mesh::vtk::AttributeType::Scalar { data, .. } => match location {
                    crate::mesh::vtk::AttributeLocation::Point => Some(data.clone()),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or_else(|| {
                println!("No scalar data found, using default values");
                vec![0.0; vertices.len()]
            })
    } else {
        println!("No attributes found, using default scalar values");
        vec![0.0; vertices.len()]
    };

    println!(
        "Extracted: {} vertices, {} indices, {} scalars",
        vertices.len(),
        indices.len(),
        scalars.len()
    );

    Ok((vertices, indices, scalars))
}
