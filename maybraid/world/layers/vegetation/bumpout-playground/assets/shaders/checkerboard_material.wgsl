//---------------------------------------------------------
// Required imports for a PBR fragment shader in Bevy 0.17
//---------------------------------------------------------
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
    pbr_bindings,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

//---------------------------------------------------------
// Material uniforms
//---------------------------------------------------------
@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> checker_size_m: f32;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> color1: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> color2: vec4<f32>;

//---------------------------------------------------------
// Fragment Shader
//---------------------------------------------------------
@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();

    let world_pos = mesh.world_position;
    let checker_x = floor(world_pos.x / checker_size_m);
    let checker_z = floor(world_pos.z / checker_size_m);
    let checker_sum = checker_x + checker_z;
    let checker = checker_sum - 2.0 * floor(checker_sum / 2.0);

    let base_color = select(color2, color1, checker < 1.0);
    pbr_input.material.base_color = base_color;

    let double_sided = (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        double_sided,
        is_front,
    );
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.N = normalize(pbr_input.world_normal);
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    let output = tone_mapping(vec4<f32>(lit_color.rgb, 1.0), view.color_grading);
    return output;
}
