#import bevy_render::view::View
#import bevy_render::globals::Globals
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(0) @binding(2) var depth_prepass_texture: texture_depth_2d;
@group(0) @binding(3) var normal_prepass_texture: texture_2d<f32>;
@group(0) @binding(4) var motion_vector_prepass_texture: texture_2d<f32>;

var<private> random_seed: f32 = 1.0;
var<private> uv: vec2<f32>;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>
}

struct Reservoir {
    x: f32,
    w: f32,
    w_total: f32,
    num: u32,
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

struct Sphere {
    center: vec3<f32>,
    radius: f32,
    material: vec3<f32>,
}

struct HitRecord {
    hit: bool,
    point: vec3<f32>,
    normal: vec3<f32>,
    t: f32,
    color: vec3<f32>,
}

fn rand() -> f32 {
    let val = fract(sin(dot(uv, vec2(12.9898, 78.233))) * 43758.5453123 * (random_seed * globals.delta_time * f32(globals.time)));
    random_seed = val;
    return val;
}

fn rand3() -> vec3<f32> {
    return vec3(rand(), rand(), rand());
}

fn no_hit() -> HitRecord {
    return HitRecord(false, vec3(0.), vec3(0.), 0., vec3(0.));
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
    let light_dir = normalize(vec3(-1.));
    let light = dot(norm, -light_dir);
    return HitRecord(true, point, norm, t, sphere.material * light);
}

fn hit_scene(ray: Ray) -> HitRecord {
    var scene = array<Sphere, 3>(Sphere(vec3(0.0), 1.0, vec3(1.0, 0.0, 1.0)), Sphere(vec3(2.0, 0.0, -1.0), 1.0, vec3(0.2, 0.7, 0.1)), Sphere(vec3(0.0, -101.0, 0.0), 100.0, vec3(0.2, 0.3, 6.0)));
    var hit = no_hit();
    for (var i = 0; i < 3; i += 1) {
        let record = hit_sphere(ray, scene[i]);
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
    let origin = view.world_position;
    let dir = normalize(view.inverse_view_proj * vec4(uv, 0.0, 1.0)).xyz;
    let depth = textureLoad(depth_prepass_texture, vec2<i32>(in.position.xy), 0);
    let normal = textureLoad(normal_prepass_texture, vec2<i32>(in.position.xy), 0).xyz * 2. - 1.;
    let motion_vector = textureLoad(motion_vector_prepass_texture, vec2<i32>(in.position.xy), 0);
    let record = hit_scene(Ray(origin, dir));
    var col = vec3(0.0);
    if record.hit {
        col = record.color / record.t;
        let record1 = hit_scene(Ray(record.point, rand3()));
        col += record1.color * 0.5;
    }
    return vec4(col, 1.0);
}
