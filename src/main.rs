use bevy::tasks::TaskPool;
use bevy::{math::vec3, prelude::*};
use bevy_egui::{egui, EguiContexts};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rand::Rng;
use ray_tracing::camera::Camera;
use ray_tracing::hittable::Hittable;
use ray_tracing::scene::Scene;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Camera::new(45.0, 0.1, 100.0),
            Scene::default(),
        ))
        .insert_resource(ImageHandle::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .add_systems(Update, ui_update)
        .run();
}

fn ui_update(mut contexts: EguiContexts, time: Res<Time>) {
    egui::Window::new("Frame time").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("{}", time.delta_seconds() * 1000.0));
    });
}

#[derive(Resource, Default)]
struct ImageHandle(Handle<Image>);

fn setup(
    mut commands: Commands,
    mut image_handle: ResMut<ImageHandle>,
    mut images: ResMut<Assets<Image>>,
) {
    image_handle.0 = images.add(Image::default());
    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: image_handle.0.clone(),
        ..Default::default()
    });
}

fn update(
    window: Query<&Window>,
    image_handle: Res<ImageHandle>,
    mut images: ResMut<Assets<Image>>,
    camera: Res<Camera>,
    mut scene: ResMut<Scene>,
) {
    let window = window.single();
    let (width, height) = (
        window.resolution.physical_width(),
        window.resolution.physical_height(),
    );
    let image = images.get_mut(image_handle.0.clone()).unwrap();
    image.resize(bevy::render::render_resource::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    });

    let cols = TaskPool::new()
        .scope(|s| {
            camera.ray_directions().for_each(|row| {
                s.spawn(async {
                    row.flat_map(|direction| {
                        per_pixel(
                            Ray {
                                origin: camera.position,
                                direction,
                            },
                            &scene,
                        )
                        .as_rgba_u8()
                    })
                    .collect::<Vec<u8>>()
                })
            });
        })
        .into_iter()
        .flatten()
        .collect::<Vec<u8>>();
    if scene.accumulate && scene.frame_index != -1 {
        scene.frame_index += 1;
        (scene.accumulation, image.data) = scene
            .accumulation
            .iter()
            .zip(cols.iter())
            .map(|(prev, new)| {
                let out = prev + (*new as f32 - prev) / scene.frame_index as f32;
                (out, out as u8)
            })
            .unzip();
    } else {
        scene.accumulation = cols.iter().map(|p| *p as f32).collect::<Vec<f32>>();
        scene.frame_index = 1;
        image.data = cols;
    }
}

fn per_pixel(ray: Ray, scene: &Scene) -> Color {
    let mut light = Color::BLACK;
    let mut contribution = Vec3::ONE;
    let mut ray = ray;
    let bounces = 8;
    for _ in 0..bounces {
        if let Some(hit_record) = scene.hit(&ray, 0.0001..f32::MAX) {
            let col = hit_record.material.albedo;
            light += hit_record.material.get_emission() * contribution;
            contribution *= vec3(col.r(), col.g(), col.b());
            let is_specular = hit_record.material.specular_chance >= rand::random();
            let diffuse = (hit_record.normal + rand_unit()).normalize();
            let specular =
                ray.direction - 2.0 * hit_record.normal.dot(ray.direction) * hit_record.normal;
            let direction = diffuse.lerp(
                specular,
                hit_record.material.roughness * is_specular as u8 as f32,
            );
            ray = Ray {
                origin: hit_record.point,
                direction,
            };
        } else {
            break;
        }
    }
    light
}

fn rand_unit() -> Vec3 {
    let mut rng = rand::thread_rng();
    vec3(
        rng.gen_range(-1.0..1.0),
        rng.gen_range(-1.0..1.0),
        rng.gen_range(-1.0..1.0),
    )
}
