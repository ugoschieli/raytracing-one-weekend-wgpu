@group(0) @binding(0) var tex: texture_storage_2d<rgba8unorm, write>;

struct Uniforms {
    time: f32,
}
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

const PI: f32 = radians(180.0);
const INFINITY: f32 = 100000000000.0;
const SAMPLE_PER_PIXEL: u32 = 10;
const MAX_DEPTH = 5;
const PIXEL_SAMPLE_SCALE: f32 = 1.0 / f32(SAMPLE_PER_PIXEL); 

struct Rand {
    iter: u32,
    seed: u32,
}

struct Camera {
    image_width: u32,
    image_height: u32,
    center: vec3<f32>,
    pixel00_loc: vec3<f32>,    // Location of pixel 0, 0
    pixel_delta_u: vec3<f32>,  // Offset to pixel to the right
    pixel_delta_v: vec3<f32>,
}

struct Ray {
    orig: vec3<f32>,
    dir: vec3<f32>,
}

struct HitRecord {
    p: vec3<f32>,
    normal: vec3<f32>,
    mat: Material,
    t: f32,
    front_face: bool,
}

struct Sphere {
    center: vec3<f32>,
    radius: f32,
    mat: Material,
}

struct Material {
    mat_type: MaterialType,
    albedo: vec3<f32>,
    fuzz: f32, // only useful for metal
}

alias MaterialType = u32;
const MAT_LAMBERTIAN: u32 = 0;
const MAT_METAL: u32 = 1;

fn pcg_hash(input: u32) -> u32 {
    var state = input * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn init_rand(base_seed: u32) -> Rand {
    return Rand(base_seed, 0);
}

fn rand_f32(r: ptr<function, Rand>) -> f32 {
    // 0xffffffffu is the max value of a u32
    (*r).iter += 1;
    return f32(pcg_hash((*r).seed + (*r).iter)) / f32(0xffffffffu);
}

fn rand_f32_min_max(r: ptr<function, Rand>, min: f32, max: f32) -> f32 {
    return min + (max - min) * rand_f32(r);
}

fn rand_vec3(r: ptr<function, Rand>) -> vec3<f32> {
    return vec3(rand_f32(r), rand_f32(r), rand_f32(r));
}

fn rand_vec3_min_max(r: ptr<function, Rand>, min: f32, max: f32) -> vec3<f32> {
    return vec3(rand_f32_min_max(r, min, max), rand_f32_min_max(r, min, max), rand_f32_min_max(r, min, max));
}

fn random_unit_vector(r: ptr<function, Rand>) -> vec3<f32> {
    while (true) {
        let p = rand_vec3_min_max(r, -1, 1);
        let lensq = dot(p, p);
        if (1e-160 < lensq && lensq <= 1) { 
            return p / sqrt(lensq);
        }
    }

    return vec3();
}

fn random_on_hemisphere(r: ptr<function, Rand>, normal: vec3<f32>) -> vec3<f32> {
    let on_unit_sphere = random_unit_vector(r);
    if (dot(on_unit_sphere, normal) > 0.0) { // In the same hemisphere as the normal
        return on_unit_sphere;
    }
    else {
        return -on_unit_sphere;
    }
}

fn linear_to_gamma(linear_component: f32) -> f32 {
    if (linear_component > 0) {
        return sqrt(linear_component);
    }

    return 0;
}

fn near_zero(e: vec3<f32>) -> bool {
    // Return true if the vector is close to zero in all dimensions.
    let s = 1e-8;
    return (abs(e[0]) < s) && (abs(e[1]) < s) && (abs(e[2]) < s);
}

fn lambert_scatter(mat: Material, rand: ptr<function, Rand>, rec: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>) -> bool {
    var scatter_direction = rec.normal + random_unit_vector(rand);

    // Catch degenerate scatter direction
    if (near_zero(scatter_direction)) {
        scatter_direction = rec.normal;
    }

    (*scattered) = Ray(rec.p, scatter_direction);
    (*attenuation) = mat.albedo;
    return true;
}

fn metal_scatter(mat: Material, rand: ptr<function, Rand>, r_in: Ray, rec: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>) -> bool {
        var reflected = reflect(r_in.dir, rec.normal);
        reflected = normalize(reflected) + (mat.fuzz * random_unit_vector(rand));
        (*scattered) = Ray(rec.p, reflected);
        (*attenuation) = mat.albedo;
        return (dot((*scattered).dir, rec.normal) > 0);
}

fn new_camera(dimensions: vec2<u32>) -> Camera {
    let image_width = dimensions.x;
    let image_height = dimensions.y;

    let center = vec3<f32>(0, 0, 0);

    // Determine viewport dimensions.
    let focal_length = 1.0;
    let viewport_height = 2.0;
    let viewport_width = viewport_height * (f32(image_width) / f32(image_height));

    // Calculate the vectors across the horizontal and down the vertical viewport edges.
    let viewport_u = vec3<f32>(viewport_width, 0, 0);
    let viewport_v = vec3<f32>(0, -viewport_height, 0);

    // Calculate the horizontal and vertical delta vectors from pixel to pixel.
    let pixel_delta_u = viewport_u / f32(image_width);
    let pixel_delta_v = viewport_v / f32(image_height);

    // Calculate the location of the upper left pixel.
    let viewport_upper_left =
        center - vec3(0, 0, focal_length) - viewport_u/2 - viewport_v/2;
    let pixel00_loc = viewport_upper_left + 0.5 * (pixel_delta_u + pixel_delta_v);

    return Camera(image_width, image_height, center, pixel00_loc, pixel_delta_u, pixel_delta_v);
}

fn render(cam: Camera, global_id: vec3<u32>, rand: ptr<function, Rand>) {
    var pixel_color = vec3<f32>();

    for (var i: u32 = 0; i < SAMPLE_PER_PIXEL; i++) {
        let r = get_ray(cam, global_id, rand);
        pixel_color += ray_color(rand, r);
    }

    
    pixel_color *= PIXEL_SAMPLE_SCALE;

    pixel_color.r = linear_to_gamma(pixel_color.r);
    pixel_color.g = linear_to_gamma(pixel_color.g);
    pixel_color.b = linear_to_gamma(pixel_color.b);

    pixel_color.r = clamp(pixel_color.r, 0, 0.999);
    pixel_color.g = clamp(pixel_color.g, 0, 0.999);
    pixel_color.b = clamp(pixel_color.b, 0, 0.999);

    textureStore(tex, vec2<i32>(global_id.xy), vec4<f32>(pixel_color, 1));
}

fn hit_sphere(s: Sphere, r: Ray, ray_tmin: f32, ray_tmax: f32, rec: ptr<function, HitRecord>) -> bool {
    let oc = s.center - r.orig;
    let a = dot(r.dir, r.dir);
    let h = dot(r.dir, oc);
    let c = dot(oc, oc) - s.radius * s.radius;
    let discriminant = h * h - a * c;

    if (discriminant < 0) {
        return false;
    }

    let sqrtd = sqrt(discriminant);

    // Find the nearest root that lies in the acceptable range.
    var root = (h - sqrtd) / a;
    if (root <= ray_tmin || ray_tmax <= root) {
        root = (h + sqrtd) / a;
        if (root <= ray_tmin || ray_tmax <= root) {
            return false;
        }
    }

    (*rec).t = root;
    (*rec).p = ray_at(r, (*rec).t);
    (*rec).mat = s.mat;
    let outward_normal = ((*rec).p - s.center) / s.radius;
    set_face_normal(rec, r, outward_normal);

    return true;
}

fn get_ray(cam: Camera, global_id: vec3<u32>, rand: ptr<function, Rand>) -> Ray {
    // Construct a camera ray originating from the origin and directed at randomly sampled
    // point around the pixel location i, j.
    let offset = sample_square(rand);
    let pixel_sample = cam.pixel00_loc
        + ((f32(global_id.x) + offset.x) * cam.pixel_delta_u)
        + ((f32(global_id.y) + offset.y) * cam.pixel_delta_v);

    let ray_origin = cam.center;
    let ray_direction = pixel_sample - ray_origin;

    return Ray(ray_origin, ray_direction);
}

fn sample_square(rand: ptr<function, Rand>) -> vec3<f32> {
    // Returns the vector to a random point in the [-.5,-.5]-[+.5,+.5] unit square.
    return vec3<f32>(rand_f32(rand) - 0.5, rand_f32(rand) - 0.5, 0);
}

fn ray_at(r: Ray, t: f32) -> vec3<f32> {
    return r.orig + t * r.dir;
}

fn ray_color(rand: ptr<function, Rand>, base_ray: Ray) -> vec3<f32> {
    let world = array(
        Sphere(vec3<f32>(0, -100.5, -1), 100, Material(MAT_LAMBERTIAN, vec3<f32>(0.8, 0.8, 0.0), 0)),
        Sphere(vec3<f32>(0, 0, -1.2), 0.5, Material(MAT_LAMBERTIAN, vec3<f32>(0.1, 0.2, 0.5), 0)),
        Sphere(vec3<f32>(-1, 0, -1), 0.5, Material(MAT_METAL, vec3<f32>(0.8, 0.8, 0.8), 0.3)),
        Sphere(vec3<f32>(1, 0, -1), 0.5, Material(MAT_METAL, vec3<f32>(0.8, 0.6, 0.2), 1.0)),
    );
    let world_size: u32 = 4;
    var stop = false;
    var depth = 0;

    var r = base_ray;

    let unit_direction = normalize(r.dir);
    let a = 0.5 * (unit_direction.y + 1.0);
    var final_color = (1.0 - a) * vec3<f32>(1.0, 1.0, 1.0) + a * vec3<f32>(0.5, 0.7, 1.0);

    while (!stop && depth <= MAX_DEPTH) {
        var rec = HitRecord();
        var temp_rec = HitRecord();
        var hit_anything = false;
        var closest_so_far = INFINITY;

        for (var i: u32 = 0; i < world_size; i++) {
            if (hit_sphere(world[i], r, 0.001, closest_so_far, &temp_rec)) {
                hit_anything = true;
                closest_so_far = temp_rec.t;
                rec = temp_rec;
            }       
        }

        if (hit_anything) {
            if (depth == MAX_DEPTH) {
                final_color = vec3<f32>();
            }
            var not_absorbed: bool;
            var scattered = Ray();
            var attenuation = vec3<f32>();
            switch rec.mat.mat_type {
                case MAT_LAMBERTIAN: {
                                         not_absorbed = lambert_scatter(rec.mat, rand, rec, &attenuation, &scattered);
                                     }
                case MAT_METAL: {
                                    not_absorbed = metal_scatter(rec.mat, rand, r, rec, &attenuation, &scattered);
                                }
                default: {}
            }
            if (not_absorbed) {
                r = scattered;
                final_color *=  attenuation;
            } else {
                final_color = vec3<f32>();
                stop = true;
            }
        } else {
            stop = true;
        }
        depth += 1;
    }

    return final_color;
}

fn set_face_normal(rec: ptr<function, HitRecord>, r: Ray, outward_normal: vec3<f32>) {
    (*rec).front_face = dot(r.dir, outward_normal) < 0;
    if ((*rec).front_face) {
        (*rec).normal = outward_normal;
    } else {
        (*rec).normal = -outward_normal;
    }
}

@compute @workgroup_size(16, 16)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(tex);
    if (global_id.x >= dimensions.x || global_id.y >= dimensions.y) {
        return;
    }

    // A better seed generation that breaks spatial and temporal patterns.
    // By chaining the hashes, we ensure each pixel gets a wildly different 
    // starting seed, preventing overlapping sequences across adjacent pixels.
    var seed = pcg_hash(global_id.x);
    seed = pcg_hash(seed ^ global_id.y);
    seed = pcg_hash(seed ^ bitcast<u32>(uniforms.time));
    var rand = init_rand(seed);

    let camera = new_camera(dimensions);
    render(camera, global_id, &rand);
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

