struct CameraUniforms {
    center: vec3<f32>,
    pad0: f32,
    pixel00_loc: vec3<f32>,
    pad1: f32,
    pixel_delta_u: vec3<f32>,
    pad2: f32,
    pixel_delta_v: vec3<f32>,
    pad3: f32,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
}

struct Uniforms {
    time: f32,
    frame: u32,
    pad0: u32,
    pad1: u32,
    camera: CameraUniforms,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct Material {
    mat_type: u32,
    roughness: f32,
    refraction_index: f32,
    _pad: u32,
    albedo: vec3<f32>,
    _pad1: u32,
}

struct Cube {
    mat: Material,
    center: vec3<f32>,
    size: f32,
}

@group(0) @binding(1) var<storage, read> world: array<Cube>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(model: VertexInput, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let cube = world[instance_index];
    
    // Calculate world position based on size and center uniform
    let scaled_position = model.position * cube.size;
    let world_position = scaled_position + cube.center;

    var out: VertexOutput;
    out.clip_position = uniforms.camera.view_proj * vec4<f32>(world_position, 1.0);
    
    out.normal = model.normal;
    out.color = vec4<f32>(cube.mat.albedo, 1.0);
    return out;
}

struct GBufferOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> GBufferOutput {
    var out: GBufferOutput;
    // Hand over the interpolated color
    out.albedo = in.color;
    out.normal = vec4<f32>(normalize(in.normal), 1.0);
    return out;
}
