//! 使用 Procedural Modelling 生成复数平面波网格
//!
//! 这个模块提供了基于半边数据结构的波形网格生成功能，
//! 支持复杂的几何操作和丰富的属性管理。

use bevy::prelude::{render::render_asset::RenderAssetUsages, *};
use procedural_modelling::{extensions::bevy::*, mesh::MeshBuilder, prelude::*};
use std::f32::consts::PI;

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

fn wave_vertex(x: f32, y: f32, wave: &PlaneWave) -> BevyVertexPayload3d {
    let height = wave.get_real_part(x, y); // calculate height of wave
    BevyVertexPayload3d::from_pos(Vec3::new(x, height, y))
}

pub fn generate_wave_surface(
    wave: &PlaneWave,
    width: f32,
    depth: f32,
    width_resolution: usize,
    depth_resolution: usize,
) -> BevyMesh3d {
    let mut mesh = BevyMesh3d::new();

    let step_x = width / (width_resolution - 1) as f32;
    let step_y = depth / (depth_resolution - 1) as f32;

    // 首行
    let mut current_row: Vec<_> = (0..width_resolution)
        .map(|i| {
            let x = i as f32 * step_x;
            let y = 0.0;
            wave_vertex(x, y, wave)
        })
        .collect();

    let mut current_edge = mesh.insert_line(current_row.iter().cloned());

    for j in 1..depth_resolution {
        let y = j as f32 * step_y;

        let next_row: Vec<_> = (0..width_resolution)
            .map(|i| {
                let x = i as f32 * step_x;
                wave_vertex(x, y, wave)
            })
            .collect();

        current_edge = mesh.loft_polygon(
            current_edge,
            width_resolution,
            width_resolution,
            next_row.iter().cloned(),
        );

        current_row = next_row;
    }

    mesh
}
