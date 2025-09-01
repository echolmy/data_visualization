// src/mesh/wave_shader.rs
//! Wave material implementation using GPU shaders
//!
//! Uses vertex shaders to compute wave deformation in real-time on GPU,
//! providing high-performance dynamic wave effects.
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

/// Data structure for shader binding
#[derive(Clone, ShaderType)]
pub struct WaveUniformData {
    pub amplitude: f32,
    pub phase: f32,
    pub wave_vector_x: f32,
    pub wave_vector_y: f32,
    pub omega: f32,
    pub time: f32,
    pub base_color: Vec3,
    pub _padding: f32, // Ensure memory alignment
}

/// Wave material structure
///
/// Contains all wave parameters passed to GPU shader
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct WaveMaterial {
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

    // Getter and setter methods
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
                base_color: Vec3::new(0.3, 0.5, 0.8), // Blue color
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

/// Create flat plane mesh for wave rendering
///
/// # Parameters
/// - `width_resolution`: Number of vertices in width direction
/// - `height_resolution`: Number of vertices in height direction
/// - `size`: Physical size of the plane
///
/// # Returns
/// Returns the generated mesh object
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

    // Generate vertices
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

    // Generate triangle indices
    for j in 0..(height_resolution - 1) {
        for i in 0..(width_resolution - 1) {
            let base = (j * width_resolution + i) as u32;

            // First triangle (counter-clockwise)
            indices.push(base);
            indices.push(base + width_resolution as u32);
            indices.push(base + 1);

            // Second triangle (counter-clockwise)
            indices.push(base + 1);
            indices.push(base + width_resolution as u32);
            indices.push(base + width_resolution as u32 + 1);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    // Calculate normals
    mesh.compute_smooth_normals();

    mesh
}

/// Animation system: Update wave material time parameters
///
/// This system updates the time parameters for all wave materials each frame,
/// creating animated wave effects
pub fn animate_wave_shader(time: Res<Time>, mut wave_materials: ResMut<Assets<WaveMaterial>>) {
    let current_time = time.elapsed_secs();

    for (_, material) in wave_materials.iter_mut() {
        material.data.time = current_time;
    }
}
