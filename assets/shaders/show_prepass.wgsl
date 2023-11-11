#import bevy_pbr::{
    mesh_view_bindings::globals,
    prepass_utils,
    forward_io::VertexOutput,
}

struct PrepassSettings {
    show_depth: u32,
    show_normals: u32,
    show_motion: u32,
}

@fragment
fn fragment(in: VertexOutput,) -> @location(0) vec4<f32> {
    let depth = prepass_utils::prepass_depth(in.position, 1u);
    let normal = prepass_utils::prepass_normal(in.position, 1u);
    let motion_vector = prepass_utils::prepass_motion_vector(in.position, 1u);
    return vec4(motion_vector, depth, 1.0);
}
