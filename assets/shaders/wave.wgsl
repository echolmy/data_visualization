#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings::globals

struct WaveUniformData {
    amplitude: f32,
    phase: f32,
    wave_vector_x: f32,
    wave_vector_y: f32,
    omega: f32,
    time: f32,
    base_color: vec3<f32>,
    _padding: f32,
};

@group(2) @binding(0) var<uniform> material: WaveUniformData;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) wave_height: f32,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    // Get world transformation matrix
    let world_from_local = get_world_from_local(vertex.instance_index);
    
    // Calculate wave displacement
    let time = material.time;
    let k = vec2<f32>(material.wave_vector_x, material.wave_vector_y);
    let phase_term = material.phase + dot(k, vertex.position.xz) - material.omega * time;
    let wave_height = material.amplitude * cos(phase_term);
    
    // Apply wave displacement to Y-axis - directly set height (base mesh Y=0)
    var displaced_position = vertex.position;
    displaced_position.y = wave_height;
    
    // Transform to world space
    let world_position = world_from_local * vec4<f32>(displaced_position, 1.0);
    
    // Transform to clip space
    out.clip_position = mesh_position_local_to_clip(
        world_from_local,
        vec4<f32>(displaced_position, 1.0),
    );
    
    out.world_position = world_position;
    out.uv = vertex.uv;
    out.wave_height = wave_height;
    
    return out;
}

struct FragmentInput {
    @location(0) world_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) wave_height: f32,
};

@fragment
fn fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    // Create color variation based on wave height
    let height_factor = (input.wave_height / material.amplitude + 1.0) * 0.5; // Normalize to 0-1
    
    let valley_color = material.base_color * 0.2;                    
    let peak_color = vec3<f32>(1.0) - material.base_color * 0.5;     
    
    let final_color = mix(valley_color, peak_color, height_factor);
    
    return vec4<f32>(final_color, 1.0);
} 