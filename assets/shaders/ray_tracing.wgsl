#import bevy_render::view::View
#import bevy_render::globals::Globals
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(0) @binding(2) var motion_vector_prepass_texture: texture_2d<f32>;
@group(1) @binding(1) var<uniform> frame_count: u32;
@group(1) @binding(2) var<storage> spheres: array<Sphere>;
@group(1) @binding(3) var<storage> lights: array<Light>;
@group(1) @binding(4) var<storage> triangles: array<Triangle>;
@group(1) @binding(5) var<storage> mesh_info: array<MeshInfo>;

struct Triangle{
    pos_a: vec3<f32>,
    pos_b: vec3<f32>,
    pos_c: vec3<f32>,
    norm_a: vec3<f32>,
    norm_b: vec3<f32>,
    norm_c: vec3<f32>,
}
struct MeshInfo{
    index: u32,
    count: u32,
}

var<private> uv: vec2<f32>;

var<private> state: u32 = 1u;
fn next_random() -> u32{
    state = state * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}
fn hash(uv: vec2<f32>) -> u32 {
    let random_seed = fract(sin(dot(uv, vec2(12.9898, 78.233))) * 43758.5453123);
    return u32(random_seed * 4294967295.0);
}

fn randint(max: u32) -> u32 {
    return u32(rand()*f32(max));
}

fn rand() -> f32 {
    return f32(next_random()) / 4294967295.0;
}

fn rand_norm() -> f32 {
    let theta = 2 * 3.1415926 * rand();
    let rho = sqrt(-2 * log(rand()));
    return rho * cos(theta);
}

fn rand_dir() -> vec3<f32> {
    return normalize(vec3(rand_norm(), rand_norm(), rand_norm()));
}

fn rand_hemi(norm: vec3<f32>) -> vec3<f32> {
    let dir = rand_dir();
    if dot(norm, dir) < 0 {
        return -dir;
    } else {
        return dir;
    }
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>
}

struct Sphere {
    center: vec3<f32>,
    radius: f32,
    material: vec3<f32>,
    light: vec3<f32>,
}

struct HitRecord {
    hit: bool,
    point: vec3<f32>,
    normal: vec3<f32>,
    t: f32,
    color: vec3<f32>,
    light: vec3<f32>,
}

fn no_hit() -> HitRecord {
    return HitRecord(false, vec3(0.), vec3(0.), 0., vec3(0.), vec3(0.0));
}

fn hit_sphere(ray: Ray, sphere: Sphere) -> HitRecord {
    let pos = ray.origin - sphere.center;
    let a = dot(ray.direction, ray.direction);
    let b = dot(pos, ray.direction);
    let c = dot(pos, pos) - sphere.radius * sphere.radius;
    let dis = b * b - a * c;
    if dis < 0. {
        return no_hit();
    }
    let t = (-b - sqrt(dis)) / a;
    if t < 0. {
        return no_hit();
    }
    let hit = pos + ray.direction * t;
    let norm = normalize(hit);
    let point = hit + sphere.center;
    let light = sphere.light;
    return HitRecord(true, point, norm, t, sphere.material, light);
}

struct Reservoir {
    x: f32, // the current value
    w: f32, // the current weight
    w_total: f32, // cumulative weight
    num: u32, // number of elements seen
}

fn new_reservoir() -> Reservoir {
    return Reservoir(0.0, 0.0, 0.0, 0);
}

fn update_reservoir(res: Reservoir, sample: f32, weight: f32) -> Reservoir {
    let w_total = res.w_total + weight;
    var x = 0.0;
    var w = 0.0;
    if (rand() < weight / w_total) {
        x = sample;
        w = weight;
    } else {
        x = res.x;
        w = res.w;
    }
    return Reservoir(x, w, w_total, res.num + 1);
}

struct Light {
    pos: vec3<f32>,
    col: vec4<f32>,
    str: f32,
}

const RED: vec3<f32> = vec3(1.0, 0.0, 0.0);
const GREEN: vec3<f32> = vec3(0.0, 1.0, 0.0);
const BLUE: vec3<f32> = vec3(0.0, 0.0, 1.0);

fn sample_lights() -> Light {
    let length = arrayLength(&lights);
    if length > 0 {
        return lights[randint(length)];
    } else {
        return Light(vec3(0.0), vec4(0.0), 0.0);
        // return Light(vec3(0.0), vec4(1.0), 1.0);
    }
    // return Light(vec3(0.0, 50.0, 0.0), vec4(1.0), 1.0);
}

fn ray_triangle(ray: Ray, tri: Triangle) -> HitRecord {
    var hit = no_hit();
    let edge_ab = tri.pos_b - tri.pos_a;
    let edge_ac = tri.pos_c - tri.pos_a;
    let norm = cross(edge_ab, edge_ac);
    let ray_cross = cross(ray.direction, edge_ac);
    let ao = ray.origin - tri.pos_a;
    let dao = cross(ao, ray.direction);

    let det = -dot(ray.direction, norm);
    let inv_det = 1f / det;

    let dst = dot(ao, norm) * inv_det;
    let u = dot(edge_ac, dao) * inv_det;
    let v = -dot(edge_ab, dao) * inv_det;
    let w = 1f - u - v;

    hit = HitRecord(det >=1e-6 && dst >= 0f && u >= 0f && v >= 0f && w >= 0f, ray.origin + ray.direction * dst, normalize(tri.norm_a * w + tri.norm_b * u + tri.norm_c * v), dst, vec3(1.0), vec3(0.0));
        
    // let det = dot(edge_ab, ray_cross);

    // if det > -0.00001 && det < 0.00001 {
    //     return hit;
    // }
    
    // let inv_det = 1.0 / det;
    // let s = ray.origin - tri.pos_a;
    // let u = inv_det * dot(s, ray_cross);
    // if u < 0.0 || u > 1.0 {
    //     return hit;
    // }

    // let s_cross = cross(s, edge_ab);
    // let v = inv_det * dot(ray.direction, s_cross);
    // if v < 0.0 || u + v > 1.0 {
    //     return hit;
    // }
    // let t = inv_det * dot(edge_ac, s_cross);
    // if t > 0.00001 {
    //     hit = HitRecord(true, ray.origin + ray.direction * t, cross(edge_ab, edge_ac) * 0.5 + 0.5, t, vec3(1.0), vec3(0.0));
    // }
    return hit;
}

fn hit_triangles(ray: Ray) -> HitRecord {
    var hit = no_hit();
    let length = i32(arrayLength(&triangles));
    for (var i = 0; i < length; i++) {
        let record = ray_triangle(ray, triangles[i]);
        if !record.hit {
            continue;
        }
        if !hit.hit || hit.t > record.t {
            hit = record;
        }
    }
    return hit;
}

fn hit_scene(ray: Ray) -> HitRecord {
    var hit = no_hit();
    let length = i32(arrayLength(&spheres));
    for (var i = 0; i < length; i++) {
        let record = hit_sphere(ray, spheres[i]);
        if !record.hit {
            continue;
        }
        if !hit.hit || hit.t > record.t{
            hit = record;
        }
    }
    return hit;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    uv = in.uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    state = hash(uv); 
    var origin = view.world_position;
    var dir = normalize(view.world_from_clip * vec4(uv, 0.0, 1.0)).xyz;
    let motion_vector = textureLoad(motion_vector_prepass_texture, vec2<i32>(in.position.xy), 0);

    // let samples = 8;
    // var light = vec3(0.0);
    // for (var j = 0; j < samples; j++) {
    //     var col = vec3(1.0);
    //     var ray = Ray(origin, dir + vec3(rand(), rand(), 0.0) * 0.0015);
    //     for (var i = 0; i < 4; i++) {
    //         let record = hit_scene(ray);
    //         if record.hit {
    //             ray = Ray(record.point, rand_hemi(record.normal));
    //             // var light = sample_lights();
    //             // col += light.col.rgb;
    //             // let l = normalize(light.pos - record.point);
    //             // col *= record.color * dot(record.normal, l);
    //             light += record.light * col;
    //             col *= record.color;
    //             // light = record.color;
    //         } else {
    //             light += vec3(0.5, 0.71, 0.86) * col;
    //             // light += vec3(0.3) * col;
    //             break;
    //         }
    //     }
    // }
    let record = hit_triangles(Ray(origin, dir));
    // let record = ray_triangle(Ray(origin, dir), triangles[0]);
    if record.hit {
        return vec4((record.normal * 0.5 + 0.5 ), 1.0);
    } else {
        return vec4(0.0);
    }
    // return vec4(light / f32(samples), 1.0);
}
