#import bevy_render::view::View
#import bevy_render::globals::Globals
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

#import ray_tracing::utils
#import ray_tracing::utils::{
    Ray,
    Sphere,
    HitRecord
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var previous: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(2) var<uniform> globals: Globals;
@group(0) @binding(3) var motion_vector_prepass_texture: texture_2d<f32>;
@group(0) @binding(4) var<storage> spheres: array<Sphere>;
@group(0) @binding(5) var<storage> lights: array<Light>;


var<private> uv: vec2<f32>;

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

fn hit_scene(ray: Ray) -> HitRecord {
    var hit = utils::no_hit();
    let length = i32(arrayLength(&spheres));
    for (var i = 0; i < length; i++) {
        let record = utils::hit_sphere(ray, spheres[i]);
        if !record.hit {
            continue;
        }
        if !hit.hit || hit.t > record.t{
            hit = record;
        }
    }
    return hit;
}


fn randint(max: u32) -> u32 {
    return u32(rand()*f32(max));
}

fn rand() -> f32 {
    return utils::rand(uv, globals);
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

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    uv = in.uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    var origin = view.world_position;
    var dir = normalize(view.inverse_view_proj * vec4(uv, 0.0, 1.0)).xyz;
    let motion_vector = textureLoad(motion_vector_prepass_texture, vec2<i32>(in.position.xy), 0);

    var col = vec3(1.0);
    var light = vec3(0.0);
    var ray = Ray(origin, dir);
    for (var i = 0; i < 8; i++) {
        let record = hit_scene(ray);
        if record.hit {
            ray = Ray(record.point, rand_hemi(record.normal));
            // var light = sample_lights();
            // col += light.col.rgb;
            // let l = normalize(light.pos - record.point);
            // col *= record.color * dot(record.normal, l);
            light += record.light * col;
            col *= record.color;
        } else {
            light += vec3(0.5, 0.8, 0.9) * col;
            // light = ray.direction;
            break;
        }
    }
    let prev = textureLoad(previous, vec2<i32>(in.position.xy));
    // let prev = vec4(0.0);
    let out = (vec4(light, 1.0) + prev) / 2.0;
    textureStore(previous, vec2<i32>(in.position.xy), out);
    return out;
}
