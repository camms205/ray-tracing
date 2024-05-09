use bevy::{
    asset::load_internal_asset,
    core_pipeline::{
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        prepass::{node::PrepassNode, ViewPrepassTextures},
        upscaling::UpscalingNode,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        globals::{GlobalsBuffer, GlobalsUniform},
        render_asset::RenderAssets,
        render_graph::{RenderGraphApp, RenderLabel, RenderSubGraph, ViewNode, ViewNodeRunner},
        render_resource::{
            binding_types::{texture_2d, uniform_buffer},
            AsBindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, MultisampleState,
            PipelineCache, PrimitiveState, RenderPassDescriptor, RenderPipelineDescriptor,
            ShaderStages, ShaderType, TextureFormat,
        },
        renderer::RenderDevice,
        texture::{BevyDefault, FallbackImage},
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        RenderApp,
    },
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct PrepassLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct UpscaleLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct RayTracingLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
pub struct RayTracingGraph;

pub const RAY_TRACING_UTILS_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(199877112663398275092447563180262563067);

pub struct RayTracingPlugin;

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
            .register_type::<GpuSphere>()
            .register_type::<RayTracingInfo>();
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_sub_graph(RayTracingGraph)
                .add_render_graph_node::<ViewNodeRunner<PrepassNode>>(RayTracingGraph, PrepassLabel)
                .add_render_graph_node::<ViewNodeRunner<RayTracingPassNode>>(
                    RayTracingGraph,
                    RayTracingLabel,
                )
                .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(
                    RayTracingGraph,
                    UpscaleLabel,
                );
        }
    }

    fn finish(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .unwrap()
            .init_resource::<RayTracingPipeline>();
    }
}

#[derive(Default)]
pub struct RayTracingPassNode;

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
pub struct RayTracingPipeline {
    pub layout: BindGroupLayout,
    pub pipeline_id: CachedRenderPipelineId,
}

#[derive(Reflect, Default, Debug, Clone, ShaderType)]
pub struct GpuSphere {
    pub center: Vec3,
    pub radius: f32,
    pub color: Color,
    pub light: Vec3,
}

impl GpuSphere {
    pub fn new(center: Vec3, radius: f32, color: Color, light: Vec3) -> GpuSphere {
        Self {
            center,
            radius,
            color,
            light,
        }
    }
}

#[derive(Reflect, Clone, Resource, ExtractResource, AsBindGroup, Default)]
pub struct RayTracingInfo {
    // #[uniform(0)]
    // view_uniform: ViewUniform,
    #[storage_texture(0, visibility(fragment))]
    #[reflect(ignore)]
    pub previous: Handle<Image>,
    #[uniform(1)]
    pub count: u32,
    // #[uniform(3)]
    // globals: GlobalsUniform,
    // #[texture(2)]
    // motion_view: Handle<Image>,
    #[storage(2, read_only)]
    pub sphere: Vec<GpuSphere>,
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
