//! 使用 Bevy 原生 API 生成复数平面波网格
//!
//! 这个模块提供了基于 bevy Mesh 的波形网格生成功能，
//! 支持复杂的几何操作和丰富的属性管理。

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
// use std::f32::consts::PI;

#[derive(Debug, Clone)]
pub struct PlaneWave {
    // A
    pub amplitude: f32,
    // φ
    pub phase: f32,
    // k
    pub k: Vec2,
    // ω
    pub omega: f32,
    // time
    pub time: f32,
}

impl PlaneWave {
    pub fn new(amplitude: f32, phase: f32, k: Vec2, omega: f32, time: f32) -> Self {
        Self {
            amplitude,
            phase,
            k,
            omega,
            time,
        }
    }

    // get real part of wave
    // A·cos(φ + k.dot(x, y) - ω·t)
    pub fn get_real_part(&self, x: f32, y: f32) -> f32 {
        self.amplitude * (self.k.dot(Vec2::new(x, y)) - self.omega * self.time + self.phase).cos()
    }

    // set direction of wave
    pub fn set_direction(&mut self, x: f32, y: f32) {
        self.k = Vec2::new(x, y).normalize();
    }

    // set time of wave (used for animation)
    pub fn set_time(&mut self, time: f32) {
        self.time = time;
    }
}

impl Default for PlaneWave {
    fn default() -> Self {
        Self {
            amplitude: 1.0,
            phase: 0.0,
            k: Vec2::ZERO,
            omega: 1.0,
            time: 0.0,
        }
    }
}

pub fn generate_wave_surface(
    wave: &PlaneWave,
    width: f32,
    depth: f32,
    width_resolution: usize,
    depth_resolution: usize,
) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    let step_x = width / (width_resolution - 1) as f32;
    let step_z = depth / (depth_resolution - 1) as f32;

    // 生成顶点位置
    let mut positions = Vec::new();
    let mut uvs = Vec::new();

    for j in 0..depth_resolution {
        for i in 0..width_resolution {
            let x = i as f32 * step_x - width * 0.5; // center
            let z = j as f32 * step_z - depth * 0.5; // center
            let y = wave.get_real_part(x, z); // wave height

            positions.push([x, y, z]);

            // UV
            let u = i as f32 / (width_resolution - 1) as f32; // horizontal
            let v = j as f32 / (depth_resolution - 1) as f32; // vertical
            uvs.push([u, v]);
        }
    }

    // 生成索引（三角形）
    // 四边形分解为两个三角形（逆时针顺序）：
    // current ---- current+1
    //   |       /      |
    //   |      /       |
    //   |     /        |
    // next_row ---- next_row+1
    let mut indices = Vec::new();
    for j in 0..(depth_resolution - 1) {
        for i in 0..(width_resolution - 1) {
            let current = j * width_resolution + i;
            let next_row = (j + 1) * width_resolution + i;

            // 第一个三角形（逆时针）
            indices.push(current as u32);
            indices.push(next_row as u32);
            indices.push((current + 1) as u32);

            // 第二个三角形（逆时针）
            indices.push((current + 1) as u32);
            indices.push(next_row as u32);
            indices.push((next_row + 1) as u32);
        }
    }

    // 设置网格属性
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    // 让 Bevy 自动计算平滑法线
    mesh.compute_smooth_normals();

    mesh
}
