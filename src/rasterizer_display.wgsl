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

@group(0) @binding(0) var t_albedo: texture_2d<f32>;
@group(0) @binding(1) var s_sampler: sampler;
@group(0) @binding(2) var t_normal: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(t_albedo, s_sampler, in.uv);
    let normal = textureSample(t_normal, s_sampler, in.uv).xyz;

    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let ndotl = max(dot(normal, light_dir), 0.1); // Add ambient

    // Background check
    if (length(normal) < 0.1) {
        return albedo;
    }

    return vec4<f32>(albedo.rgb * ndotl, 1.0);
}
