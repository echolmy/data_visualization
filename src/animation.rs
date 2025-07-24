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
}

/// 时间序列动画事件
#[derive(Event)]
pub enum TimeSeriesEvent {
    LoadSeries(Vec<PathBuf>), // 加载时间序列文件
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
                                                // 未来添加：动画播放系统
                                                // update_animation_timer,
                                                // update_mesh_colors,
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
        }
    }
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
