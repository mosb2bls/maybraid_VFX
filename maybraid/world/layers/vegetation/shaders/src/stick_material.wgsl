//---------------------------------------------------------
// Chico stick / bark: PBR + screen-space edge darkening from world normals.
// Adapted from `playgrounds/objects/assets/shaders/edge_material.wgsl`.
//---------------------------------------------------------
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_types::{PbrInput, pbr_input_new},
    pbr_functions as fns,
    pbr_bindings,
}
#import bevy_core_pipeline::tonemapping::tone_mapping
#ifdef DISTANCE_FOG
#import bevy_pbr::mesh_view_bindings::fog
#endif

fn with_distance_fog(color: vec4<f32>, world_position: vec3<f32>, frag_xy: vec2<f32>) -> vec4<f32> {
#ifdef DISTANCE_FOG
    return fns::apply_fog(fog, color, world_position, view.world_position.xyz, frag_xy);
#else
    return color;
#endif
}


@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;


fn fwidth3(v: vec3<f32>) -> vec3<f32> {
    return abs(dpdx(v)) + abs(dpdy(v));
}


@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {

    var pbr_input: PbrInput = pbr_input_new();

    pbr_input.material.base_color = base_color;

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        true,
        is_front,
    );
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.N = normalize(pbr_input.world_normal);
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let lit_color = fns::apply_pbr_lighting(pbr_input);

    let n = normalize(mesh.world_normal);
    let dN = fwidth3(n);
    let edge_val = length(dN);

    let edge = smoothstep(0.0001, 0.05, edge_val);

    let intensity = 1.0 - edge;

    let shaded = lit_color.rgb * intensity;
    let fogged = with_distance_fog(
        vec4<f32>(shaded, 1.0),
        mesh.world_position.xyz,
        mesh.position.xy,
    );

    return tone_mapping(fogged, view.color_grading);
}
