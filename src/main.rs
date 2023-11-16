use bevy::{
    core_pipeline::{
        clear_color::ClearColorConfig,
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        prepass::{
            node::PrepassNode, DepthPrepass, MotionVectorPrepass, NormalPrepass,
            ViewPrepassTextures,
        },
        upscaling::UpscalingNode,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::{CameraRenderGraph, ExtractedCamera},
        render_graph::{RenderGraphApp, ViewNode, ViewNodeRunner},
        render_resource::{
            BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingType::{self},
            BufferBindingType, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            FragmentState, LoadOp, MultisampleState, Operations, PipelineCache, PrimitiveState,
            RenderPassDescriptor, RenderPipelineDescriptor, ShaderStages, ShaderType,
            TextureAspect, TextureFormat, TextureViewDescriptor,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderApp,
    },
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
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(Color::BLACK.into());
    commands.spawn((
        Camera3dBundle {
            camera_render_graph: CameraRenderGraph::new(RAY_TRACING),
            transform: Transform::from_xyz(0., 1.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        DepthPrepass,
        NormalPrepass,
        MotionVectorPrepass,
        FlyCam,
    ));
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(shape::Cube::new(1.0).into()),
        material: material.clone(),
        transform: Transform::from_xyz(0., 0.0, 0.)
            .with_rotation(Quat::from_axis_angle(Vec3::X, 45.0_f32.to_radians())),
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(shape::Plane::from_size(3.).into()),
        material: material.clone(),
        ..default()
    });
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(
            shape::UVSphere {
                radius: 0.5,
                sectors: 16,
                stacks: 16,
            }
            .into(),
        ),
        material,
        transform: Transform::from_xyz(1., 0.5, 1.),
        ..default()
    });
}

const RAY_TRACING: &str = "ray_tracing";

struct RayTracingPlugin;
impl Plugin for RayTracingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off);
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .add_render_sub_graph(RAY_TRACING)
            .add_render_graph_node::<ViewNodeRunner<PrepassNode>>(RAY_TRACING, "prepass")
            .add_render_graph_node::<ViewNodeRunner<RayTracingPassNode>>(
                RAY_TRACING,
                "ray_tracing_node",
            )
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(RAY_TRACING, "upscaling_node");
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
        &'static Camera3d,
        &'static ViewTarget,
        &'static ViewUniformOffset,
        &'static ViewPrepassTextures,
    );

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        (camera, camera_3d, target, view_uniform_offset, view_prepass_textures): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let load = match camera_3d.clear_color {
            ClearColorConfig::Default => LoadOp::Clear(world.resource::<ClearColor>().0.into()),
            ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
            ClearColorConfig::None => LoadOp::Load,
        };

        let view_uniforms = &world.resource::<ViewUniforms>().uniforms;
        let view_uniforms = view_uniforms.binding().unwrap();
        let depth = view_prepass_textures.depth.as_ref().unwrap();
        let normal = view_prepass_textures.normal.as_ref().unwrap();
        let motion = view_prepass_textures.motion_vectors.as_ref().unwrap();

        let depth_desc = TextureViewDescriptor {
            label: Some("prepass_depth"),
            aspect: TextureAspect::DepthOnly,
            ..default()
        };
        let depth_view = depth.texture.create_view(&depth_desc);

        let ray_tracing_pipeline = world.resource::<RayTracingPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(ray_tracing_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            "ray_tracing_bind_group",
            &ray_tracing_pipeline.layout,
            &BindGroupEntries::sequential((
                view_uniforms,
                &depth_view,
                &normal.default_view,
                &motion.default_view,
                // &ray_tracing_pipeline.sampler,
            )),
        );
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("main_opaque_pass_3d"),
            color_attachments: &[Some(
                target.get_color_attachment(Operations { load, store: true }),
            )],
            depth_stencil_attachment: None,
        });

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }
        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[view_uniform_offset.offset]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[derive(Resource)]
struct RayTracingPipeline {
    layout: BindGroupLayout,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for RayTracingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ray_tracing_pipeline_bind_group_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
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
            ],
        });

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
            pipeline_id,
        }
    }
}
