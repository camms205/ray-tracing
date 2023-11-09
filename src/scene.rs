use std::ops::Range;

use crate::hittable::{HitRecord, Hittable};
use crate::material::Mat;
use crate::shape::*;

use bevy::{math::vec3, prelude::*};

#[derive(Reflect, Resource)]
#[reflect(Resource, Default)]
pub struct Scene {
    pub shapes: Vec<Shape>,
    pub accumulate: bool,
    #[reflect(ignore)]
    pub frame_index: i32,
    #[reflect(ignore)]
    pub accumulation: Vec<f32>,
}

impl Default for Scene {
    fn default() -> Self {
        Scene::new(vec![
            Shape::Sphere(Sphere {
                center: vec3(0.0, 0.0, 0.0),
                radius: 1.0,
                material: Mat {
                    albedo: Color::rgb(1.0, 0.0, 1.0),
                    roughness: 0.8,
                    ..Default::default()
                },
            }),
            Shape::Sphere(Sphere {
                center: vec3(2.0, 0.0, -1.0),
                radius: 1.0,
                material: Mat {
                    albedo: Color::rgb(0.2, 0.7, 0.1),
                    roughness: 0.6,
                    ..Default::default()
                },
            }),
            Shape::Sphere(Sphere {
                center: vec3(0.0, -101.0, 0.0),
                radius: 100.0,
                material: Mat {
                    albedo: Color::rgb(0.2, 0.3, 6.0),
                    roughness: 0.5,
                    ..Default::default()
                },
            }),
            Shape::Sphere(Sphere {
                center: vec3(100.0, 101.0, -20.0),
                radius: 100.0,
                material: Mat {
                    emission: 3.0,
                    emission_color: Color::rgb(0.9, 0.9, 0.7),
                    ..Default::default()
                },
            }),
        ])
    }
}

impl Plugin for Scene {
    fn build(&self, app: &mut App) {
        app.insert_resource(Self::default())
            .register_type::<Scene>()
            .register_type::<Mat>()
            .register_type::<Sphere>()
            .register_type::<Plane>()
            .register_type::<Shape>();
    }
}

impl Scene {
    pub fn new(shapes: Vec<Shape>) -> Self {
        Self {
            shapes,
            accumulate: true,
            frame_index: 0,
            accumulation: vec![],
        }
    }

    pub fn resize(&mut self) {
        self.frame_index = -1;
    }
}

impl Hittable for Scene {
    fn hit(&self, ray: &Ray, interval: Range<f32>) -> Option<HitRecord> {
        self.shapes.hit(ray, interval)
    }
}
