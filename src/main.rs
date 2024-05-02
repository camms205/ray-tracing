use bevy::{
    asset::load_internal_asset,
    core::FrameCount,
    core_pipeline::{
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        prepass::{node::PrepassNode, MotionVectorPrepass, ViewPrepassTextures},
        upscaling::UpscalingNode,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::{CameraRenderGraph, ExtractedCamera},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        globals::{GlobalsBuffer, GlobalsUniform},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{RenderGraphApp, RenderLabel, RenderSubGraph, ViewNode, ViewNodeRunner},
        render_resource::{
            binding_types::{texture_2d, uniform_buffer},
            AsBindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FragmentState,
            MultisampleState, PipelineCache, PrimitiveState, RenderPassDescriptor,
            RenderPipelineDescriptor, ShaderStages, ShaderType, TextureDimension, TextureFormat,
            TextureUsages,
        },
        renderer::RenderDevice,
        texture::{BevyDefault, FallbackImage},
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderApp,
    },
    window::WindowResized,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use ray_tracing::fly_cam::{FlyCam, NoCameraPlayerPlugin};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            WorldInspectorPlugin::default(),
            NoCameraPlayerPlugin,
            RayTracingPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (close_on_q, resize))
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
    mut images: ResMut<Assets<Image>>,
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
        previous: images.add(image),
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
            GpuSphere::new(
                Vec3::new(-50.0, 30.0, 50.0),
                20.0,
                Color::BLACK,
                Vec3::new(1.0, 0.0, 0.0),
            ),
            GpuSphere::new(
                Vec3::new(50.0, 30.0, -50.0),
                20.0,
                Color::BLACK,
                Vec3::new(0.0, 1.0, 0.0),
            ),
            GpuSphere::new(
                Vec3::new(-50.0, 30.0, -50.0),
                20.0,
                Color::BLACK,
                Vec3::new(0.0, 0.0, 1.0),
            ),
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
    // commands.spawn(Light::new(Vec3::new(-50.0, 30.0, 50.0), Color::RED, 1.0));
    // commands.spawn(Light::new(Vec3::new(50.0, 30.0, -50.0), Color::GREEN, 1.0));
    // commands.spawn(Light::new(Vec3::new(-50.0, 30.0, -50.0), Color::BLUE, 1.0));
    // commands.spawn(Light::new(Vec3::new(0.0, 50.0, 0.0), Color::WHITE, 1.0));
}

fn resize(
    mut info: ResMut<RayTracingInfo>,
    mut images: ResMut<Assets<Image>>,
    mut resize_reader: EventReader<WindowResized>,
    frame_count: Res<FrameCount>,
) {
    for e in resize_reader.read() {
        info.count = frame_count.0;
        let width = e.width as u32;
        let height = e.height as u32;
        let image = images.get_mut(&info.previous).unwrap();
        image.resize(bevy::render::render_resource::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        });
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct PrepassLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct UpscaleLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct RayTracingLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
struct RayTracingGraph;

const RAY_TRACING_UTILS_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(199877112663398275092447563180262563067);

struct RayTracingPlugin;
impl Plugin for RayTracingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            RAY_TRACING_UTILS_HANDLE,
            "utils.wgsl",
            Shader::from_wgsl
        );
        app.insert_resource(Msaa::Off)
            .add_plugins(ExtractResourcePlugin::<RayTracingInfo>::default())
            .register_type::<Light>()
            .register_type::<GpuSphere>()
            .register_type::<RayTracingInfo>();
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .add_render_sub_graph(RayTracingGraph)
            .add_render_graph_node::<ViewNodeRunner<PrepassNode>>(RayTracingGraph, PrepassLabel)
            .add_render_graph_node::<ViewNodeRunner<RayTracingPassNode>>(
                RayTracingGraph,
                RayTracingLabel,
            )
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(RayTracingGraph, UpscaleLabel);
    }

    fn finish(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<RayTracingPipeline>();
    }
}

#[derive(Default)]
struct RayTracingPassNode;
impl ViewNode for RayTracingPassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        (camera, target, view_prepass_textures, view_uniform_offset): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let render_device = render_context.render_device();
        let view_uniforms = &world.resource::<ViewUniforms>().uniforms;
        let globals_uniform = world.resource::<GlobalsBuffer>().buffer.binding().unwrap();
        let motion = view_prepass_textures.motion_vectors_view().unwrap();
        let ray_tracing_pipeline = world.resource::<RayTracingPipeline>();
        let ray_tracing_info = world.resource::<RayTracingInfo>();
        let bind_group = ray_tracing_info
            .as_bind_group(
                &RayTracingInfo::bind_group_layout(render_device),
                render_device,
                world.resource::<RenderAssets<Image>>(),
                world.resource::<FallbackImage>(),
            )
            .unwrap()
            .bind_group;

        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(ray_tracing_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let global_bind_group = render_device.create_bind_group(
            "ray_tracing_bind_group",
            &ray_tracing_pipeline.layout,
            &BindGroupEntries::sequential((view_uniforms, globals_uniform, motion)),
        );
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("main_opaque_pass_3d"),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }
        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &global_bind_group, &[view_uniform_offset.offset]);
        render_pass.set_bind_group(1, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[derive(Resource)]
struct RayTracingPipeline {
    layout: BindGroupLayout,
    pipeline_id: CachedRenderPipelineId,
}

#[derive(Reflect, Default, Debug, Clone, ShaderType)]
struct Light {
    position: Vec3,
    color: Color,
    strength: f32,
}

impl Light {
    fn new(position: Vec3, color: Color, strength: f32) -> Light {
        Self {
            position,
            color,
            strength,
        }
    }
}

#[derive(Reflect, Default, Debug, Clone, ShaderType)]
struct GpuSphere {
    center: Vec3,
    radius: f32,
    color: Color,
    light: Vec3,
}

impl GpuSphere {
    fn new(center: Vec3, radius: f32, color: Color, light: Vec3) -> GpuSphere {
        Self {
            center,
            radius,
            color,
            light,
        }
    }
}

#[derive(Reflect, Clone, Resource, ExtractResource, AsBindGroup, Default)]
struct RayTracingInfo {
    // #[uniform(0)]
    // view_uniform: ViewUniform,
    #[storage_texture(0, visibility(fragment))]
    #[reflect(ignore)]
    previous: Handle<Image>,
    #[uniform(1)]
    count: u32,
    // #[uniform(3)]
    // globals: GlobalsUniform,
    // #[texture(2)]
    // motion_view: Handle<Image>,
    #[storage(2, read_only)]
    sphere: Vec<GpuSphere>,
    #[storage(3, read_only)]
    light: Vec<Light>,
}

impl FromWorld for RayTracingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = RayTracingInfo::bind_group_layout(render_device);
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/ray_tracing.wgsl");
        let global_layout = render_device.create_bind_group_layout(
            "gloabl_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<GlobalsUniform>(false),
                    texture_2d(bevy::render::render_resource::TextureSampleType::Float {
                        filterable: true,
                    }),
                ),
            ),
        );
        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("ray_tracing_pipeline".into()),
                    layout: vec![global_layout.clone(), layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::bevy_default(),
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    push_constant_ranges: vec![],
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                });
        Self {
            layout: global_layout,
            pipeline_id,
        }
    }
}
