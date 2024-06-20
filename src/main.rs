use bevy::{
    core_pipeline::prepass::MotionVectorPrepass,
    prelude::*,
    render::{
        camera::CameraRenderGraph,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};
use ray_tracing::{
    fly_cam::{FlyCam, NoCameraPlayerPlugin},
    ray_tracing::{RayTracingGraph, RayTracingInfo, RayTracingPlugin},
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, NoCameraPlayerPlugin, RayTracingPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, close_on_q)
        .add_systems(Update, rotate)
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
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::STORAGE_BINDING;
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
        transform: Transform::from_xyz(-50.0, 30.0, 50.0),
        point_light: PointLight {
            color: Color::srgb(1.0, 0.0, 0.0),
            radius: 1.0,
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(50.0, 30.0, -50.0),
        point_light: PointLight {
            color: Color::srgb(0.0, 1.0, 0.0),
            radius: 1.0,
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(-50.0, 30.0, -50.0),
        point_light: PointLight {
            color: Color::srgb(0.0, 0.0, 1.0),
            radius: 1.0,
            ..Default::default()
        },
        ..Default::default()
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

fn rotate(mut rotate: Query<&mut Transform, With<Rotate>>) {
    for mut ele in rotate.iter_mut() {
        ele.rotate_y(0.1);
    }
}
