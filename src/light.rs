use bevy::prelude::*;

#[derive(Reflect, Default, Copy, Clone)]
#[reflect(Default)]
pub struct Point {
    pub position: Vec3,
    pub color: Color,
    pub strength: f32,
}

impl Point {
    pub fn new(position: Vec3, color: Color, strength: f32) -> Self {
        Self {
            position,
            color,
            strength,
        }
    }
}

#[derive(Reflect, Copy, Clone)]
#[reflect(Default)]
pub enum Light {
    Point(Point),
}

impl Default for Light {
    fn default() -> Self {
        Self::Point(Point::default())
    }
}

impl Light {
    pub fn sample(&self) -> Color {
        match self {
            Light::Point(point) => point.color * point.strength,
        }
    }

    pub fn get_pos(&self) -> Vec3 {
        match self {
            Light::Point(point) => point.position,
        }
    }
}
