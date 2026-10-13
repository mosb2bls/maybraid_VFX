//---------------------------------------------------------
// Chico canopy leaf: object-space leafy breakup on cheap-ball
// cards (planes through a centroid) + vertex sway + split light.
//
// Holes are glued to each card (kit space) so they ride with
// wind. Rim discard + interior swiss cheese are near/mid only,
// measured in ball-radii so a large card keeps the breakup as
// long as a small one at the same angular size. Far hubs stay
// solid so overlapping cards keep early-Z.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal, morph_tangent},
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals, lights},
    pbr_functions as fns,
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

// Interpolators match Bevy's `VertexOutput` locations 0–7, then local pos.
struct LeafVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
    @location(8) local_pos: vec3<f32>,
    /// Centroid distance in ball-radii (`abs / scale`).
    @location(9) view_dist: f32,
    /// Plant instance origin (flat) — canopy outward for fake occlusion.
    @location(10) @interpolate(flat) plant_origin: vec3<f32>,
}

// --------------------------------------------------------
// Hash / noise
// --------------------------------------------------------

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33, 33.33, 33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (vec3<f32>(3.0, 3.0, 3.0) - 2.0 * f0);

    let n000 = hash13(i + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash13(i + vec3<f32>(1.0, 1.0, 0.0));

    let n001 = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash13(i + vec3<f32>(1.0, 1.0, 1.0));

    let nx00 = mix(n000, n100, f.x);
    let nx10 = mix(n010, n110, f.x);
    let nx01 = mix(n001, n101, f.x);
    let nx11 = mix(n011, n111, f.x);

    let nxy0 = mix(nx00, nx10, f.y);
    let nxy1 = mix(nx01, nx11, f.y);

    return mix(nxy0, nxy1, f.z);
}

/// Two-octave FBM — near and mid cheese (4 octaves do not read under overlap).
fn fbm_3d_2(p: vec3<f32>) -> f32 {
    let a = value_noise_3d(p) * 0.5;
    let b = value_noise_3d(p * 2.03) * 0.25;
    return (a + b) / 0.75;
}

// Cheese / sway bands in ball-radii. `view_dist` is `abs / scale`
// so a unit kit cheeses to MID meters and a larger card keeps the
// same angular window. An absolute-meter floor made big banyan
// cards go rectangular while they still filled the screen.
const LEAF_MID_DIST: f32 = 80.0;
const LEAF_SWAY_CUT_DIST: f32 = 60.0;
/// Blot radius ceiling (`0.52 + 0.38`); outside this, skip hole FBM.
const LEAF_RIM_CUT: f32 = 0.92;

fn instance_scale(world_from_local: mat4x4<f32>) -> f32 {
    return max(
        length(world_from_local[0].xyz),
        max(length(world_from_local[1].xyz), length(world_from_local[2].xyz)),
    );
}

fn canopy_sway(
    local_pos: vec3<f32>,
    world_normal: vec3<f32>,
    centroid: vec3<f32>,
    scale: f32,
    view_dist: f32,
) -> vec3<f32> {
    if (view_dist >= LEAF_SWAY_CUT_DIST) {
        return vec3<f32>(0.0);
    }
    let r = min(length(local_pos), 1.0);
    let t = globals.time;
    let phase = dot(centroid, vec3<f32>(0.07, 0.03, 0.11));
    let gust = sin(t * 0.85 + phase) * sin(t * 0.31 + phase * 1.7);
    let flutter = sin(t * 2.15 + local_pos.x * 3.0 + phase);
    let wind_dir = vec3<f32>(0.92, 0.08, 0.38);
    var offset = wind_dir * (gust * r * 0.08 * scale);
    offset += world_normal * (flutter * r * 0.028 * scale);
    return offset;
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

// --------------------------------------------------------
// Vertex: Bevy mesh transform + rim-weighted wind
// --------------------------------------------------------

@vertex
fn vertex(vertex_no_morph: Vertex) -> LeafVertexOutput {
    var out: LeafVertexOutput;
    out.local_pos = vec3<f32>(0.0, 0.0, 0.0);
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);
    out.view_dist = 0.0;
    out.plant_origin = vec3<f32>(0.0, 0.0, 0.0);

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    out.plant_origin = mesh_world_from_local[3].xyz;

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

#ifdef VERTEX_POSITIONS
#ifdef VERTEX_COLORS
    // Merged kits: COLOR.xyz is pre-bake unit-kit position; COLOR.w is part scale.
    // POSITION is collection space. Recover per-ball sway (not the tree instance).
    let kit_local = vertex.color.xyz;
    let part_scale = max(vertex.color.w, 1e-4);
    let ball_local = vertex.position - kit_local * part_scale;
    let centroid = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(ball_local, 1.0),
    ).xyz;
    let scale = instance_scale(mesh_world_from_local) * part_scale;
#else
    let kit_local = vertex.position;
    let centroid = mesh_world_from_local[3].xyz;
    let scale = instance_scale(mesh_world_from_local);
#endif
    out.local_pos = kit_local;
    // Placement may be baked into vertices (grove flatten). LOD is
    // angular (centroid meters / ball radius) so large cards keep
    // rim + interior cheese as long as small cards of the same
    // apparent size. Far path still skips discard for early-Z.
    let abs_dist = length(centroid - view.world_position);
    out.view_dist = abs_dist / max(scale, 1e-4);
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.world_position += vec4<f32>(
        canopy_sway(
            kit_local,
            out.world_normal,
            centroid,
            scale,
            out.view_dist,
        ),
        0.0,
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index
    );
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index,
        mesh_world_from_local[3],
    );
#endif

    return out;
}

// --------------------------------------------------------
// Fragment
// --------------------------------------------------------

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: LeafVertexOutput,
) -> @location(0) vec4<f32> {
    let local_pos = mesh.local_pos;
    let r = saturate(length(local_pos));
    let view_dist = mesh.view_dist;
    let cheese = view_dist < LEAF_MID_DIST;

    var radial_alpha = 1.0;
    var hole_alpha = 1.0;
    var tint_noise = 0.5;

    if (cheese) {
        // Noisy rim + interior bites. Far skips both so cheap-ball hubs
        // stay opaque (r > 0.92 was punching clear color through distant crowns).
        if (r > LEAF_RIM_CUT) {
            discard;
        }
        let blot = 0.52 + 0.38 * value_noise_3d(local_pos * 2.4);
        let rim_w = max(fwidth(r) * 2.0, 0.08);
        radial_alpha = smoothstep(0.0, rim_w, blot - r);
        if (radial_alpha < 0.08) {
            discard;
        }
        let hole = fbm_3d_2(local_pos * 3.25) * 0.62 + value_noise_3d(local_pos * 8.5) * 0.38;
        let hub = 1.0 - smoothstep(0.22, 0.62, r);
        let threshold = mix(0.22, 0.52, 1.0 - hub);
        let fw = max(fwidth(hole) * 1.35, 0.01);
        hole_alpha = smoothstep(threshold - fw, threshold + fw, hole);
        tint_noise = value_noise_3d(local_pos * 1.75);
        if (hole_alpha * radial_alpha < 0.08) {
            discard;
        }
    } else {
        let field = value_noise_3d(local_pos * 3.25);
        hole_alpha = mix(0.85, 1.0, field);
        tint_noise = field;
    }

    let alpha = hole_alpha * radial_alpha;
    let base_rgb = vec3<f32>(base_color.x, base_color.y, base_color.z);
    // Hue wobble only — old 0.82/0.72 ramps read as dirt, not leaf.
    let warm_cool = mix(
        vec3<f32>(0.94, 1.02, 0.96),
        vec3<f32>(1.08, 1.04, 0.94),
        tint_noise,
    );
    let brightness = mix(0.94, 1.08, tint_noise);
    let wash = mix(0.90, 1.0, saturate(min(alpha, radial_alpha) * 1.25));
    let albedo = base_rgb * warm_cool * brightness * wash;

    let prepared_normal = fns::prepare_world_normal(
        mesh.world_normal,
        true,
        is_front,
    );
    let n = normalize(mix(prepared_normal, vec3<f32>(0.0, 1.0, 0.0), 0.15));
    var ndl = 0.55;
    if (lights.n_directional_lights > 0u) {
        let L = lights.directional_lights[0].direction_to_light;
        ndl = saturate(dot(n, L));
    }
    // Ambient brightness is 520 at golden and ~20 at night. Scale the
    // cartoon sky bounce so foliage is not a daylight stamp after dark.
    let day = saturate((dot(lights.ambient_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) - 20.0) / 600.0);
    let sun = ndl * mix(0.22, 0.95, day) + mix(0.02, 0.14, day);
    let sky = mix(0.06, 0.38, day) + mix(0.04, 0.17, day) * saturate(n.y);
    let sky_rgb = mix(vec3<f32>(0.16, 0.08, 0.30), vec3<f32>(0.78, 0.88, 1.0), day);
    // Fake canopy shadow: inward faces + puff hubs. Fade out with distance
    // so far cards stay green instead of reading as black.
    let outward = mesh.world_position.xyz - mesh.plant_origin;
    let olen = length(outward);
    var occ = mix(0.72, 1.0, r);
    if (olen > 1e-3) {
        let shade = saturate(dot(n, outward / olen));
        occ *= mix(0.68, 1.0, shade * shade);
    }
    occ = mix(occ, 1.0, saturate((view_dist - 32.0) / 48.0));
    let lifted = (albedo * sun + albedo * sky * sky_rgb) * occ;
    let fogged = with_distance_fog(
        vec4<f32>(lifted, alpha),
        mesh.world_position.xyz,
        mesh.position.xy,
    );

    return tone_mapping(fogged, view.color_grading);
}
