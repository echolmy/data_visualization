// src/mesh/wave_shader.rs
//! GPU Shader 实现的波浪材质
//!
//! 使用顶点着色器在GPU上实时计算波形变形，
//! 提供高性能的动态波浪效果。
#![allow(unused)]

const SHADER_PATH: &str = "shaders/wave.wgsl";
use bevy::{
    math::{Vec2, Vec3},
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        render_resource::{AsBindGroup, ShaderRef, ShaderType},
    },
};

/// 内部uniform数据结构，用于shader绑定
#[derive(Clone, ShaderType)]
pub struct WaveUniformData {
    pub amplitude: f32,
    pub phase: f32,
    pub wave_vector_x: f32,
    pub wave_vector_y: f32,
    pub omega: f32,
    pub time: f32,
    pub base_color: Vec3,
    pub _padding: f32, // 确保内存对齐
}

/// 波浪材质结构体
///
/// 包含所有传递给GPU shader的波浪参数
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct WaveMaterial {
    /// 所有波浪参数打包在一个uniform结构体中
    #[uniform(0)]
    pub data: WaveUniformData,
}

impl WaveMaterial {
    pub fn new(
        amplitude: f32,
        phase: f32,
        wave_vector_x: f32,
        wave_vector_y: f32,
        omega: f32,
        time: f32,
        base_color: Vec3,
    ) -> Self {
        Self {
            data: WaveUniformData {
                amplitude,
                phase,
                wave_vector_x,
                wave_vector_y,
                omega,
                time,
                base_color,
                _padding: 0.0,
            },
        }
    }

    // 便于访问的getter和setter方法
    pub fn amplitude(&self) -> f32 {
        self.data.amplitude
    }
    pub fn set_amplitude(&mut self, amplitude: f32) {
        self.data.amplitude = amplitude;
    }

    pub fn phase(&self) -> f32 {
        self.data.phase
    }
    pub fn set_phase(&mut self, phase: f32) {
        self.data.phase = phase;
    }

    pub fn wave_vector_x(&self) -> f32 {
        self.data.wave_vector_x
    }
    pub fn set_wave_vector_x(&mut self, wave_vector_x: f32) {
        self.data.wave_vector_x = wave_vector_x;
    }

    pub fn wave_vector_y(&self) -> f32 {
        self.data.wave_vector_y
    }
    pub fn set_wave_vector_y(&mut self, wave_vector_y: f32) {
        self.data.wave_vector_y = wave_vector_y;
    }

    pub fn omega(&self) -> f32 {
        self.data.omega
    }
    pub fn set_omega(&mut self, omega: f32) {
        self.data.omega = omega;
    }

    pub fn time(&self) -> f32 {
        self.data.time
    }
    pub fn set_time(&mut self, time: f32) {
        self.data.time = time;
    }

    pub fn base_color(&self) -> Vec3 {
        self.data.base_color
    }
    pub fn set_base_color(&mut self, base_color: Vec3) {
        self.data.base_color = base_color;
    }
}

impl Default for WaveMaterial {
    fn default() -> Self {
        Self {
            data: WaveUniformData {
                amplitude: 1.0,
                phase: 0.0,
                wave_vector_x: 0.5,
                wave_vector_y: 0.3,
                omega: 2.0,
                time: 0.0,
                base_color: Vec3::new(0.3, 0.5, 0.8), // 蓝色
                _padding: 0.0,
            },
        }
    }
}

impl Material for WaveMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}

/// 创建平面网格用于波浪渲染
///
/// # 参数
/// - `width_resolution`: 宽度方向的顶点数量
/// - `height_resolution`: 高度方向的顶点数量
/// - `size`: 平面的物理尺寸
///
/// # 返回值
/// 返回生成的网格对象
pub fn create_flat_plane_mesh(
    width_resolution: usize,
    height_resolution: usize,
    size: Vec2,
) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // 生成顶点
    for j in 0..height_resolution {
        for i in 0..width_resolution {
            let x = (i as f32 / (width_resolution - 1) as f32 - 0.5) * size.x;
            let z = (j as f32 / (height_resolution - 1) as f32 - 0.5) * size.y;

            positions.push([x, 0.0, z]);

            let u = i as f32 / (width_resolution - 1) as f32;
            let v = j as f32 / (height_resolution - 1) as f32;
            uvs.push([u, v]);
        }
    }

    // 生成三角形索引
    for j in 0..(height_resolution - 1) {
        for i in 0..(width_resolution - 1) {
            let base = (j * width_resolution + i) as u32;

            // 第一个三角形 (逆时针)
            indices.push(base);
            indices.push(base + width_resolution as u32);
            indices.push(base + 1);

            // 第二个三角形 (逆时针)
            indices.push(base + 1);
            indices.push(base + width_resolution as u32);
            indices.push(base + width_resolution as u32 + 1);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    // 计算法线
    mesh.compute_smooth_normals();

    mesh
}

/// 动画系统：更新波浪材质的时间参数
///
/// 该系统每帧更新所有波浪材质的时间参数，使波浪产生动画效果
pub fn animate_wave_shader(time: Res<Time>, mut wave_materials: ResMut<Assets<WaveMaterial>>) {
    let current_time = time.elapsed_secs();

    for (_, material) in wave_materials.iter_mut() {
        material.data.time = current_time;
    }
}
