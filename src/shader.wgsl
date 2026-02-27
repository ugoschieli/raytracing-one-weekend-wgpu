@group(0) @binding(0) var tex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(tex);
    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y) {
        return;
    }
    
    let u = f32(global_id.x) / f32(dimensions.x);
    
    // Gradient from red to green
    let r = 1.0 - u;
    let g = u;
    
    textureStore(tex, vec2<i32>(global_id.xy), vec4<f32>(r, g, 0.0, 1.0));
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv_x = f32((in_vertex_index << 1u) & 2u);
    let uv_y = f32(in_vertex_index & 2u);
    out.uv = vec2<f32>(uv_x, uv_y);
    out.clip_position = vec4<f32>(uv_x * 2.0 - 1.0, 1.0 - uv_y * 2.0, 0.0, 1.0);
    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
}