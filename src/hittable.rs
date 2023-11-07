use bevy::prelude::*;

pub trait Hittable {
    fn hit(&self, ray: &Ray, interval: Range<f32>) -> Option<HitRecord>;
}

pub struct HitRecord {
    pub point: Vec3,
    pub normal: Vec3,
    pub t: f32,
    pub color: Vec4,
}

impl<T> Hittable for Vec<T>
where
    T: Hittable + Sync,
{
    fn hit(&self, ray: &Ray, interval: Range<f32>) -> Options<HitRecord> {
        let (_closest, hit_record) = self.iter().fold((interval.end, None), |acc, item| {
            if let Some(temp_rec) = item.hit(ray, iterval.start..acc.0) {
                (temp_rec.t, Some(temp_rec))
            } else {
                acc
            }
        });
        hit_record
    }
}
