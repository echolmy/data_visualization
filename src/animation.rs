// 时间序列动画系统
use crate::mesh::vtk::VtkMeshExtractor;
use bevy::prelude::*;
use std::path::PathBuf;

/// 时间序列中每一帧的标量数据
#[derive(Clone, Debug)]
pub struct TimeStepData {
    pub scalars: Vec<f32>,  // 该时间步的标量值
    pub time_step: usize,   // 时间步索引
    pub file_path: PathBuf, // 源文件路径
}

/// 时间序列资产 - 包含静态网格和所有时间步的标量数据
#[derive(Resource)]
pub struct TimeSeriesAsset {
    // 第一步：静态模型导入相关
    pub pending_first_file: Option<PathBuf>, // 等待导入的第一个文件
    pub is_step1_ready: bool,                // 第一步是否准备就绪
    pub is_step1_complete: bool,             // 第一步是否完成

    // 第二步：时间序列数据相关
    pub all_file_paths: Vec<PathBuf>,  // 所有文件路径
    pub time_steps: Vec<TimeStepData>, // 所有时间步的标量数据
    pub is_step2_complete: bool,       // 第二步是否完成

    // 静态网格数据（由第一步的单个文件导入系统创建）
    pub mesh_entity: Option<Entity>, // 网格实体

    // 几何数据（从CurrentModelData获取）
    pub vertices: Vec<Vec3>, // 静态顶点位置
    pub indices: Vec<u32>,   // 静态索引

    // 动画控制
    pub current_time_step: usize, // 当前时间步
    pub is_loaded: bool,          // 是否完全加载完成
    pub is_playing: bool,         // 是否正在播放
    pub fps: f32,                 // 播放帧率
    pub timer: Timer,             // 播放计时器
    pub loop_animation: bool,     // 是否循环播放
    pub colors_need_update: bool, // 标记颜色是否需要更新
}

/// 时间序列动画事件
#[derive(Event)]
pub enum TimeSeriesEvent {
    LoadSeries(Vec<PathBuf>), // 加载时间序列文件
    // 第二步：动画控制事件
    Play,               // 播放动画
    Pause,              // 暂停动画
    Stop,               // 停止动画
    SetTimeStep(usize), // 设置到指定时间步
    NextTimeStep,       // 下一时间步
    PrevTimeStep,       // 上一时间步
    SetFPS(f32),        // 设置播放帧率
    ToggleLoop,         // 切换循环播放
}

impl Default for TimeSeriesAsset {
    fn default() -> Self {
        Self {
            // 第一步相关
            pending_first_file: None,
            is_step1_ready: false,
            is_step1_complete: false,

            // 第二步相关
            all_file_paths: Vec::new(),
            time_steps: Vec::new(),
            is_step2_complete: false,

            // 静态网格数据
            mesh_entity: None,
            vertices: Vec::new(),
            indices: Vec::new(),

            // 动画控制
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
    /// 开始加载时间序列 - 第一步：导入第0帧作为静态状态（完全使用单个文件导入逻辑）
    pub fn start_loading(&mut self, file_paths: Vec<PathBuf>) {
        println!(
            "Loading time series - Step 1: Import frame 0 as static model: {} files available",
            file_paths.len()
        );

        // 重置状态
        *self = Self::default();

        // 存储所有文件路径，供第二步使用
        self.all_file_paths = file_paths.clone();

        // 第一步：只处理第一个文件，完全使用单个文件导入逻辑
        if let Some(first_file) = file_paths.first() {
            println!(
                "Step 1: Loading frame 0 as static model: {}",
                first_file.display()
            );

            // 标记准备导入第一个文件（让 UI 系统处理）
            self.pending_first_file = Some(first_file.clone());
            self.is_step1_ready = true;
        }
    }

    /// 获取当前时间步数据（UI显示需要）
    pub fn get_current_time_step_data(&self) -> Option<&TimeStepData> {
        self.time_steps.get(self.current_time_step)
    }

    /// 播放动画
    pub fn play(&mut self) {
        if self.is_step2_complete && !self.time_steps.is_empty() {
            self.is_playing = true;
            self.colors_need_update = true; // 开始播放时确保颜色更新
            println!("开始播放时序动画，共{}帧", self.time_steps.len());
        }
    }

    /// 暂停动画
    pub fn pause(&mut self) {
        self.is_playing = false;
        println!("暂停动画在第{}帧", self.current_time_step);
    }

    /// 停止动画并回到第一帧
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_time_step = 0;
        println!("停止动画并回到第0帧");
    }

    /// 设置到指定时间步
    pub fn set_time_step(&mut self, step: usize) {
        if step < self.time_steps.len() && step != self.current_time_step {
            self.current_time_step = step;
            self.colors_need_update = true; // 标记需要更新颜色
            println!("设置到第{}帧", step);
        }
    }

    /// 下一时间步
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
                self.colors_need_update = true; // 标记需要更新颜色
            }
        }
    }

    /// 上一时间步
    pub fn prev_time_step(&mut self) {
        if !self.time_steps.is_empty() {
            let old_step = self.current_time_step;
            if self.current_time_step > 0 {
                self.current_time_step -= 1;
            } else if self.loop_animation {
                self.current_time_step = self.time_steps.len() - 1;
            }

            if old_step != self.current_time_step {
                self.colors_need_update = true; // 标记需要更新颜色
            }
        }
    }

    /// 设置播放帧率
    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps.clamp(0.1, 60.0);
        self.timer = Timer::from_seconds(1.0 / self.fps, TimerMode::Repeating);
        println!("设置播放帧率为{}fps", self.fps);
    }

    /// 获取总时间步数
    pub fn get_total_time_steps(&self) -> usize {
        self.time_steps.len()
    }
}

/// 时间序列动画插件
pub struct TimeSeriesAnimationPlugin;

impl Plugin for TimeSeriesAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeSeriesAsset>()
            .add_event::<TimeSeriesEvent>()
            .add_systems(
                Update,
                (
                    handle_time_series_events,
                    trigger_first_frame_import, // 第一步：触发单个文件导入
                    detect_step1_completion,    // 检测第一步完成
                    load_all_time_series_data,  // 第二步：加载所有时间序列数据
                    // 第二步：动画播放系统
                    update_animation_timer,  // 动画计时器
                    update_animation_colors, // 动画颜色更新
                )
                    .chain(), // 确保系统按顺序执行
            );
    }
}

/// 处理时间序列事件
fn handle_time_series_events(
    mut events: EventReader<TimeSeriesEvent>,
    mut time_series_asset: ResMut<TimeSeriesAsset>,
) {
    for event in events.read() {
        match event {
            TimeSeriesEvent::LoadSeries(file_paths) => {
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
                println!("循环播放: {}", time_series_asset.loop_animation);
            }
        }
    }
}

/// 动画计时器更新系统
fn update_animation_timer(time: Res<Time>, mut time_series_asset: ResMut<TimeSeriesAsset>) {
    if time_series_asset.is_playing && time_series_asset.is_step2_complete {
        time_series_asset.timer.tick(time.delta());
        if time_series_asset.timer.finished() {
            time_series_asset.next_time_step();
        }
    }
}

/// 动画颜色更新系统 - 根据当前时间步更新网格顶点颜色
fn update_animation_colors(
    mut time_series_asset: ResMut<TimeSeriesAsset>,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_query: Query<&Mesh3d, With<crate::ui::UserModelMesh>>,
    color_bar_config: Res<crate::ui::ColorBarConfig>,
) {
    // 只有在时间序列完全加载且需要更新颜色时才处理
    if !time_series_asset.is_step2_complete
        || time_series_asset.time_steps.is_empty()
        || !time_series_asset.colors_need_update
    {
        return;
    }

    // 获取当前时间步的标量数据
    let current_data = match time_series_asset.get_current_time_step_data() {
        Some(data) => data,
        None => return,
    };

    // 查找用户模型网格并更新颜色
    let mesh_count = mesh_query.iter().count();
    if mesh_count > 0 {
        for mesh3d in mesh_query.iter() {
            if let Some(mesh) = meshes.get_mut(&mesh3d.0) {
                // 使用现有的颜色映射函数更新顶点颜色
                apply_scalar_colors_to_mesh(mesh, &current_data.scalars, &color_bar_config);
                println!(
                    "Updated mesh colors for time step {} with {} scalars",
                    time_series_asset.current_time_step,
                    current_data.scalars.len()
                );
            }
        }
        // 清除更新标记
        time_series_asset.colors_need_update = false;
    }
}

/// 应用标量值到网格顶点颜色
fn apply_scalar_colors_to_mesh(
    mesh: &mut Mesh,
    scalars: &[f32],
    color_bar_config: &crate::ui::ColorBarConfig,
) {
    use crate::mesh::color_maps::get_color_map;

    // 获取顶点数量
    let vertex_count = mesh.count_vertices();

    // 确保标量数据数量与顶点数量匹配
    if scalars.len() != vertex_count {
        println!(
            "警告：标量数据数量({})与顶点数量({})不匹配",
            scalars.len(),
            vertex_count
        );
        return;
    }

    // 计算标量值的范围
    let (min_val, max_val) = scalars
        .iter()
        .fold((f32::MAX, f32::MIN), |(min, max), &val| {
            (min.min(val), max.max(val))
        });

    // 使用ColorBarConfig中的范围，如果范围为0则使用自动范围
    let (range_min, range_max) = if color_bar_config.max_value > color_bar_config.min_value {
        (color_bar_config.min_value, color_bar_config.max_value)
    } else {
        (min_val, max_val)
    };

    // 获取当前选择的颜色映射表
    let color_map = get_color_map(&color_bar_config.color_map_name);

    // 生成颜色数据
    let colors: Vec<[f32; 4]> = scalars
        .iter()
        .map(|&scalar| {
            // 将标量值归一化到[0, 1]范围
            let normalized = if range_max > range_min {
                ((scalar - range_min) / (range_max - range_min)).clamp(0.0, 1.0)
            } else {
                0.5 // 如果范围为0，使用中间值
            };

            // 使用颜色映射表获取颜色
            color_map.get_interpolated_color(normalized)
        })
        .collect();

    // 更新网格的顶点颜色属性
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
}

/// 第一步：触发第0帧的单个文件导入（完全使用现有的单个文件导入逻辑）
fn trigger_first_frame_import(
    mut time_series_asset: ResMut<TimeSeriesAsset>,
    mut events: EventWriter<crate::ui::events::LoadModelEvent>,
) {
    // 如果第一步准备就绪且尚未完成
    if time_series_asset.is_step1_ready && !time_series_asset.is_step1_complete {
        if let Some(first_file_path) = &time_series_asset.pending_first_file {
            println!(
                "Step 1: Triggering single file import for frame 0: {}",
                first_file_path.display()
            );

            // 发送单个文件导入事件，完全使用现有的单个文件导入系统
            events.send(crate::ui::events::LoadModelEvent(first_file_path.clone()));

            // 标记第一步已开始处理
            time_series_asset.is_step1_ready = false;
            println!("Step 1: Single file import event sent, waiting for completion...");
        }
    }
}

/// 第一步完成检测：检测单个文件导入是否完成
fn detect_step1_completion(
    mut time_series_asset: ResMut<TimeSeriesAsset>,
    current_model: Res<crate::ui::CurrentModelData>,
    query: Query<Entity, With<crate::ui::UserModelMesh>>,
) {
    // 如果第一步尚未完成，检测是否已经完成
    if !time_series_asset.is_step1_complete {
        // 检查是否有模型实体和几何数据
        let has_model_entity = !query.is_empty();
        let has_geometry_data = current_model.geometry.is_some();

        if has_model_entity && has_geometry_data {
            println!("Step 1 completed: Static model (frame 0) successfully imported");
            time_series_asset.is_step1_complete = true;

            // 从CurrentModelData获取几何信息
            if let Some(ref geometry) = current_model.geometry {
                time_series_asset.vertices = geometry
                    .vertices
                    .iter()
                    .map(|v| Vec3::new(v[0], v[1], v[2]))
                    .collect();
                time_series_asset.indices = geometry.indices.clone();

                println!(
                    "Step 1: Extracted {} vertices, {} indices from imported model",
                    time_series_asset.vertices.len(),
                    time_series_asset.indices.len()
                );
            }

            // 开始第二步：解析所有时间序列文件
            println!("Starting Step 2: Loading all time series scalar data...");
        }
    }
}

/// 第二步：解析所有时间序列文件的标量数据
fn load_all_time_series_data(mut time_series_asset: ResMut<TimeSeriesAsset>) {
    // 如果第一步完成且第二步尚未完成
    if time_series_asset.is_step1_complete && !time_series_asset.is_step2_complete {
        println!(
            "Step 2: Loading scalar data from {} files",
            time_series_asset.all_file_paths.len()
        );

        let mut loaded_count = 0;
        // 克隆文件路径列表以避免借用冲突
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

/// 从VTU文件加载完整数据（网格+标量）
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
            return Err("Only UnstructuredGrid (VTU) format is supported".into());
        }
    };

    // 提取顶点
    let vertices: Vec<Vec3> = geometry
        .vertices
        .iter()
        .map(|v| Vec3::new(v[0], v[1], v[2]))
        .collect();

    let indices = geometry.indices.clone();

    // 提取标量数据（查找第一个点标量属性）
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
