use crate::camera::Camera;
use crate::hittable::Hittable;
use crate::scene::Scene;
use bevy::math::vec3;
use bevy::prelude::*;
use bevy::tasks::ComputeTaskPool;
use bevy_egui::egui;
use bevy_egui::EguiContexts;
use rand::Rng;

pub struct CpuRaytracing;

impl Plugin for CpuRaytracing {
    fn build(&self, app: &mut App) {
        app.add_plugins((Camera::new(45.0, 0.1, 100.0), Scene::default()))
            .insert_resource(ImageHandle::default())
            .add_systems(Startup, setup)
            .add_systems(Update, update)
            .add_systems(Update, ui_update);
    }
}

pub fn ui_update(mut contexts: EguiContexts, time: Res<Time>) {
    egui::Window::new("Frame time").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("{}", time.delta_seconds() * 1000.0));
    });
}

#[derive(Resource, Default)]
pub struct ImageHandle(Handle<Image>);

pub fn setup(
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

pub fn update(
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

    let cols = ComputeTaskPool::get()
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

pub fn col_to_vec(col: Color) -> Vec3 {
    vec3(col.r(), col.g(), col.b())
}

pub fn per_pixel(ray: Ray, scene: &Scene) -> Color {
    let m = 2;
    let n = 1;
    let mut color = Color::BLACK;
    for _ in 0..n {
        color += if let Some(hit_record) = scene.hit(&ray, 0.0..f32::MAX) {
            let lights = &scene.lights;
            let mut rng = rand::thread_rng();
            let pos = hit_record.point;
            let norm = hit_record.normal;
            let mut reservoir = Reservoir::default();
            for _ in 0..lights.len().min(m) {
                let light_index = rng.gen_range(0..lights.len());
                let light = lights[light_index];
                let p = 1.0 / lights.len() as f32;
                let l = (light.get_pos() - pos)
                    .normalize()
                    .clamp(Vec3::ZERO, Vec3::ONE);
                let ndotl = norm.dot(l).clamp(0.0, 1.0);
                let color = light.sample() * col_to_vec(hit_record.material.albedo) * ndotl;
                let w = col_to_vec(color).length() / p;
                reservoir.update(light_index as f32, w);
            }
            let light = lights[reservoir.y as usize];
            let l = (light.get_pos() - pos)
                .normalize()
                .clamp(Vec3::ZERO, Vec3::ONE);
            let ndotl = norm.dot(l).clamp(0.0, 1.0);
            let color = light.sample() * ndotl;
            let p_hat = col_to_vec(color).length(); // should be divided by pdf of light sample but point is just 1
            if p_hat == 0.0 {
                reservoir.w = 0.0;
            } else {
                reservoir.w =
                    (1.0 / p_hat.max(0.00001)) * (reservoir.wsum / reservoir.m.max(0.000001));
            }
            if scene
                .hit(
                    &Ray {
                        origin: pos,
                        direction: l,
                    },
                    0.0001..f32::MAX,
                )
                .is_some()
            {
                reservoir.w = 0.0;
            }
            color * reservoir.w
        } else {
            Color::BLACK
        };
    }
    color * (1.0 / n as f32)

    // lights.sample();
    // Path tracing
    // let mut ray = ray;
    // let bounces = 8;
    // for _ in 0..bounces {
    //     if let Some(hit_record) = scene.hit(&ray, 0.0001..f32::MAX) {
    //         let direction = match hit_record.material {
    //             Mat::Standard(mat) => {
    //                 let col = mat.albedo;
    //                 contribution *= vec3(col.r(), col.g(), col.b());
    //                 let is_specular = mat.specular_chance >= rand::random();
    //                 let mut rng = rand::thread_rng();
    //                 let diffuse = (hit_record.normal
    //                     + vec3(
    //                         rng.gen_range(-1.0..1.0),
    //                         rng.gen_range(-1.0..1.0),
    //                         rng.gen_range(-1.0..1.0),
    //                     ))
    //                 .normalize();
    //                 let specular = ray.direction
    //                     - 2.0 * hit_record.normal.dot(ray.direction) * hit_record.normal;
    //                 diffuse.lerp(specular, mat.roughness * is_specular as u8 as f32)
    //             }
    //             Mat::Light(mat) => {
    //                 light += mat.get_emission() * contribution;
    //                 let mut rng = rand::thread_rng();
    //                 (hit_record.normal
    //                     + vec3(
    //                         rng.gen_range(-1.0..1.0),
    //                         rng.gen_range(-1.0..1.0),
    //                         rng.gen_range(-1.0..1.0),
    //                     ))
    //                 .normalize()
    //             }
    //         };
    //         ray = Ray {
    //             origin: hit_record.point,
    //             direction,
    //         };
    //     } else {
    //         break;
    //     }
    // }
}

#[derive(Default, Clone)]
pub struct Reservoir {
    pub y: f32,    // output sample
    pub wsum: f32, // sum of all weights
    pub m: f32,    // number of samples seen
    pub w: f32,
}

impl Reservoir {
    pub fn update(&mut self, x: f32, w: f32) {
        self.wsum += w;
        self.m += 1.0;
        if rand::random::<f32>() < (w / self.wsum) {
            self.y = x;
        }
    }

    pub fn combine(reservoirs: Vec<Reservoir>) -> Self {
        let mut s = Self::default();
        let mut m = 0.0;
        for r in reservoirs {
            s.update(r.y, todo!("p_hat(r.y) * r.w * r.m"));
            m += r.m;
        }
        s.m = m;
        s.w = todo!("(1/p_hat(s.y))(s.wsum/s.m)");
        s
    }

    pub fn ris(num_samples: u32) -> Self {
        let mut r = Self::default();
        for _ in 0..num_samples {
            let sample = todo!();
            r.update(sample, todo!("weight p_hat(x)/p(x)"));
        }
        r.w = todo!("(1/p_hat(r.y))(r.wsum/r.m)");
        r
    }
}
