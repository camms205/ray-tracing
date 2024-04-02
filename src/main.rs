use std::f32::consts::PI;

use bevy::{
    core_pipeline::{
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
        extract_resource::ExtractResource,
        render_graph::{RenderGraphApp, RenderLabel, RenderSubGraph, ViewNode, ViewNodeRunner},
        render_resource::{
            AsBindGroup, BindGroupEntries, BindGroupLayout, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, MultisampleState, PipelineCache,
            PrimitiveState, RenderPassDescriptor, RenderPipelineDescriptor, TextureFormat,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderApp,
    },
};
use ray_tracing::fly_cam::{FlyCam, NoCameraPlayerPlugin};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // WorldInspectorPlugin::default(),
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
    let material_black = materials.add(Color::BLACK);
    let material_red = materials.add(Color::RED);
    commands.spawn((
        Camera3dBundle {
            camera_render_graph: CameraRenderGraph::new(RayTracingGraph),
            transform: Transform::from_xyz(0., 1.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        DepthPrepass,
        NormalPrepass,
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
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct PrepassLabel;
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct UpscaleLabel;
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct RayTracingLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
struct RayTracingGraph;

struct RayTracingPlugin;
impl Plugin for RayTracingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off);
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
        let view_uniforms = &world.resource::<ViewUniforms>().uniforms;
        let view_uniforms = view_uniforms.binding().unwrap();
        let depth = view_prepass_textures.depth_view().unwrap();
        let normal = view_prepass_textures.normal_view().unwrap();
        let motion = view_prepass_textures.motion_vectors_view().unwrap();

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
                depth,
                normal,
                motion, // &ray_tracing_pipeline.sampler,
            )),
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
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[derive(Resource)]
struct RayTracingPipeline {
    layout: BindGroupLayout,
    pipeline_id: CachedRenderPipelineId,
}

#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
struct RayTracingInfo {
    #[uniform(0)]
    view_uniform: ViewUniform,
    #[texture(1, sample_type = "depth")]
    depth_view: Handle<Image>,
    #[texture(2)]
    normal_view: Handle<Image>,
    #[texture(3)]
    motion_view: Handle<Image>,
}

impl FromWorld for RayTracingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = RayTracingInfo::bind_group_layout(render_device);
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
