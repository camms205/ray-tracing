use bevy::{
    asset::load_internal_asset,
    core_pipeline::{
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        prepass::{node::PrepassNode, ViewPrepassTextures},
    },
    ecs::query::QueryItem,
    math::bounding::Bounded3d,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_resource::ExtractResource,
        globals::{GlobalsBuffer, GlobalsUniform},
        primitives::Aabb,
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
        texture::{BevyDefault, FallbackImage, GpuImage},
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        Extract, RenderApp,
    },
};
use itertools::Itertools;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct PrepassLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct RayTracingLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
pub struct RayTracingGraph;

const RAY_TRACING_UTILS_HANDLE: Handle<Shader> =
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
        app.insert_resource(Msaa::Off);
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_systems(ExtractSchedule, prepare_meshinfo)
                .add_render_sub_graph(RayTracingGraph)
                .add_render_graph_node::<ViewNodeRunner<PrepassNode>>(RayTracingGraph, PrepassLabel)
                .add_render_graph_node::<ViewNodeRunner<RayTracingPassNode>>(
                    RayTracingGraph,
                    RayTracingLabel,
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
                world.resource::<RenderAssets<GpuImage>>(),
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
            label: Some("ray_tracing_render_pass"),
            color_attachments: &[Some(target.out_texture_color_attachment(None))],
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
pub struct GpuSphere {
    pub center: Vec3,
    pub radius: f32,
    pub color: LinearRgba,
    pub light: Vec3,
}

impl GpuSphere {
    pub fn new(center: Vec3, radius: f32, color: LinearRgba, light: Vec3) -> GpuSphere {
        Self {
            center,
            radius,
            color,
            light,
        }
    }
}

#[derive(Reflect, Default, Debug, Clone, ShaderType)]
pub struct Triangle {
    indices: [u32; 3],
}

#[derive(Reflect, Default, Debug, Clone, ShaderType)]
pub struct MeshInfo {
    first_tri: u32,
    tri_count: u32,
    material: u32,
    aabb_left_bottom: Vec3,
    aabb_right_top: Vec3,
}

#[derive(Reflect, Default, Debug, Clone, ShaderType)]
pub struct SimpleMaterial {
    pub color: LinearRgba,
}

impl From<Srgba> for SimpleMaterial {
    fn from(value: Srgba) -> Self {
        Self {
            color: value.into(),
        }
    }
}

impl From<LinearRgba> for SimpleMaterial {
    fn from(value: LinearRgba) -> Self {
        Self { color: value }
    }
}

#[derive(Clone, Resource, ExtractResource, AsBindGroup, Default)]
pub struct RayTracingInfo {
    #[uniform(0)]
    pub count: u32,
    #[storage(1, read_only)]
    pub triangles: Vec<Triangle>,
    #[storage(2, read_only)]
    pub meshes: Vec<MeshInfo>,
    #[storage(3, read_only)]
    pub vertices: Vec<[Vec3; 2]>,
    #[storage(4, read_only)]
    pub materials: Vec<SimpleMaterial>,
}

pub fn prepare_meshinfo(
    mut commands: Commands,
    query: Extract<
        Query<(
            &Handle<Mesh>,
            &Handle<StandardMaterial>,
            &GlobalTransform,
            &Aabb,
        )>,
    >,
    mesh_assets: Extract<Res<Assets<Mesh>>>,
    material_assets: Extract<Res<Assets<StandardMaterial>>>,
    ray_tracing_info: Extract<Res<RayTracingInfo>>,
) {
    let mut ray_tracing_info = ray_tracing_info.clone();
    let mut vertices = vec![];
    let mut triangles = vec![];
    let mut mesh_info = vec![];
    let mut materials = vec![];
    // let materials: Vec<(AssetId<StandardMaterial>, &StandardMaterial)> = material_assets.iter().collect()
    let mut material_index = 0;
    for (mesh_handle, material_handle, transform, aabb) in query.iter() {
        let mesh: &Mesh = mesh_assets.get(mesh_handle).unwrap();
        let material: &StandardMaterial = material_assets.get(material_handle).unwrap();
        materials.push(material.base_color.to_linear().into());
        let (Some(pos), Some(norm)) = (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION),
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL),
        ) else {
            println!("Mesh missing attribute");
            continue;
        };
        let vertices_len = vertices.len();
        let pos = pos.as_float3().unwrap().iter();
        let norm = norm.as_float3().unwrap().iter();
        let (_scale, rotate, translate) = transform.to_scale_rotation_translation();
        vertices.extend(
            pos.zip(norm)
                .map(|(a, b)| {
                    let pos = transform.transform_point(a.to_owned().into());
                    // let norm = transform.transform_point(b.to_owned().into());
                    let b: Vec3 = b.to_owned().into();
                    let norm = rotate * b;
                    [pos, norm]
                })
                .collect::<Vec<[Vec3; 2]>>(),
        );
        let len = mesh.indices().unwrap().len();
        let indices: Vec<usize> = mesh.indices().unwrap().iter().collect();
        let triangle_len = triangles.len();
        let cube = Cuboid::from_size(Vec3::from(aabb.half_extents) * 2.0);
        let aabb = cube.aabb_3d(translate, rotate);
        mesh_info.push(MeshInfo {
            first_tri: triangle_len as u32,
            tri_count: len as u32 / 3,
            material: material_index,
            aabb_left_bottom: aabb.min.into(),
            aabb_right_top: aabb.max.into(),
        });
        material_index += 1;
        indices
            .iter()
            .map(|i| (i + vertices_len) as u32)
            .tuples::<(_, _, _)>()
            .for_each(|(a, b, c)| triangles.push(Triangle { indices: [a, b, c] }));
    }
    ray_tracing_info.triangles = triangles;
    ray_tracing_info.meshes = mesh_info;
    ray_tracing_info.vertices = vertices;
    ray_tracing_info.materials = materials;
    commands.insert_resource(ray_tracing_info);
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
