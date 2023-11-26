#import bevy_render::view::View
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var depth_prepass_texture: texture_depth_2d;
@group(0) @binding(2) var normal_prepass_texture: texture_2d<f32>;
@group(0) @binding(3) var motion_vector_prepass_texture: texture_2d<f32>;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>
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

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv * vec2(1., -1.) * 2. - 1.;
    let origin = view.world_position;
    let ss_near = vec4(uv, 0., 1.);
    let ss_far = vec4(uv, 1., 1.);
    let near = view.inverse_view_proj * ss_near;
    let far = view.inverse_view_proj * ss_far;
    let dir1 = view.inverse_view_proj * vec4(uv, 1., 1.);
    // let dir1 = view.inverse_projection * vec4(uv, 1., 1.);
    // let dir = normalize(dir1.xyz);
    // let dir = normalize(far.xyz / far.w - near.xyz / near.w);
    // let dir = normalize(view.inverse_view * vec4((view.inverse_projection * vec4(uv, 0., 1.)).xyz, 1.)).xyz;
    let ray_world = view.inverse_view_proj * vec4(uv, 0., 1.);
    let dir = normalize(ray_world.xyz);
    let ray = Ray(origin, dir);
    let depth = textureLoad(depth_prepass_texture, vec2<i32>(in.position.xy), 0);
    let normal = textureLoad(normal_prepass_texture, vec2<i32>(in.position.xy), 0).xyz * 2. - 1.;
    let motion_vector = textureLoad(motion_vector_prepass_texture, vec2<i32>(in.position.xy), 0);
    let col = vec3(.2, .7, .9);
    let sphere = Sphere(vec3(0.), 0.5, col);
    let record = hit_sphere(ray, sphere);
    if record.hit {
        return vec4(record.color / record.t, 1.0);
    }
    return vec4(depth);
    // return vec4(dir - origin + depth, 1.0);
}
