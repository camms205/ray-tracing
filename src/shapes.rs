use std::ops::Range;

use crate::hittable::{HitRecord, Hittable};

use bevy::prelude::*;

#[derive(Reflect, Resource)]
#[reflect(Resource, Default)]
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

#[derive(Reflect, Resource, Default)]
#[reflect(Resource, Default)]
pub struct Shapes(pub Vec<Shape>);

impl Shapes {
    pub fn new(vec: Vec<Shape>) -> Self {
        Self(vec)
    }
}

impl Hittable for Shapes {
    fn hit(&self, ray: &Ray, interval: Range<f32>) -> Option<HitRecord> {
        self.0.hit(ray, interval)
    }
}

#[derive(Reflect, Default)]
#[reflect(Default)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub albedo: Color,
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
        let closest_t = (-b - dis.sqrt()) / a;
        if !interval.contains(&closest_t) {
            return None;
        }
        let hit = origin + ray.direction * closest_t;
        let norm = hit.normalize();
        let light_dir = Vec3::splat(-1.0).normalize();
        let light = norm.dot(-light_dir);
        let col = self.albedo * light;
        Some(HitRecord {
            point: hit,
            normal: norm,
            t: closest_t,
            albedo: col,
        })
    }
}

#[derive(Reflect, Default)]
#[reflect(Default)]
pub struct Plane {
    pub point: Vec3,
    pub normal: Vec3,
    pub albedo: Color,
}

impl Hittable for Plane {
    fn hit(&self, ray: &Ray, interval: Range<f32>) -> Option<HitRecord> {
        let t = self.normal.dot(self.point - ray.origin) / self.normal.dot(ray.direction);
        (interval.contains(&t)).then_some(HitRecord {
            point: ray.origin + ray.direction * t,
            normal: self.normal,
            t,
            albedo: self.albedo,
        })
    }
}
