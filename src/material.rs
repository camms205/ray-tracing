use bevy::prelude::*;
use bevy_inspector_egui::{
    inspector_options::std_options::NumberDisplay, prelude::ReflectInspectorOptions,
    InspectorOptions,
};

#[derive(Reflect, InspectorOptions, Default, Clone, Copy)]
#[reflect(Default, InspectorOptions)]
pub struct Mat {
    pub albedo: Color,
    #[inspector(min = 0.0, max = 1.0, display = NumberDisplay::Slider)]
    pub roughness: f32,
    #[inspector(min = 0.0, max = f32::MAX, display = NumberDisplay::Drag)]
    pub emission: f32,
    pub emission_color: Color,
    #[inspector(min = 0.0, max = 1.0, display = NumberDisplay::Slider)]
    pub specular_chance: f32,
}

impl Mat {
    pub fn get_emission(&self) -> Color {
        self.emission_color * self.emission
    }
}
