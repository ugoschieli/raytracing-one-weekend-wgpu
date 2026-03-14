@group(0) @binding(0) var tex: texture_storage_2d<rgba8unorm, write>;

struct CameraUniforms {
    center: vec3<f32>,
    pad0: f32,
    pixel00_loc: vec3<f32>,
    pad1: f32,
    pixel_delta_u: vec3<f32>,
    pad2: f32,
    pixel_delta_v: vec3<f32>,
    pad3: f32,
}

struct Uniforms {
    time: f32,
    frame: u32,
    pad0: u32,
    pad1: u32,
    camera: CameraUniforms,
}
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

@group(0) @binding(2) var accum_tex: texture_storage_2d<rgba32float, read_write>;

@group(0) @binding(3) var<storage, read> world: array<Sphere>;

const PI: f32 = radians(180.0);
const INFINITY: f32 = 100000000000.0;
const SAMPLE_PER_PIXEL: u32 = 1;
const MAX_DEPTH = 50;
const PIXEL_SAMPLE_SCALE: f32 = 1.0 / f32(SAMPLE_PER_PIXEL); 

struct Rand {
    iter: u32,
    seed: u32,
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
    mat: Material,
    center: vec3<f32>,
    radius: f32,
}

struct Material {
    mat_type: MaterialType,
    fuzz: f32, // only useful for metal
    refraction_index: f32, // only useful for dielectrics
    _pad: u32,
    albedo: vec3<f32>,
}

fn Lambertian(albedo: vec3<f32>) -> Material {
    return Material(MAT_LAMBERTIAN, 0, 0, 0, albedo);
}

fn Metal(albedo: vec3<f32>, fuzz: f32) -> Material {
    return Material(MAT_METAL, clamp(fuzz, 0.0, 1.0), 0, 0, albedo);
}

fn Dielectric(refraction_index: f32) -> Material {
    return Material(MAT_DIELECTRIC, 0, refraction_index, 0, vec3<f32>());
}

fn mat_scatter(rand: ptr<function, Rand>, r_in: Ray, rec: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>) -> bool {
    switch rec.mat.mat_type {
        case MAT_LAMBERTIAN: {
                                 return lambert_scatter(rand, rec, attenuation, scattered);
                             }
        case MAT_METAL: {
                            return metal_scatter(rand, rec, r_in, attenuation, scattered);
                        }
        case MAT_DIELECTRIC: {
                                 return dielectric_scatter(rand, rec, r_in, attenuation, scattered);
                             }
        default: {
                     return false;
                 }
    }
}

fn lambert_scatter(rand: ptr<function, Rand>, rec: HitRecord, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>) -> bool {
    var scatter_direction = rec.normal + random_unit_vector(rand);

    // Catch degenerate scatter direction
    if (near_zero(scatter_direction)) {
        scatter_direction = rec.normal;
    }

    (*scattered) = Ray(rec.p, scatter_direction);
    (*attenuation) = rec.mat.albedo;
    return true;
}

fn metal_scatter(rand: ptr<function, Rand>, rec: HitRecord, r_in: Ray, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>) -> bool {
        var reflected = reflect(r_in.dir, rec.normal);
        reflected = normalize(reflected) + (rec.mat.fuzz * random_unit_vector(rand));
        (*scattered) = Ray(rec.p, reflected);
        (*attenuation) = rec.mat.albedo;
        return (dot((*scattered).dir, rec.normal) > 0);
}

fn dielectric_scatter(rand: ptr<function, Rand>, rec: HitRecord, r_in: Ray, attenuation: ptr<function, vec3<f32>>, scattered: ptr<function, Ray>) -> bool {
    (*attenuation) = vec3<f32>(1.0, 1.0, 1.0);
    var ri: f32;
    if (rec.front_face) { 
        ri = (1.0 / rec.mat.refraction_index);
    } else { 
        ri = rec.mat.refraction_index;
    };

    let unit_direction = normalize(r_in.dir);
    let cos_theta = min(dot(-unit_direction, rec.normal), 1.0);
    let sin_theta = sqrt(1.0 - cos_theta * cos_theta);

    let cannot_refract = ri * sin_theta > 1.0;
    var direction: vec3<f32>;

    if (cannot_refract || reflectance(cos_theta, ri) > rand_f32(rand)) {
        direction = reflect(unit_direction, rec.normal);
    }
    else
    {
        direction = refract(unit_direction, rec.normal, ri);
    }

    (*scattered) = Ray(rec.p, direction);
    return true;
}

fn reflectance(cosine: f32, refraction_index: f32) -> f32 {
    // Use Schlick's approximation for reflectance.
    var r0 = (1 - refraction_index) / (1 + refraction_index);
    r0 = r0 * r0;
    return r0 + (1 - r0) * pow((1.0 - cosine), 5.0);
}

alias MaterialType = u32;
const MAT_LAMBERTIAN: MaterialType = 0;
const MAT_METAL: MaterialType = 1;
const MAT_DIELECTRIC: MaterialType = 2;

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

    return vec3<f32>();
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

    return 0.0;
}

fn near_zero(e: vec3<f32>) -> bool {
    // Return true if the vector is close to zero in all dimensions.
    let s = 1e-8;
    return (abs(e[0]) < s) && (abs(e[1]) < s) && (abs(e[2]) < s);
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

fn get_ray(cam: CameraUniforms, global_id: vec3<u32>, rand: ptr<function, Rand>) -> Ray {
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
    let world_size: u32 = arrayLength(&world);
    var stop = false;
    var depth = 0;

    var r = base_ray;
    var cur_attenuation = vec3<f32>(1.0, 1.0, 1.0);

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
                cur_attenuation = vec3<f32>();
            }
            var scattered = Ray();
            var attenuation = vec3<f32>();
            if (mat_scatter(rand, r, rec, &attenuation, &scattered)) {
                r = scattered;
                cur_attenuation *= attenuation;
            } else {
                cur_attenuation = vec3<f32>();
                stop = true;
            }
        } else {
            stop = true;
        }
        depth += 1;
    }

    let unit_direction = normalize(r.dir);
    let a = 0.5 * (unit_direction.y + 1.0);
    var final_color = cur_attenuation * ((1.0 - a) * vec3<f32>(1.0, 1.0, 1.0) + a * vec3<f32>(0.5, 0.7, 1.0));

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
    seed = pcg_hash(seed ^ uniforms.frame);
    seed = pcg_hash(seed ^ bitcast<u32>(uniforms.time));
    var rand = init_rand(seed);
    
    var frame_color = vec3<f32>();
    for (var i: u32 = 0; i < SAMPLE_PER_PIXEL; i++) {
        let r = get_ray(uniforms.camera, global_id, &rand);
        frame_color += ray_color(&rand, r);
    }
    frame_color *= PIXEL_SAMPLE_SCALE;

    let tex_coords = vec2<i32>(global_id.xy);
    var accumulated_color: vec3<f32>;

    if (uniforms.frame == 0u) {
        accumulated_color = frame_color;
    } else {
        let prev_color = textureLoad(accum_tex, tex_coords).rgb;
        accumulated_color = prev_color + frame_color;
    }

    textureStore(accum_tex, tex_coords, vec4<f32>(accumulated_color, 1.0));

    var final_color = accumulated_color / f32(uniforms.frame + 1u);

    final_color.r = clamp(linear_to_gamma(final_color.r), 0.0, 0.999);
    final_color.g = clamp(linear_to_gamma(final_color.g), 0.0, 0.999);
    final_color.b = clamp(linear_to_gamma(final_color.b), 0.0, 0.999);

    textureStore(tex, tex_coords, vec4<f32>(final_color, 1.0));
}
