// Bind group 1: [uniforms(0..2), storage_buffer(3)]
@group(1) @binding(3) var<storage, read> color_data: array<f32>;

struct VertexOutput {
    @builtin(position) Position : vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @location(0) in_pos: vec3<f32>,
    @uniform(0) frame_index: u32,
    @uniform(1) n_vertices: u32,
    @uniform(2) n_frames: u32,
) -> VertexOutput {
    var out: VertexOutput;
    
    // 标准 MVP
    out.Position = /* ... */;
    
    // 从大数组中读: frame_index * n_vertices + vi
    let idx = frame_index * n_vertices + vi;
    let u = color_data[idx];
    
    // 简单灰度映射，也可以做任何颜色 colormap
    out.color = vec4<f32>(u, u, u, 1.0);
    
    return out;
}

@fragment  
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
