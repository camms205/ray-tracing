use bevy::tasks::TaskPool;
use bevy::{math::vec3, prelude::*};
use bevy_egui::{egui, EguiContexts};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
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
    scene: Res<Scene>,
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
    // do magic to spawn a task for each line in the window
    image.data = TaskPool::new()
        .scope(|s| {
            camera.ray_directions().for_each(|row| {
                s.spawn(async {
                    row.flat_map(|dir| {
                        per_pixel(
                            Ray {
                                origin: camera.position,
                                direction: dir,
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
}

fn per_pixel(ray: Ray, scene: &Scene) -> Color {
    let mut col = Color::BLACK;
    let sky = Color::rgb(0.6, 0.7, 0.9);
    let mut factor = 1.0;
    let mut ray = ray;
    let bounces = 3;
    for _ in 0..bounces {
        if let Some(hit_record) = scene.hit(&ray, 0.0..100.0) {
            let light_dir = Vec3::splat(-1.0).normalize();
            let light_intensity = hit_record.normal.dot(-light_dir);
            col += hit_record.material.albedo * light_intensity * factor;
            factor *= 0.5;
            let dir = ray.direction;
            let norm = hit_record.normal
                + hit_record.material.roughness
                    * (vec3(rand::random(), rand::random(), rand::random()) - 0.5);
            ray = Ray {
                origin: hit_record.point,
                direction: dir - 2.0 * dir.dot(norm) * norm,
            };
        } else {
            col += sky * factor;
            break;
        }
    }
    col
}
