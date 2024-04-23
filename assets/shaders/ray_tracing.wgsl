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
@group(0) @binding(1) var<uniform> globals: Globals;
@group(0) @binding(2) var depth_prepass_texture: texture_depth_2d;
@group(0) @binding(3) var normal_prepass_texture: texture_2d<f32>;
@group(0) @binding(4) var motion_vector_prepass_texture: texture_2d<f32>;
@group(0) @binding(5) var<storage> spheres: array<Sphere>;
@group(0) @binding(6) var<storage> lights: array<Light>;


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
        var light = sample_lights();
        col = light.col.rgb;
        let l = normalize(light.pos - record.point);
        col *= record.color * dot(record.normal, l);
    }
    return vec4(col, 1.0);
}
