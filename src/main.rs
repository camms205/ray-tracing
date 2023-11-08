use bevy::tasks::TaskPool;
use bevy::{math::vec3, prelude::*};
use bevy_egui::{egui, EguiContexts};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use ray_tracing::hittable::Hittable;
use ray_tracing::shapes::{Shape, Shapes};
use ray_tracing::{camera::Camera, shapes::*};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Camera::new(45.0, 0.1, 100.0)))
        .insert_resource(ImageHandle::default())
        .insert_resource(Shapes::new(vec![
            Shape::Sphere(Sphere {
                center: vec3(0.0, 0.0, 0.0),
                radius: 0.5,
                albedo: Color::rgb(1.0, 0.0, 1.0),
            }),
            Shape::Plane(Plane {
                point: vec3(0.0, -1.0, 0.0),
                normal: Vec3::Y,
                albedo: Color::rgb(0.2, 0.3, 0.8),
            }),
        ]))
        .register_type::<Shapes>()
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
    spheres: Res<Shapes>,
) {
    let window = window.single();
    let (width, height) = (
        window.resolution.physical_width(),
        window.resolution.physical_height(),
    );
    // do magic to spawn a task for each line in the window
    let pixels: Vec<u8> = TaskPool::new()
        .scope(|s| {
            camera.ray_directions().for_each(|row| {
                s.spawn(async {
                    row.flat_map(|dir| {
                        get_col(
                            Ray {
                                origin: camera.position,
                                direction: dir,
                            },
                            &spheres,
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
    images.insert(
        image_handle.0.clone(),
        Image::new_fill(
            bevy::render::render_resource::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            pixels.as_slice(),
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        ),
    );
}

fn get_col(ray: Ray, shapes: &Res<Shapes>) -> Color {
    if let Some(hit_record) = shapes.hit(&ray, 0.0..100.0) {
        hit_record.albedo * (-hit_record.t + 1.0).clamp(0.0, 1.0)
    } else {
        Color::BLACK
    }
}
