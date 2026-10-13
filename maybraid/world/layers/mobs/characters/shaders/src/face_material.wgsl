//---------------------------------------------------------
// Crozon face: painted lid wrap + painted lip idle.
// Kind: 0 eye, 1 mouth. Phase = globals.time + instance seed.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal, morph_tangent},
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct FaceMaterialUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    kind: u32,
    flags: u32,
    _pad: vec2<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: FaceMaterialUniform;

const KIND_EYE: u32 = 0u;
const KIND_MOUTH: u32 = 1u;

struct FaceVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
    @location(8) local_pos: vec3<f32>,
    @location(9) blink: f32,
}

fn hash11(n: f32) -> f32 {
    let x = fract(n * 0.1031);
    return fract(x * (x + 33.33));
}

/// Instance-stable. Do not hash world origin — idle / locomotion would retune the phase every frame.
fn face_seed(instance_index: u32) -> f32 {
    return hash11(f32(instance_index) * 0.618 + 0.17 + material.noise.z * 0.03);
}

fn face_smoothstep(t: f32) -> f32 {
    let u = saturate(t);
    return u * u * (3.0 - 2.0 * u);
}

fn blink_pulse(t: f32, start: f32, close: f32, hold: f32, open: f32) -> f32 {
    let u = t - start;
    if u <= 0.0 || u >= close + hold + open {
        return 0.0;
    }
    if u < close {
        return face_smoothstep(u / close);
    }
    if u < close + hold {
        return 1.0;
    }
    return 1.0 - face_smoothstep((u - close - hold) / open);
}

fn blink_depth(cycle: f32, seed: f32) -> f32 {
    let h = hash11(cycle * 1.73 + seed * 9.1 + 2.4);
    if h < 0.58 {
        return 0.24 + h * 0.38;
    }
    if h < 0.86 {
        return 0.52 + (h - 0.58);
    }
    return 1.0;
}

/// Designed 1D envelope. Do not replace with raw 4D noise.
/// Peak depth varies per cycle so most blinks are slighter than a full slit.
fn blink_envelope(time: f32, seed: f32) -> f32 {
    let period = 4.2 + seed * 2.0;
    let phase_time = time + seed * 17.0;
    let t = fract(phase_time / period);
    let cycle = floor(phase_time / period);
    let close = 0.045;
    let hold = 0.020;
    let open = 0.10;
    var shape = blink_pulse(t, 0.0, close, hold, open);
    if seed > 0.62 {
        let second_start = close + hold + open + 0.025;
        shape = max(shape, blink_pulse(t, second_start, close, hold, open));
    }
    return shape * blink_depth(cycle, seed);
}

fn disk_mask(r: f32, radius: f32, feather: f32) -> f32 {
    return saturate((radius - r) / max(feather, 1e-4));
}

/// `0` = globe, `1` = lid. Almond aperture; blink grows lids toward the midline.
fn lid_wrap(xy: vec2<f32>, blink: f32) -> f32 {
    let taper = sqrt(saturate(1.0 - (xy.x / 0.52) * (xy.x / 0.52)));
    let upper_edge = (0.15 * (1.0 - blink) - 0.06 * blink) * taper;
    let lower_edge = (0.30 * (1.0 - blink) - 0.06 * blink) * taper;
    let upper = face_smoothstep((xy.y - upper_edge) / 0.04);
    let lower = face_smoothstep((-xy.y - lower_edge) / 0.04);
    return max(upper, lower);
}

fn pupil_mask(q: vec2<f32>, shape: f32) -> f32 {
    let round_m = disk_mask(length(q), 0.14, 0.02);
    let slit_m = disk_mask(length(vec2<f32>(q.x / 0.05, q.y / 0.18)), 1.0, 0.12);
    return mix(round_m, slit_m, saturate(shape));
}

/// Authored eyes face +Z; iris is an XY disk around the origin (~0.55 radius).
fn eye_look(local_pos: vec3<f32>, blink: f32) -> vec3<f32> {
    let iris = material.colors[0].xyz;
    let pupil = material.colors[1].xyz;
    let sclera = material.colors[2].xyz;
    let iris_secondary = material.colors[3].xyz;
    let lid = material.colors[4].xyz;
    let highlight = material.colors[5].xyz;
    let shape = material.scalars[0].x;

    let q = local_pos.xy;
    let r = length(q);
    let ang = atan2(q.y, q.x);
    let spokes = 0.88 + 0.12 * sin(ang * 11.0 + r * 18.0);
    let radial = saturate((0.36 - r) / 0.22);
    let iris_col = mix(iris_secondary, iris, radial) * spokes;

    var tint = sclera;
    tint = mix(tint, iris_col, disk_mask(r, 0.36, 0.03));
    tint = mix(tint, pupil, pupil_mask(q, shape));

    let lid_m = lid_wrap(q, blink);
    let crease = lid_m * (1.0 - lid_m) * 2.0;
    let lid_col = mix(lid, lid * vec3<f32>(0.72, 0.62, 0.58), crease);
    let open = 1.0 - lid_m;
    let catch_a = disk_mask(length(q - vec2<f32>(-0.07, 0.09)), 0.055, 0.02);
    let catch_b = disk_mask(length(q - vec2<f32>(0.06, -0.04)), 0.024, 0.012);
    tint = mix(tint, highlight, (catch_a * 0.85 + catch_b * 0.45) * open);
    tint = mix(tint, lid_col, lid_m);
    return tint;
}

/// Split the lips away from the midline. Corners stay pinched.
fn mouth_deform(local_pos: vec3<f32>, open: f32) -> vec3<f32> {
    let taper = sqrt(saturate(1.0 - (local_pos.x / 0.88) * (local_pos.x / 0.88)));
    let split = open * 0.20 * taper;
    let side = sign(local_pos.y);
    let lower = saturate(-local_pos.y) * open * 0.06 * taper;
    return vec3<f32>(0.0, side * split - lower, open * 0.05 * taper);
}

/// Stable rest crease; occasional part. Rate from scalars[0].x (0 = default 2.2).
fn mouth_open_envelope(time: f32, seed: f32) -> f32 {
    var rate = material.scalars[0].x;
    if rate < 1e-4 {
        rate = 2.2;
    }
    let rest = 0.08;
    let period = (8.5 + seed * 3.0) / rate;
    let phase_time = time + seed * 11.0;
    let t = fract(phase_time / period);
    let cycle = floor(phase_time / period);
    let pulse = blink_pulse(t, 0.0, 0.08, 0.10, 0.16);
    let h = hash11(cycle * 2.1 + seed * 6.3);
    var depth = 0.40;
    if h >= 0.72 && h < 0.92 {
        depth = 0.65;
    } else if h >= 0.92 {
        depth = 0.95;
    }
    return saturate(rest + pulse * depth);
}

/// `0` = lip flesh, `1` = opening. Rest crease; `open` widens it.
fn lip_opening(xy: vec2<f32>, open: f32) -> f32 {
    let taper = sqrt(saturate(1.0 - (xy.x / 0.88) * (xy.x / 0.88)));
    let half = (0.028 + open * 0.22) * taper;
    return face_smoothstep((half - abs(xy.y)) / 0.03);
}

/// Authored mouth is a wide XY lip slab (~±1 x, ±0.52 y).
fn mouth_look(local_pos: vec3<f32>, open: f32) -> vec3<f32> {
    let lip = material.colors[0].xyz;
    let crease = material.colors[1].xyz;
    let interior = material.colors[2].xyz;
    let highlight = material.colors[3].xyz;
    let teeth = material.colors[4].xyz;

    let q = local_pos.xy;
    let opening = lip_opening(q, open);
    let half = (0.028 + open * 0.22) * sqrt(saturate(1.0 - (q.x / 0.88) * (q.x / 0.88)));
    let teeth_m = opening * face_smoothstep((0.38 * half - abs(q.y)) / 0.014)
        * saturate(1.0 - abs(q.x) / 0.55) * saturate((open - 0.12) / 0.18);
    let corner = saturate((abs(q.x) - 0.52) / 0.28);
    let gloss = exp(-((q.x - 0.06) * (q.x - 0.06) / 0.07 + (q.y + 0.16) * (q.y + 0.16) / 0.035))
        * saturate(-q.y * 3.0) * (1.0 - opening);

    var tint = mix(lip, lip * vec3<f32>(0.78, 0.62, 0.58), corner);
    tint = mix(tint, highlight, gloss * 0.55);
    tint = mix(tint, crease, opening);
    tint = mix(tint, interior, opening * saturate(0.35 + open * 1.1));
    tint = mix(tint, teeth, teeth_m);
    return tint;
}

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;

    let weight_count = bevy_pbr::morph::layer_count(instance_index);
    for (var i: u32 = 0u; i < weight_count; i++) {
        let weight = bevy_pbr::morph::weight_at(i, instance_index);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph_position(vertex_index, i, instance_index);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph_normal(vertex_index, i, instance_index);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph_tangent(vertex_index, i, instance_index), 0.0);
#endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> FaceVertexOutput {
    var out: FaceVertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    let seed = face_seed(vertex_no_morph.instance_index);
    var amount = 0.0;
    if material.kind == KIND_EYE {
        amount = blink_envelope(globals.time, seed);
        out.local_pos = vertex.position;
    } else {
        let local = vertex.position;
        amount = mouth_open_envelope(globals.time, seed);
        vertex.position += mouth_deform(local, amount);
        out.local_pos = local;
    }
    out.blink = amount;

#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    var world_from_local = mesh_world_from_local;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index
    );
#endif
#endif

    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index
    );
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif

    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: FaceVertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    let base = material.colors[0].xyz;
    var roughness = 0.55;
    var metallic = 0.02;
    var tint = base;

    if material.kind == KIND_EYE {
        tint = eye_look(mesh.local_pos, mesh.blink);
        let lid_m = lid_wrap(mesh.local_pos.xy, mesh.blink);
        let iris_m = disk_mask(length(mesh.local_pos.xy), 0.36, 0.03) * (1.0 - lid_m);
        roughness = mix(0.42, 0.16, iris_m);
        roughness = mix(roughness, 0.72, lid_m);
        metallic = mix(0.04, 0.12, iris_m);
    } else {
        tint = mouth_look(mesh.local_pos, mesh.blink);
        let opening = lip_opening(mesh.local_pos.xy, mesh.blink);
        roughness = mix(0.58, 0.78, opening);
        metallic = mix(0.06, 0.02, opening);
    }

    pbr_input.material.base_color = vec4<f32>(tint, 1.0);
    pbr_input.material.perceptual_roughness = roughness;
    pbr_input.material.metallic = metallic;
    pbr_input.material.reflectance = vec3<f32>(0.18, 0.18, 0.18);
    pbr_input.material.flags = pbr_input.material.flags | STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let prepared_normal = fns::prepare_world_normal(mesh.world_normal, true, is_front);
    let n = normalize(prepared_normal);
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb, 1.0), view.color_grading);
}
