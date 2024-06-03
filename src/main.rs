use bevy::{
    core_pipeline::prepass::MotionVectorPrepass,
    prelude::*,
    render::{
        camera::CameraRenderGraph,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use ray_tracing::{
    fly_cam::{FlyCam, NoCameraPlayerPlugin},
    ray_tracing::{GpuSphere, RayTracingGraph, RayTracingInfo, RayTracingPlugin},
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, NoCameraPlayerPlugin, RayTracingPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, close_on_q)
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
        sphere: vec![
            GpuSphere::new(Vec3::ZERO, 1.0, Color::FUCHSIA, Vec3::ZERO),
            GpuSphere::new(
                Vec3::new(2.0, 0.0, -1.0),
                1.0,
                Color::Rgba {
                    red: 0.2,
                    green: 0.7,
                    blue: 0.1,
                    alpha: 1.0,
                },
                Vec3::ZERO,
            ),
            GpuSphere::new(
                Vec3::new(0.0, -101.0, 0.0),
                100.0,
                Color::Rgba {
                    red: 0.2,
                    green: 0.3,
                    blue: 6.0,
                    alpha: 1.0,
                },
                Vec3::ZERO,
            ),
            // GpuSphere::new(
            //     Vec3::new(-50.0, 30.0, 50.0),
            //     20.0,
            //     Color::BLACK,
            //     Vec3::new(1.0, 0.0, 0.0),
            // ),
            // GpuSphere::new(
            //     Vec3::new(50.0, 30.0, -50.0),
            //     20.0,
            //     Color::BLACK,
            //     Vec3::new(0.0, 1.0, 0.0),
            // ),
            // GpuSphere::new(
            //     Vec3::new(-50.0, 30.0, -50.0),
            //     20.0,
            //     Color::BLACK,
            //     Vec3::new(0.0, 0.0, 1.0),
            // ),
        ],
        ..Default::default()
    });
    let material_black = materials.add(Color::BLACK);
    let material_red = materials.add(Color::RED);
    commands.spawn((
        Camera3dBundle {
            camera_render_graph: CameraRenderGraph::new(RayTracingGraph),
            transform: Transform::from_xyz(0., 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MotionVectorPrepass,
        FlyCam,
    ));
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: material_black.clone(),
        transform: Transform::from_rotation(Quat::from_axis_angle(Vec3::X, 45.0_f32.to_radians())),
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Plane3d::new(Vec3::Y)),
        material: material_black,
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Sphere::new(0.5).mesh().uv(32, 18)),
        material: material_red,
        transform: Transform::from_xyz(1., 0.5, 1.),
        ..default()
    });
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(-50.0, 30.0, 50.0),
        point_light: PointLight {
            color: Color::RED,
            radius: 1.0,
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(50.0, 30.0, -50.0),
        point_light: PointLight {
            color: Color::GREEN,
            radius: 1.0,
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(-50.0, 30.0, -50.0),
        point_light: PointLight {
            color: Color::BLUE,
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
