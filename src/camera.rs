use bevy::math::vec2;
use bevy::math::vec3;
use bevy::math::vec4;
use bevy::prelude::*;

#[derive(Resource, Default, Reflect, Clone)]
#[reflect(Resource)]
pub struct Camera {
    pub projection: Mat4,
    pub view: Mat4,
    pub inverse_projection: Mat4,
    pub inverse_view: Mat4,
    pub vertical_fov: f32,
    pub near_clip: f32,
    pub far_clip: f32,
    pub position: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub width: u32,
    pub height: u32,
}

impl Plugin for Camera {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.clone())
            .add_systems(PreUpdate, Self::resize)
            .add_systems(Update, update);
    }
}

pub fn update(keys: Res<Input<KeyCode>>, mut camera: ResMut<Camera>, time: Res<Time>) {
    let dt = time.delta_seconds();
    let speed = 1.0 * dt;
    keys.get_pressed().for_each(|key| {
        match key {
            KeyCode::W => camera.translate(Vec3::NEG_Z, speed),
            KeyCode::S => camera.translate(Vec3::Z, speed),
            KeyCode::A => camera.translate(Vec3::NEG_X, speed),
            KeyCode::D => camera.translate(Vec3::X, speed),
            _ => {}
        };
    });
}

impl Camera {
    pub fn new(vertical_fov: f32, near_clip: f32, far_clip: f32) -> Self {
        Self {
            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            inverse_projection: Mat4::IDENTITY,
            inverse_view: Mat4::IDENTITY,
            vertical_fov,
            near_clip,
            far_clip,
            position: vec3(0.0, 0.0, 3.0),
            forward: vec3(0.0, 0.0, -1.0),
            right: Vec3::X,
            width: 1,
            height: 1,
        }
    }

    pub fn translate(&mut self, dir: Vec3, speed: f32) {
        self.position += dir * speed;
        self.recalculate_view();
    }

    fn resize(mut camera: ResMut<Camera>, window: Query<&Window>) {
        let window = window.single();
        camera.on_resize(
            window.resolution.physical_width(),
            window.resolution.physical_height(),
        );
    }

    pub fn on_resize(&mut self, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        self.width = width;
        self.height = height;
        self.recalculate_projection();
        self.recalculate_view();
    }

    fn recalculate_projection(&mut self) {
        self.projection = Mat4::perspective_rh(
            self.vertical_fov.to_radians(),
            self.width as f32 / self.height as f32,
            self.near_clip,
            self.far_clip,
        );
        self.inverse_projection = self.projection.inverse();
    }

    fn recalculate_view(&mut self) {
        self.view = Mat4::look_at_rh(self.position, self.position + self.forward, Vec3::Y);
        self.inverse_view = self.view.inverse();
    }

    pub fn ray_directions(&self) -> impl Iterator<Item = impl Iterator<Item = Vec3> + '_> + '_ {
        (0..self.height).rev().map(move |y| {
            (0..self.width).map(move |x| {
                let uv = (vec2(x as f32, y as f32) / vec2(self.width as f32, self.height as f32))
                    * 2.0
                    - 1.0;
                let target = self.inverse_projection * vec4(uv.x, uv.y, 1.0, 1.0);
                (self.inverse_view * Vec4::from((target.xyz().normalize() / target.w, 0.0))).xyz()
            })
        })
    }
}
