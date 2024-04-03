#define_import_path ray_tracing::utils

#import bevy_render::globals::Globals

var<private> random_seed: f32 = 1.0;
fn rand(uv: vec2<f32>, globals: Globals) -> f32 {
    let val = fract(sin(dot(uv, vec2(12.9898, 78.233))) * 43758.5453123 * (random_seed * globals.delta_time * f32(globals.time % 200)));
    random_seed = val;
    return val;
}

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

