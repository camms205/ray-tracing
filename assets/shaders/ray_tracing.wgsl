#import bevy_render::view::View
#import bevy_render::globals::Globals
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(0) @binding(2) var motion_vector_prepass_texture: texture_2d<f32>;
@group(1) @binding(0) var<uniform> frame_count: u32;
@group(1) @binding(1) var<storage> triangles: array<Triangle>;
@group(1) @binding(2) var<storage> mesh_info: array<MeshInfo>;
@group(1) @binding(3) var<storage> vertices: array<Vertex>;
@group(1) @binding(4) var<storage> materials: array<Material>;

struct Vertex{
    pos: vec3<f32>,
    norm: vec3<f32>,
}

struct Triangle{
    pos_a: u32,
    pos_b: u32,
    pos_c: u32,
}
struct MeshInfo{
    index: u32,
    count: u32,
    material: u32,
    aabb_left_bottom: vec3<f32>,
    aabb_right_top: vec3<f32>,
}

struct Material {
    color: vec4<f32>,
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

struct HitRecord {
    hit: bool,
    point: vec3<f32>,
    normal: vec3<f32>,
    t: f32,
    material: u32,
}

fn no_hit() -> HitRecord {
    return HitRecord(false, vec3(0.), vec3(0.), 0., 0);
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

fn ray_triangle(ray: Ray, tri: Triangle) -> HitRecord {
    let vertex_a = vertices[tri.pos_a];
    let vertex_b = vertices[tri.pos_b];
    let vertex_c = vertices[tri.pos_c];
    let edge_ab = vertex_b.pos - vertex_a.pos;
    let edge_ac = vertex_c.pos - vertex_a.pos;
    let norm = cross(edge_ab, edge_ac);
    let ray_cross = cross(ray.direction, edge_ac);
    let ao = ray.origin - vertex_a.pos;
    let dao = cross(ao, ray.direction);

    let det = -dot(ray.direction, norm);
    let inv_det = 1f / det;

    let dst = dot(ao, norm) * inv_det;
    let u = dot(edge_ac, dao) * inv_det;
    let v = -dot(edge_ab, dao) * inv_det;
    let w = 1f - u - v;

    return HitRecord(det >=1e-6 && dst >= 0f && u >= 0f && v >= 0f && w >= 0f, ray.origin + ray.direction * dst, 
                    normalize(vertex_a.norm * w + vertex_b.norm * u + vertex_c.norm * v), dst, 0);
}

fn ray_aabb(ray: Ray, lb: vec3<f32>, rt: vec3<f32>) -> bool {
    let ray_inv = 1.0 / ray.direction;
    let t1 = (lb - ray.origin) * ray_inv;
    let t2 = (rt - ray.origin) * ray_inv;
    let tmin = max(max(min(t1.x, t2.x), min(t1.y, t2.y)), min(t1.z, t2.z));
    let tmax = min(min(max(t1.x, t2.x), max(t1.y, t2.y)), max(t1.z, t2.z));
    return tmax >= tmin;
}

fn hit_triangles(ray: Ray) -> HitRecord {
    var hit = no_hit();
    let length = i32(arrayLength(&mesh_info));
    for (var i = 0; i < length; i++) {
        let mesh = mesh_info[i];
        if !ray_aabb(ray, mesh.aabb_left_bottom, mesh.aabb_right_top) {
            continue;
        }
        for (var j = mesh.index; j < mesh.index + mesh.count; j++) {
            var record = ray_triangle(ray, triangles[j]);
            record.material = mesh.material;
            if !record.hit {
                continue;
            }
            if !hit.hit || hit.t > record.t {
                hit = record;
            }
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
    if record.hit {
        if record.material >= 100 {
            return vec4(0.5 + f32(record.material - 100u) / 6.0);
        }
        return materials[record.material].color * dot(record.normal, vec3(0f, 1f, 0f));
    } else {
        return vec4(0.0);
    }
    // return vec4(light / f32(samples), 1.0);
}
