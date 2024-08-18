use bevy::{
    core_pipeline::prepass::node::PrepassNode,
    math::bounding::Bounded3d,
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        primitives::Aabb,
        render_graph::{RenderGraphApp, RenderLabel, RenderSubGraph, ViewNodeRunner},
        render_resource::{AsBindGroup, ShaderType},
        Extract, RenderApp,
    },
};
use itertools::Itertools;

use crate::{node::RayTracingPassNode, pipeline::RayTracingPipeline};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct PrepassLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct RayTracingLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
pub struct RayTracingGraph;

pub struct RayTracingPlugin;

impl Plugin for RayTracingPlugin {
    fn build(&self, app: &mut App) {
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

#[derive(Reflect, Default, Debug, Clone, ShaderType)]
pub struct Triangle {
    indices: [u32; 3],
}

#[derive(Reflect, Default, Debug, Clone, ShaderType)]
pub struct MeshInfo {
    first_tri: u32,
    tri_count: u32,
    material: u32,
    aabb_min: Vec3,
    aabb_max: Vec3,
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
            aabb_min: aabb.min.into(),
            aabb_max: aabb.max.into(),
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
