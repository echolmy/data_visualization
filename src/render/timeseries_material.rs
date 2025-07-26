use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::renderer::{RenderDevice, RenderQueue};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[uuid = "c97d1e2b-5a1c-4f6b-9d2f-1a2b3c4d5e6f"]
pub struct TimeSeriesMaterial {
    /// 当前帧索引 (0..n_frames-1)
    #[uniform(0)]
    pub frame_index: u32,
    /// 顶点数
    #[uniform(1)] 
    pub n_vertices: u32,
    /// 总帧数
    #[uniform(2)]
    pub n_frames: u32,
    /// storage buffer 里存了 n_frames*n_vertices 个 f32
    #[buffer(3)]
    pub data: Handle<wgpu::Buffer>,
}

impl Material for TimeSeriesMaterial {
    fn fragment_shader() -> ShaderRef { "shaders/timeseries.wgsl".into() }
    fn vertex_shader() -> ShaderRef { "shaders/timeseries.wgsl".into() }
}

/// Storage Buffer 资源包装器
#[derive(Resource)]
pub struct ColorTimeSeriesBuffer(pub wgpu::Buffer);

pub fn setup_color_buffer(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) -> ColorTimeSeriesBuffer {
    let buf = render_device.create_buffer_with_data(&wgpu::util::BufferInitDescriptor {
        label: Some("ColorTimeSeries"),
        contents: bytemuck::cast_slice(&all_data),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });
    ColorTimeSeriesBuffer(buf)
}

pub fn animate(
    time: Res<Time>,
    mut mats: ResMut<Assets<TimeSeriesMaterial>>,
    query: Query<&Handle<TimeSeriesMaterial>>,
) {
    let cur_frame = ((time.elapsed_seconds() * fps as f32) as u32) % n_frames;
    for mat_handle in &query {
        if let Some(mat) = mats.get_mut(mat_handle) {
            mat.frame_index = cur_frame;
        }
    }
}
