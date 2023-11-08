use bevy::math::vec2;
use bevy::math::vec3;
use bevy::math::vec4;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;

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
    pub width: u32,
    pub height: u32,
}

impl Plugin for Camera {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.clone())
            .add_systems(PreUpdate, Self::resize)
            .register_type::<Camera>()
            .add_systems(Update, update);
    }
}

pub fn update(
    keys: Res<Input<KeyCode>>,
    mouse_button: Res<Input<MouseButton>>,
    mut mouse_move: EventReader<bevy::input::mouse::MouseMotion>,
    mut camera: ResMut<Camera>,
    time: Res<Time>,
    mut window: Query<&mut Window>,
) {
    let dt = time.delta_seconds();
    let speed = 1.0 * dt;
    keys.get_pressed().for_each(|key| {
        let forward = camera.forward;
        let right = forward.cross(Vec3::Y);
        match key {
            KeyCode::W => camera.translate(forward, speed),
            KeyCode::S => camera.translate(-forward, speed),
            KeyCode::A => camera.translate(-right, speed),
            KeyCode::D => camera.translate(right, speed),
            KeyCode::Q => camera.translate(-Vec3::Y, speed),
            KeyCode::E => camera.translate(Vec3::Y, speed),
            _ => {}
        };
    });

    let mut window = window.single_mut();
    if mouse_button.pressed(MouseButton::Right) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
        for ev in mouse_move.read() {
            camera.rotate(ev.delta, speed / 3.0);
        }
    } else {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }
    camera.recalculate_projection();
    camera.recalculate_view();
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
            width: 1,
            height: 1,
        }
    }

    pub fn translate(&mut self, dir: Vec3, speed: f32) {
        self.position += dir * speed;
    }

    pub fn rotate(&mut self, delta: Vec2, speed: f32) {
        let right = self.forward.cross(Vec3::Y);
        let q = Quat::from_axis_angle(right, -delta.y * speed)
            .mul_quat(Quat::from_axis_angle(Vec3::Y, -delta.x * speed))
            .normalize();
        self.forward = q.mul_vec3(self.forward);
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
