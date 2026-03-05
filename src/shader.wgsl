@group(0) @binding(0) var tex: texture_storage_2d<rgba8unorm, write>;

const PI: f32 = radians(180.0);
const INFINITY: f32 = 100000000000.0;

struct Ray {
    orig: vec3<f32>,
    dir: vec3<f32>,
}

struct HitRecord {
    p: vec3<f32>,
    normal: vec3<f32>,
    t: f32,
    front_face: bool,
}

struct Sphere {
    center: vec3<f32>,
    radius: f32,
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
    let outward_normal = ((*rec).p - s.center) / s.radius;
    set_face_normal(rec, r, outward_normal);

    return true;
}

fn ray_at(r: Ray, t: f32) -> vec3<f32> {
    return r.orig + t * r.dir;
}

fn ray_color(r: Ray) -> vec3<f32> {
    let world = array(Sphere(vec3<f32>(0, 0, -1), 0.5), Sphere(vec3<f32>(0, -100.5, -1), 100));
    let world_size: u32 = 2;

    var rec = HitRecord();
    var temp_rec = HitRecord();
    var hit_anything = false;
    var closest_so_far = INFINITY;

    for (var i: u32 = 0; i < world_size; i++) {
        if (hit_sphere(world[i], r, 0, closest_so_far, &temp_rec)) {
            hit_anything = true;
            closest_so_far = temp_rec.t;
            rec = temp_rec;
        }       
    }
    
    if (hit_anything) {
        return 0.5 * (rec.normal + vec3<f32>(1, 1, 1));
    }

    let unit_direction = normalize(r.dir);
    let a = 0.5 * (unit_direction.y + 1.0);
    return (1.0 - a) * vec3<f32>(1.0, 1.0, 1.0) + a * vec3<f32>(0.5, 0.7, 1.0);
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
    let i = global_id.x;
    let j = global_id.y;

    let image_width = dimensions.x;
    let image_height = dimensions.y;
    let aspect_ratio = f32(image_width) / f32(image_height);

    // The viewport is 'world coordinate' instead of pixels
    let  viewport_height = 2.0;
    let viewport_width = viewport_height * aspect_ratio;

    let focal_length = 1.0;
    let camera_center = vec3<f32>(0, 0, 0);
    let viewport_u = vec3<f32>(viewport_width, 0, 0);
    let viewport_v = vec3<f32>(0, -viewport_height, 0);
    let pixel_delta_u = viewport_u / f32(image_width);
    let pixel_delta_v = viewport_v / f32(image_height);
    let viewport_upper_left = camera_center - vec3<f32>(0, 0, focal_length) - viewport_u/2 - viewport_v/2;
    let pixel00_loc = viewport_upper_left + 0.5 * (pixel_delta_u + pixel_delta_v);

    let pixel_center = pixel00_loc + (f32(i) * pixel_delta_u) + (f32(j) * pixel_delta_v);
    let ray_direction = pixel_center - camera_center;
    let r = Ray(camera_center, ray_direction);

    let pixel_color = ray_color(r);
    
    textureStore(tex, vec2<i32>(global_id.xy), vec4<f32>(pixel_color, 1));
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
