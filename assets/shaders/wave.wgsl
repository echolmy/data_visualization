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
    
    // 获取世界变换矩阵
    let world_from_local = get_world_from_local(vertex.instance_index);
    
    // 计算波形位移 - 与CPU版本保持一致
    let time = material.time;
    let k = vec2<f32>(material.wave_vector_x, material.wave_vector_y);
    let phase_term = material.phase + dot(k, vertex.position.xz) - material.omega * time;
    let wave_height = material.amplitude * cos(phase_term);
    
    // 应用波形位移到Y轴 - 直接设置高度（基础网格Y=0）
    var displaced_position = vertex.position;
    displaced_position.y = wave_height;  // 直接赋值，因为基础网格Y=0
    
    // 转换到世界空间
    let world_position = world_from_local * vec4<f32>(displaced_position, 1.0);
    
    // 转换到裁剪空间
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
    // 基于波高创建颜色变化
    let height_factor = (input.wave_height / material.amplitude + 1.0) * 0.5; // 归一化到0-1
    
    // 创建非常明显的颜色对比 - 使用基础色和它的补色
    let valley_color = material.base_color * 0.2;                    // 波谷：很暗的基础色
    let peak_color = vec3<f32>(1.0) - material.base_color * 0.5;     // 波峰：近似补色
    
    let final_color = mix(valley_color, peak_color, height_factor);
    
    return vec4<f32>(final_color, 1.0);
} 