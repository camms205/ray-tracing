use bevy::{
    core_pipeline::{
        core_3d::{self, MainOpaquePass3dNode},
        deferred::node::DeferredGBufferPrepassNode,
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        prepass::{
            node::PrepassNode, DepthPrepass, MotionVectorPrepass, NormalPrepass,
            ViewPrepassTextures,
        },
        upscaling::UpscalingNode,
    },
    ecs::query::QueryItem,
    pbr::get_bindings,
    prelude::*,
    render::{
        camera::CameraRenderGraph,
        mesh::InnerMeshVertexBufferLayout,
        render_graph::{RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
        render_resource::{
            AsBindGroup, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingType, BufferBindingType, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, LoadOp, MultisampleState, Operations,
            PipelineCache, PrimitiveState, RawRenderPipelineDescriptor, RenderPassColorAttachment,
            RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderRef,
            ShaderStage, ShaderStages, ShaderType, TextureAspect, TextureFormat,
            TextureViewDescriptor,
        },
        renderer::{RenderContext, RenderDevice},
        texture::BevyDefault,
        view::{ViewTarget, ViewUniform, ViewUniforms},
        RenderApp,
    },
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use ray_tracing::fly_cam::{FlyCam, NoCameraPlayerPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugins(CpuRaytracing)
        .add_plugins(RayTracing)
        .add_plugins(WorldInspectorPlugin::new())
        .run();
}

struct RayTracing;

type Type = ViewNodeRunner<DeferredGBufferPrepassNode>;

const RAY_TRACING: &str = "ray_tracing";
impl Plugin for RayTracing {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<CustomMaterial>::default(),
            MaterialPlugin::<PrepassMaterial> {
                prepass_enabled: false,
                ..Default::default()
            },
            NoCameraPlayerPlugin,
        ))
        .register_type::<CustomMaterial>()
        .add_systems(Startup, setup)
        .insert_resource(Msaa::Off)
        .add_systems(Update, (update_quad_pos, rotate));
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        use core_3d::graph::node::*;
        render_app
            .add_render_sub_graph(RAY_TRACING)
            .add_render_graph_node::<ViewNodeRunner<PrepassNode>>(RAY_TRACING, PREPASS)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(RAY_TRACING, UPSCALING)
            .add_render_graph_node::<ViewNodeRunner<RayTracingNode>>(
                RAY_TRACING,
                RayTracingNode::NAME,
            )
            .add_render_graph_edges(RAY_TRACING, &[PREPASS, RayTracingNode::NAME, UPSCALING]);
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<RayTracingPipeline>();
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut depth: ResMut<Assets<PrepassMaterial>>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0., 0., 3.).looking_at(Vec3::ZERO, Vec3::Y),
            camera_render_graph: CameraRenderGraph::new(RAY_TRACING),
            ..default()
        },
        DepthPrepass,
        NormalPrepass,
        MotionVectorPrepass,
        FlyCam,
    ));
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(shape::Quad::new(Vec2::new(10.0, 10.0)).into()),
            material: depth.add(PrepassMaterial {}),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        RayTracingOutput,
    ));
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(CustomMaterial::default()),
        ..default()
    });
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(shape::Cube { size: 1.0 }.into()),
            material: materials.add(CustomMaterial {
                color: Color::GREEN,
            }),
            transform: Transform::from_xyz(-1.0, 0.5, 0.0),
            ..default()
        },
        Rotate,
    ));
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(
            shape::UVSphere {
                radius: 0.5,
                sectors: 32,
                stacks: 32,
            }
            .into(),
        ),
        material: materials.add(CustomMaterial {
            color: Color::WHITE,
        }),
        transform: Transform::from_xyz(1.0, 0.5, 0.0),
        ..default()
    });
}

#[derive(Component)]
struct Rotate;
fn rotate(mut transforms: Query<&mut Transform, With<Rotate>>, time: Res<Time>) {
    transforms.for_each_mut(|mut transform| {
        transform.rotate_y(5. * time.delta_seconds());
    });
}

#[derive(Component)]
struct RayTracingOutput;
fn update_quad_pos(
    mut quad: Query<&mut Transform, (With<RayTracingOutput>, Without<Camera>)>,
    camera: Query<&Transform, With<Camera>>,
) {
    let mut pos = quad.single_mut();
    let camera = camera.single();
    pos.rotation = camera.rotation;
    pos.translation = camera.translation + camera.forward() * 1.0;
}

#[derive(Default, Asset, Reflect, AsBindGroup, Debug, Clone)]
#[reflect(Default)]
struct CustomMaterial {
    #[uniform(0)]
    color: Color,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material.wgsl".into()
    }
}

#[derive(Default, Asset, Reflect, AsBindGroup, Debug, Clone)]
#[reflect(Default)]
struct PrepassMaterial {}

impl Material for PrepassMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/show_prepass.wgsl".into()
    }
}

#[derive(Resource)]
struct RayTracingPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for RayTracingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("pipeline_bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ViewUniform::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: bevy::render::render_resource::TextureSampleType::Depth,
                        view_dimension: bevy::render::render_resource::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: bevy::render::render_resource::TextureSampleType::Float {
                            filterable: false,
                        },
                        view_dimension: bevy::render::render_resource::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: bevy::render::render_resource::TextureSampleType::Float {
                            filterable: false,
                        },
                        view_dimension: bevy::render::render_resource::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(
                        bevy::render::render_resource::SamplerBindingType::Filtering,
                    ),
                    count: None,
                },
            ],
        });

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/ray_tracing.wgsl");
        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("ray_tracing_pipeline".into()),
                    layout: vec![layout.clone()],
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
            layout,
            sampler,
            pipeline_id,
        }
    }
}

#[derive(Default)]
struct RayTracingNode;
impl RayTracingNode {
    const NAME: &str = "pipeline_node";
}

impl ViewNode for RayTracingNode {
    type ViewQuery = (&'static ViewPrepassTextures, &'static ViewTarget);

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_prepass_textures, view_target): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let ray_tracing_pipeline = world.resource::<RayTracingPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(ray_tracing_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let view_uniforms = world.resource::<ViewUniforms>().uniforms.binding().unwrap();
        let depth = view_prepass_textures.depth.as_ref().unwrap();
        let normal = view_prepass_textures.normal.as_ref().unwrap();
        let motion = view_prepass_textures.motion_vectors.as_ref().unwrap();

        let depth_desc = TextureViewDescriptor {
            label: Some("prepass_depth"),
            aspect: TextureAspect::DepthOnly,
            ..default()
        };
        let depth_view = depth.texture.create_view(&depth_desc);

        let bind_group = render_context.render_device().create_bind_group(
            "ray_tracing_bind_group",
            &ray_tracing_pipeline.layout,
            &BindGroupEntries::sequential((
                view_uniforms.clone(),
                &depth_view,
                &normal.default_view,
                &motion.default_view,
                &ray_tracing_pipeline.sampler,
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("ray_tracing_pass"),
            color_attachments: &[Some(view_target.get_color_attachment(Operations {
                load: LoadOp::Clear(world.resource::<ClearColor>().0.into()),
                store: true,
            }))],
            depth_stencil_attachment: None,
        });
        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}
