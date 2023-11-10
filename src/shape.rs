use crate::hittable::HitRecord;
use crate::hittable::Hittable;
use crate::material::Mat;
use bevy::prelude::*;
use std::ops::Range;

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
