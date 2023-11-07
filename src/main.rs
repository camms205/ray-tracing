pub mod hittable;

use bevy::{
    math::{uvec2, vec2, vec3, vec4},
    prelude::*,
    render::render_resource::Extent3d,
    window::WindowResized,
};
use hittable::{HitRecord, Hittable};
use itertools::Itertools;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Resolution(UVec2 { x: 100, y: 100 }))
        .insert_resource(ImageHandle::default())
        .insert_resource(Camera::new(45.0, 0.1, 100.0))
        .add_systems(Startup, setup)
        .add_systems(Update, on_resize)
        .add_systems(Update, update)
        .run();
}

#[derive(Resource)]
struct Resolution(UVec2);

#[derive(Resource, Default)]
struct ImageHandle(Handle<Image>);

fn setup(
    mut commands: Commands,
    mut image_handle: ResMut<ImageHandle>,
    mut images: ResMut<Assets<Image>>,
) {
    image_handle.0 = images.add(Image::new_fill(
        Extent3d {
            width: 100,
            height: 100,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[255u8; 100 * 100 * 4],
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
    ));
    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: image_handle.0.clone(),
        ..Default::default()
    });
}

fn on_resize(
    mut resize: EventReader<WindowResized>,
    mut res: ResMut<Resolution>,
    mut camera: ResMut<Camera>,
) {
    for ev in resize.read() {
        res.0 = uvec2(ev.width as u32, ev.height as u32);
    }
    camera.on_resize(res.0.x, res.0.y);
}

#[derive(Resource)]
struct Camera {
    projection: Mat4,
    view: Mat4,
    inverse_projection: Mat4,
    inverse_view: Mat4,
    vertical_fov: f32,
    near_clip: f32,
    far_clip: f32,
    position: Vec3,
    forward: Vec3,
    width: u32,
    height: u32,
}

impl Camera {
    fn new(vertical_fov: f32, near_clip: f32, far_clip: f32) -> Self {
        Self {
            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            inverse_projection: Mat4::IDENTITY,
            inverse_view: Mat4::IDENTITY,
            vertical_fov,
            near_clip,
            far_clip,
            position: vec3(0.0, 0.0, 3.0),
            forward: vec3(0.0, 0.0, -1.0),
            width: 100,
            height: 100,
        }
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        self.width = width;
        self.height = height;
        self.recalculate_projection();
        self.recalculate_view();
    }

    fn recalculate_projection(&mut self) {
        self.projection = Mat4::perspective_rh(
            self.vertical_fov.to_radians(),
            self.width as f32 / self.height as f32,
            self.near_clip,
            self.far_clip,
        );
        self.inverse_projection = self.projection.inverse();
    }

    fn recalculate_view(&mut self) {
        self.view = Mat4::look_at_rh(self.position, self.position + self.forward, Vec3::Y);
        self.inverse_view = self.view.inverse();
    }

    fn ray_directions(&self) -> impl Iterator<Item = Vec3> + '_ {
        (0..self.height)
            .rev()
            .cartesian_product(0..self.width)
            .map(|(y, x)| {
                let uv = (vec2(x as f32, y as f32) / vec2(self.width as f32, self.height as f32))
                    * 2.0
                    - 1.0;
                let target = self.inverse_projection * vec4(uv.x, uv.y, 1.0, 1.0);
                (self.inverse_view * Vec4::from((target.xyz().normalize() / target.w, 0.0))).xyz()
            })
    }
}

fn update(
    image_handle: Res<ImageHandle>,
    mut images: ResMut<Assets<Image>>,
    res: Res<Resolution>,
    camera: Res<Camera>,
) {
    let pixels = camera.ray_directions().flat_map(|dir| {
        vec4_to_u8(get_col(Ray {
            origin: camera.position,
            direction: dir,
        }))
    });
    images.insert(
        image_handle.0.clone(),
        Image::new_fill(
            Extent3d {
                width: res.0.x,
                height: res.0.y,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            pixels.collect_vec().as_slice(),
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        ),
    );
}

fn vec4_to_u8(col: Vec4) -> [u8; 4] {
    let col = col * 255.0;
    [col.x as u8, col.y as u8, col.z as u8, col.w as u8]
}

// given uv -1 - 1, return color vec4 0 - 1
fn get_col(ray: Ray) -> Vec4 {
    // return ray.direction.extend(1.0);
    let radius = 0.5;
    let center = vec3(0.0, 0.0, 0.0);

    if let Some(col) = sphere(ray, center, radius) {
        return col;
    } else {
        return Vec3::splat(0.0).extend(1.0);
    }
}

#[derive(Default)]
struct Sphere {
    center: Vec3,
    radius: f32,
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, interval: Range<f32>) -> Option<HitRecord> {
        let origin = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let b = origin.dot(ray.direction);
        let c = origin.dot(origin) - radius * radius;

        let dis = b * b - a * c;
        if dis < 0.0 {
            return None;
        }
        let closest_t = (-b - dis.sqrt()) / a;
        let hit = ray.origin + ray.direction * closest_t;
        let norm = hit.normalize();
        let light_dir = Vec3::splat(-1.0);
        let light = norm.dot(-light_dir);
        let col = (vec3(1.0, 0.0, 1.0) * light).extend(1.0);
        Some(HitRecord {
            point: hit,
            normal: norm,
            t: closest_t,
            color: col,
        })
    }
}

fn sphere(ray: Ray, center: Vec3, radius: f32) -> Option<Vec4> {
    let origin = ray.origin - center;
    let a = ray.direction.dot(ray.direction);
    let b = origin.dot(ray.direction);
    let c = origin.dot(origin) - radius * radius;

    let dis = b * b - a * c;
    (dis >= 0.0).then(|| {
        let closest_t = (-b - dis.sqrt()) / a;
        let hit = ray.origin + ray.direction * closest_t;
        let norm = hit.normalize();
        let light_dir = Vec3::splat(-1.0);
        let light = norm.dot(-light_dir);
        (vec3(1.0, 0.0, 1.0) * light).extend(1.0)
    })
}
