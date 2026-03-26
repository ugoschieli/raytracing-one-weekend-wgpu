@group(0) @binding(0) var tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var accum_tex: texture_storage_2d<rgba32float, read_write>;

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
    is_hdr: u32,
    pad1: u32,
    camera: CameraUniforms,
}
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

struct Material {
    mat_type: MaterialType,
    fuzz: f32, // only useful for metal
    refraction_index: f32, // only useful for dielectrics
    _pad: u32,
    albedo: vec3<f32>,
    _pad1: u32,
}

struct Cube {
    mat: Material,
    center: vec3<f32>,
    size: f32,
}
@group(0) @binding(3) var<storage, read> world: array<Cube>;
@group(0) @binding(4) var gbuffer_albedo: texture_2d<f32>;
@group(0) @binding(5) var gbuffer_normal: texture_2d<f32>;
@group(0) @binding(6) var gbuffer_depth: texture_depth_2d;
@group(0) @binding(7) var gbuffer_material: texture_2d<f32>;
@group(0) @binding(8) var<storage, read> lights: array<u32>;
@group(0) @binding(9) var skybox_tex: texture_2d<f32>;
@group(0) @binding(10) var skybox_sampler: sampler;


const PI: f32 = radians(180.0);
const INFINITY: f32 = 100000000000.0;
const SAMPLE_PER_PIXEL: u32 = 1;
const MAX_DEPTH = 50;
const PIXEL_SAMPLE_SCALE: f32 = 1.0 / f32(SAMPLE_PER_PIXEL); 
fn sample_skybox(dir: vec3<f32>) -> vec3<f32> {
    let unit_dir = normalize(dir);
    let phi = atan2(unit_dir.z, unit_dir.x);
    let theta = acos(unit_dir.y);
    let u = 0.5 + phi / (2.0 * PI);
    let v = theta / PI;
    return textureSampleLevel(skybox_tex, skybox_sampler, vec2<f32>(u, v), 0.0).rgb;
}

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
        let unit_direction = normalize(r_in.dir);
        var reflected = reflect(unit_direction, rec.normal);
        reflected = normalize(reflected) + (rec.mat.fuzz * random_unit_vector(rand));
        (*scattered) = Ray(rec.p, reflected);
        (*attenuation) = rec.mat.albedo;
        return (dot((*scattered).dir, rec.normal) > 0.0);
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
const MAT_EMISSIVE: MaterialType = 3;

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

fn aces_tonemap(v: vec3<f32>) -> vec3<f32> {
    // We use a Hue-Preserving Tonemap.
    // Instead of applying ACES to each channel (which turns bright orange into white/yellow),
    // we apply ACES to the *brightest* channel, and scale the others proportionally.
    let max_comp = max(v.r, max(v.g, v.b));
    
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    let mapped_max = clamp((max_comp * (a * max_comp + b)) / (max_comp * (c * max_comp + d) + e), 0.0, 1.0);
    
    return v * (mapped_max / max(max_comp, 1e-5));
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

fn hit_cube(cube: Cube, r: Ray, ray_tmin: f32, ray_tmax: f32, rec: ptr<function, HitRecord>) -> bool {
    let min_bound = cube.center - vec3<f32>(cube.size, cube.size, cube.size);
    let max_bound = cube.center + vec3<f32>(cube.size, cube.size, cube.size);

    // Provide a protected dir that scales zeros to epsilon avoiding 1/0.0 = NaN anomalies.
    var dir_sign = sign(r.dir);
    dir_sign = select(dir_sign, vec3<f32>(1.0), dir_sign == vec3<f32>(0.0));
    let safe_dir = max(abs(r.dir), vec3<f32>(1e-8)) * dir_sign;
    let invD = 1.0 / safe_dir;

    let t0 = (min_bound - r.orig) * invD;
    let t1 = (max_bound - r.orig) * invD;

    let tmin_vec = min(t0, t1);
    let tmax_vec = max(t0, t1);

    let tmin_val = max(tmin_vec.x, max(tmin_vec.y, tmin_vec.z));
    let tmax_val = min(tmax_vec.x, min(tmax_vec.y, tmax_vec.z));

    if (tmin_val >= tmax_val) {
        return false;
    }

    var t = tmin_val;
    if (t < ray_tmin) {
        t = tmax_val;
        if (t < ray_tmin) {
            return false;
        }
    }
    if (t > ray_tmax) {
        return false;
    }

    (*rec).t = t;
    (*rec).p = ray_at(r, (*rec).t);
    (*rec).mat = cube.mat;
    
    let p_local = (*rec).p - cube.center;
    let p_abs = abs(p_local);
    let max_c = max(p_abs.x, max(p_abs.y, p_abs.z));
    
    var outward_normal = vec3<f32>(0.0, 0.0, 0.0);
    if (p_abs.x >= max_c - 1e-5) {
        outward_normal.x = sign(p_local.x);
    } else if (p_abs.y >= max_c - 1e-5) {
        outward_normal.y = sign(p_local.y);
    } else {
        outward_normal.z = sign(p_local.z);
    }

    set_face_normal(rec, r, outward_normal);
    if (!(*rec).front_face && cube.mat.mat_type != MAT_DIELECTRIC) {
        return false; // Eliminate Lambertian/Metal shadow acne efficiently
    }

    return true;
}

fn hit_shadow(r_in: Ray, max_distance: f32, target_cube_idx: u32) -> bool {
    let world_size: u32 = arrayLength(&world);
    for (var i: u32 = 0; i < world_size; i++) {
        if (i == target_cube_idx) {
            continue;
        }

        let cube = world[i];
        let min_bound = cube.center - vec3<f32>(cube.size, cube.size, cube.size);
        let max_bound = cube.center + vec3<f32>(cube.size, cube.size, cube.size);

        var dir_sign = sign(r_in.dir);
        dir_sign = select(dir_sign, vec3<f32>(1.0), dir_sign == vec3<f32>(0.0));
        let safe_dir = max(abs(r_in.dir), vec3<f32>(1e-8)) * dir_sign;
        let invD = 1.0 / safe_dir;

        let t0 = (min_bound - r_in.orig) * invD;
        let t1 = (max_bound - r_in.orig) * invD;

        let tmin_vec = min(t0, t1);
        let tmax_vec = max(t0, t1);

        let tmin_val = max(tmin_vec.x, max(tmin_vec.y, tmin_vec.z));
        let tmax_val = min(tmax_vec.x, min(tmax_vec.y, tmax_vec.z));

        if (tmin_val >= tmax_val) {
            continue;
        }

        var t = tmin_val;
        if (t < 0.001) {
            t = tmax_val;
            if (t < 0.001) {
                continue;
            }
        }
        if (t < max_distance) {
            return true;
        }
    }
    return false;
}

fn compute_nee(rand: ptr<function, Rand>, rec: HitRecord) -> vec3<f32> {
    var nee_color = vec3<f32>(0.0);
    let num_lights = arrayLength(&lights);
    if (num_lights > 0u && rec.mat.mat_type == MAT_LAMBERTIAN) { 
        let light_index = u32(rand_f32_min_max(rand, 0.0, f32(num_lights) - 0.0001));
        let light_world_idx = lights[light_index];
        let light = world[light_world_idx];
                    
        var light_p = vec3<f32>();
        var light_n = vec3<f32>();
        var light_pdf = 0.0;
        sample_cube_light(rand, light, rec.p, &light_p, &light_n, &light_pdf);
                    
        let light_dir = light_p - rec.p;
        let distance_to_light = length(light_dir);
        let L = light_dir / distance_to_light; 
        let N = rec.normal;
                    
        let cos_theta_recv = dot(N, L);
        let cos_theta_light = dot(light_n, -L);
                    
        if (cos_theta_recv > 0.0 && cos_theta_light > 0.0) {
            let shadow_ray = Ray(rec.p + N * 0.005, L);
            if (!hit_shadow(shadow_ray, distance_to_light, light_world_idx)) {
                let brdf = rec.mat.albedo / PI;
                let G = (cos_theta_recv * cos_theta_light) / (distance_to_light * distance_to_light);
                let pdf_light = light_pdf * (1.0 / f32(num_lights));
                            
                nee_color = (light.mat.albedo * brdf * G) / pdf_light;
            }
        }
    }
    return nee_color;
}

fn sample_cube_light(rand: ptr<function, Rand>, light: Cube, rec_p: vec3<f32>, light_p: ptr<function, vec3<f32>>, light_normal: ptr<function, vec3<f32>>, pdf: ptr<function, f32>) {
    let s = light.size;
    let relative_p = rec_p - light.center;
    
    var weights: array<f32, 6>;
    var total_weight: f32 = 0.0;
    
    // Weight each face by its distance to the receiver along its normal.
    // This perfectly biases the random choice towards faces that point heavily towards the receiver!
    weights[0] = max(0.0, relative_p.x - s); // Right
    weights[1] = max(0.0, -relative_p.x - s); // Left
    weights[2] = max(0.0, relative_p.y - s); // Top
    weights[3] = max(0.0, -relative_p.y - s); // Bottom
    weights[4] = max(0.0, relative_p.z - s); // Front
    weights[5] = max(0.0, -relative_p.z - s); // Back
    
    for (var i: u32 = 0; i < 6u; i++) {
        total_weight += weights[i];
    }
    
    if (total_weight < 1e-5) {
        for (var i: u32 = 0; i < 6u; i++) { weights[i] = 1.0; }
        total_weight = 6.0;
    }
    
    let r = rand_f32_min_max(rand, 0.0, total_weight - 1e-5);
    var cumul: f32 = 0.0;
    var face_idx: u32 = 5u;
    for (var i: u32 = 0; i < 6u; i++) {
        cumul += weights[i];
        if (r < cumul) {
            face_idx = i;
            break;
        }
    }
    
    let face_prob = weights[face_idx] / total_weight;
    
    let u = rand_f32_min_max(rand, -s, s);
    let v = rand_f32_min_max(rand, -s, s);
    
    var p = vec3<f32>(0.0);
    var n = vec3<f32>(0.0);
    
    switch face_idx {
        case 0u: { p = vec3<f32>(s, u, v); n = vec3<f32>(1.0, 0.0, 0.0); } // Right
        case 1u: { p = vec3<f32>(-s, u, v); n = vec3<f32>(-1.0, 0.0, 0.0); } // Left
        case 2u: { p = vec3<f32>(u, s, v); n = vec3<f32>(0.0, 1.0, 0.0); } // Top
        case 3u: { p = vec3<f32>(u, -s, v); n = vec3<f32>(0.0, -1.0, 0.0); } // Bottom
        case 4u: { p = vec3<f32>(u, v, s); n = vec3<f32>(0.0, 0.0, 1.0); } // Front
        case 5u: { p = vec3<f32>(u, v, -s); n = vec3<f32>(0.0, 0.0, -1.0); } // Back
        default: {}
    }
    
    (*light_p) = light.center + p;
    (*light_normal) = n;
    
    let area_per_face = 4.0 * s * s;
    (*pdf) = face_prob * (1.0 / area_per_face);
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

fn ray_color_bounce(rand: ptr<function, Rand>, base_ray: Ray, initial_attenuation: vec3<f32>, last_bounce_was_specular: bool) -> vec3<f32> {
    let world_size: u32 = arrayLength(&world);
    var stop = false;
    var depth = 1;

    var r = base_ray;
    var cur_attenuation = initial_attenuation;
    var emitted_color = vec3<f32>(0.0);
    var is_specular_bounce = last_bounce_was_specular;

    while (!stop && depth <= MAX_DEPTH) {
        var rec = HitRecord();
        var temp_rec = HitRecord();
        var hit_anything = false;
        var closest_so_far = INFINITY;

        for (var i: u32 = 0; i < world_size; i++) {
            if (hit_cube(world[i], r, 0.001, closest_so_far, &temp_rec)) {
                hit_anything = true;
                closest_so_far = temp_rec.t;
                rec = temp_rec;
            }       
        }

        if (hit_anything) {
            if (rec.mat.mat_type == MAT_EMISSIVE) {
                // If the previous bounce didn't use NEE (it was specular), we must add the light's emission here
                // to avoid entirely missing the light. If it used NEE (Lambertian), we ignore it to prevent double counting.
                if (is_specular_bounce) {
                    emitted_color += cur_attenuation * rec.mat.albedo;
                }
                cur_attenuation = vec3<f32>(0.0);
                stop = true;
            } else {
                emitted_color += cur_attenuation * compute_nee(rand, rec);

                if (depth == MAX_DEPTH) {
                    cur_attenuation = vec3<f32>();
                }
                var scattered = Ray();
                var attenuation = vec3<f32>();
                let current_mat_type = rec.mat.mat_type;
                if (mat_scatter(rand, r, rec, &attenuation, &scattered)) {
                    let is_outward = (dot(scattered.dir, rec.normal) > 0.0);
                    let offset_normal = select(-rec.normal, rec.normal, is_outward);
                    scattered.orig = rec.p + offset_normal * 0.005;

                    r = scattered;
                    cur_attenuation *= attenuation;
                    is_specular_bounce = (current_mat_type != MAT_LAMBERTIAN);
                } else {
                    cur_attenuation = vec3<f32>();
                    stop = true;
                }
            }
        } else {
            emitted_color += cur_attenuation * sample_skybox(r.dir);
            stop = true;
        }
        depth += 1;
    }

    return emitted_color;
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
    // Combine x and y to ensure every pixel has a unique starting state, avoiding collisions.
    let pixel_index = global_id.y * dimensions.x + global_id.x;
    var seed = pcg_hash(pixel_index);
    seed = pcg_hash(seed ^ uniforms.frame);
    var rand = init_rand(seed);
    
    let tex_coords = vec2<i32>(global_id.xy);
    let depth = textureLoad(gbuffer_depth, tex_coords, 0);

    // Compute unjittered primary ray dir for incident vector
    let pixel_sample = uniforms.camera.pixel00_loc
        + (f32(global_id.x) * uniforms.camera.pixel_delta_u)
        + (f32(global_id.y) * uniforms.camera.pixel_delta_v);
    let primary_ray = Ray(uniforms.camera.center, pixel_sample - uniforms.camera.center);

    var frame_color = vec3<f32>();

    if (depth >= 1.0) {
        frame_color = sample_skybox(primary_ray.dir) * f32(SAMPLE_PER_PIXEL);
    } else {
        let ndc_x = (f32(global_id.x) + 0.5) / f32(dimensions.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (f32(global_id.y) + 0.5) / f32(dimensions.y) * 2.0;

        let ndc_pos = vec4<f32>(ndc_x, ndc_y, depth, 1.0);
        let world_pos_homo = uniforms.camera.inv_view_proj * ndc_pos;
        let world_pos = world_pos_homo.xyz / world_pos_homo.w;

        let normal = textureLoad(gbuffer_normal, tex_coords, 0).xyz;
        let albedo = textureLoad(gbuffer_albedo, tex_coords, 0).rgb;
        let material = textureLoad(gbuffer_material, tex_coords, 0);

        var rec = HitRecord();
        rec.mat = Material(u32(material.x + 0.5), material.y, material.z, 0u, albedo, 0u); 
        rec.normal = normalize(normal); // Trust the specific G-Buffer rasterizer rendering normal
        rec.front_face = true;
        
        let t_dist = length(world_pos - primary_ray.orig);
        rec.p = world_pos;

        for (var i: u32 = 0; i < SAMPLE_PER_PIXEL; i++) {
            var scattered = Ray();
            var attenuation = vec3<f32>();
            if (rec.mat.mat_type == MAT_EMISSIVE) {
                frame_color += rec.mat.albedo;
            } else {
                frame_color += compute_nee(&rand, rec);

                if (mat_scatter(&rand, primary_ray, rec, &attenuation, &scattered)) {
                    let is_outward = (dot(scattered.dir, rec.normal) > 0.0);
                    let offset_normal = select(-rec.normal, rec.normal, is_outward);
                    let epsilon = max(0.005, t_dist * 0.001);
                    scattered.orig = rec.p + offset_normal * epsilon;

                    let is_specular = (rec.mat.mat_type != MAT_LAMBERTIAN);
                    frame_color += ray_color_bounce(&rand, scattered, attenuation, is_specular);
                }
            }
        }
    }

    frame_color *= PIXEL_SAMPLE_SCALE;

    var accumulated_color: vec3<f32>;

    if (uniforms.frame == 0u) {
        accumulated_color = frame_color;
    } else {
        let prev_color = textureLoad(accum_tex, tex_coords).rgb;
        accumulated_color = prev_color + frame_color;
    }

    textureStore(accum_tex, tex_coords, vec4<f32>(accumulated_color, 1.0));

    var final_color = accumulated_color / f32(uniforms.frame + 1u);

    // Hardcoded exposure for HDR tonemapping. Tweaking this adjusts the camera's brightness!
    let exposure = 1.0; 
    
    if (uniforms.is_hdr == 0u) {
        // Apply ACES HDR tonemapping to bring extreme brightness back into 0-1 range
        final_color = aces_tonemap(final_color * exposure);

        final_color.r = clamp(linear_to_gamma(final_color.r), 0.0, 0.999);
        final_color.g = clamp(linear_to_gamma(final_color.g), 0.0, 0.999);
        final_color.b = clamp(linear_to_gamma(final_color.b), 0.0, 0.999);
    }

    textureStore(tex, tex_coords, vec4<f32>(final_color, 1.0));
}
