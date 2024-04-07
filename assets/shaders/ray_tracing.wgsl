#import bevy_render::view::View
#import bevy_render::globals::Globals
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

#import ray_tracing::utils
#import ray_tracing::utils::{
    hit_scene,
    Ray
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(0) @binding(2) var depth_prepass_texture: texture_depth_2d;
@group(0) @binding(3) var normal_prepass_texture: texture_2d<f32>;
@group(0) @binding(4) var motion_vector_prepass_texture: texture_2d<f32>;

var<private> uv: vec2<f32>;

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

struct Light {
    pos: vec3<f32>,
    col: vec3<f32>,
    str: f32,
}

const RED: vec3<f32> = vec3(1.0, 0.0, 0.0);
const GREEN: vec3<f32> = vec3(0.0, 1.0, 0.0);
const BLUE: vec3<f32> = vec3(0.0, 0.0, 1.0);

fn sample_lights() -> vec3<f32> {
    var lights = array<Light, 3>(
        Light(vec3(-50.0, 30.0, 50.0), RED, 1.0),
        Light(vec3(50.0, 30.0, -50.0), GREEN, 1.0),
        Light(vec3(-50.0, 30.0, -50.0), BLUE, 1.0),
    );
    return lights[randint(3u)].col;
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
        col = sample_lights();
        let l = normalize(vec3(1.));
        col *= record.color * dot(record.normal, l);
    }
    return vec4(col, 1.0);
}
