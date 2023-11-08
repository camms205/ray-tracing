use std::ops::Range;

use crate::hittable::{HitRecord, Hittable};

use bevy::{math::vec3, prelude::*};
use bevy_inspector_egui::{
    inspector_options::std_options::NumberDisplay, prelude::ReflectInspectorOptions,
    InspectorOptions,
};

#[derive(Reflect)]
#[reflect(Default)]
pub enum Shape {
    Sphere(Sphere),
    Plane(Plane),
}

impl Default for Shape {
    fn default() -> Self {
        Self::Sphere(Sphere::default())
    }
}

impl Hittable for Shape {
    fn hit(
        &self,
        ray: &bevy::prelude::Ray,
        interval: std::ops::Range<f32>,
    ) -> Option<crate::hittable::HitRecord> {
        match self {
            Shape::Sphere(object) => object.hit(ray, interval),
            Shape::Plane(object) => object.hit(ray, interval),
        }
    }
}

#[derive(Reflect, Resource)]
#[reflect(Resource, Default)]
pub struct Scene {
    pub shapes: Vec<Shape>,
    pub accumulation: Option<Vec<Color>>,
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
                },
            }),
            Shape::Sphere(Sphere {
                center: vec3(0.0, -101.0, 0.0),
                radius: 100.0,
                material: Mat {
                    albedo: Color::rgb(0.2, 0.3, 1.0),
                    roughness: 0.5,
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
            .register_type::<Shape>();
    }
}

impl Scene {
    pub fn new(shapes: Vec<Shape>) -> Self {
        Self {
            shapes,
            accumulation: None,
        }
    }
}

impl Hittable for Scene {
    fn hit(&self, ray: &Ray, interval: Range<f32>) -> Option<HitRecord> {
        self.shapes.hit(ray, interval)
    }
}

#[derive(Reflect, InspectorOptions, Default, Clone, Copy)]
#[reflect(Default, InspectorOptions)]
pub struct Mat {
    pub albedo: Color,
    #[inspector(min = 0.0, max = 1.0, display = NumberDisplay::Slider)]
    pub roughness: f32,
}

#[derive(Reflect, Default)]
#[reflect(Default)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: Mat,
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, interval: Range<f32>) -> Option<HitRecord> {
        let origin = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let b = origin.dot(ray.direction);
        let c = origin.dot(origin) - self.radius * self.radius;

        let dis = b * b - a * c;
        if dis < 0.0 {
            return None;
        }
        let t = (-b - dis.sqrt()) / a;
        if !interval.contains(&t) {
            return None;
        }
        let hit = origin + ray.direction * t;
        let normal = hit.normalize();
        let point = hit + self.center;
        let material = self.material;
        Some(HitRecord {
            point,
            normal,
            t,
            material,
        })
    }
}

#[derive(Reflect, Default)]
#[reflect(Default)]
pub struct Plane {
    pub point: Vec3,
    pub normal: Vec3,
    pub material: Mat,
}

impl Hittable for Plane {
    fn hit(&self, ray: &Ray, interval: Range<f32>) -> Option<HitRecord> {
        let t = self.normal.dot(self.point - ray.origin) / self.normal.dot(ray.direction);
        (interval.contains(&t)).then_some(HitRecord {
            point: ray.origin + ray.direction * t,
            normal: self.normal,
            t,
            material: self.material,
        })
    }
}
