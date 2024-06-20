use bevy::{
    core_pipeline::{core_3d::graph::Core3d, prepass::MotionVectorPrepass},
    prelude::*,
    render::camera::CameraRenderGraph,
};
use ray_tracing::{
    fly_cam::{FlyCam, NoCameraPlayerPlugin},
    ray_tracing::{RayTracingGraph, RayTracingInfo, RayTracingPlugin},
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, NoCameraPlayerPlugin, RayTracingPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (close_on_q, change_render_graph, rotate))
        .run();
}

fn close_on_q(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }

        if input.just_pressed(KeyCode::KeyQ) {
            commands.entity(window).despawn();
        }
    }
}

#[derive(Component, Default)]
enum Rendering {
    Core3d,
    #[default]
    RayTracing,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(RayTracingInfo {
        ..Default::default()
    });
    let material_black = materials.add(Color::BLACK);
    let material_red = materials.add(Color::srgb(1.0, 0.0, 0.0));
    commands.spawn((
        Camera3dBundle {
            camera_render_graph: CameraRenderGraph::new(RayTracingGraph),
            transform: Transform::from_xyz(0., 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MotionVectorPrepass,
        FlyCam,
    ));
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: material_black.clone(),
            transform: Transform::from_rotation(Quat::from_axis_angle(
                Vec3::X,
                45.0_f32.to_radians(),
            )),
            ..default()
        },
        Rotate,
    ));
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(5.0))),
        material: material_black,
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Sphere::new(0.5).mesh()),
        material: material_red,
        transform: Transform::from_xyz(1., 0.5, 1.),
        ..default()
    });
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(0.0, 50.0, 0.0),
        point_light: PointLight {
            color: Color::WHITE,
            radius: 1.0,
            ..Default::default()
        },
        ..Default::default()
    });
}

#[derive(Component)]
struct Rotate;

fn rotate(mut rotate: Query<&mut Transform, With<Rotate>>, time: Res<Time>) {
    for mut ele in rotate.iter_mut() {
        ele.rotate_y(1f32 * time.delta_seconds());
    }
}

fn change_render_graph(
    mut rendering: Local<Rendering>,
    mut query: Query<&mut CameraRenderGraph>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let mut render_graph = query.single_mut();

    if input.just_pressed(KeyCode::Tab) {
        match *rendering {
            Rendering::Core3d => {
                render_graph.set(RayTracingGraph);
                *rendering = Rendering::RayTracing
            }
            Rendering::RayTracing => {
                render_graph.set(Core3d);
                *rendering = Rendering::Core3d
            }
        }
    }
}
