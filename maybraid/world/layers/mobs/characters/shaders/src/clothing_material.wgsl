//---------------------------------------------------------
// Crozon clothing: palette tint, look kind, tiny hem sway.
//
// Kind: 0 cloth, 1 space suit, 2 tattered, 3 hawaiian,
//       4 wizard veins, 5 glitter, 6 scales, 7 lava veins, 8 brushed metal.
//
// Looks stay matte and worn. Ribbing and a short-range implicit
// POM (Durham solver, UV jacobian) sit on weave, seams, scales,
// and brush grain. Albedo stays on mesh UV; the march only
// shades cavity and tilts the normal. No silhouette or
// self-shadow ray.
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

struct ClothingMaterialUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    kind: u32,
    flags: u32,
    _pad: vec2<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: ClothingMaterialUniform;

const KIND_CLOTH: u32 = 0u;
const KIND_SPACE_SUIT: u32 = 1u;
const KIND_TATTERED: u32 = 2u;
const KIND_HAWAIIAN: u32 = 3u;
const KIND_WIZARDS_VEINS: u32 = 4u;
const KIND_GLITTER: u32 = 5u;
const KIND_SCALES: u32 = 6u;
const KIND_LAVA_VEINS: u32 = 7u;
const KIND_BRUSHED_METAL: u32 = 8u;
const FLAG_NO_SWAY: u32 = 1u;

struct ClothingVertexOutput {
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
}

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn hash21(p: vec2<f32>) -> f32 {
    return hash13(vec3<f32>(p, 17.13));
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (3.0 - 2.0 * f0);

    let n000 = hash13(i);
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
    return mix(mix(nx00, nx10, f.y), mix(nx01, nx11, f.y), f.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var a = 0.5;
    var s = 0.0;
    var q = p;
    for (var i = 0; i < 4; i++) {
        s += a * value_noise_3d(q);
        q *= 2.03;
        a *= 0.5;
    }
    return s;
}

fn clothing_sway(local_pos: vec3<f32>, world_normal: vec3<f32>) -> vec3<f32> {
    let hem = saturate(-local_pos.y * 0.85 + 0.15);
    let t = globals.time;
    let phase = dot(local_pos, vec3<f32>(2.7, 1.1, 3.4));
    let gust = sin(t * 1.25 + phase) * 0.006 + sin(t * 2.1 + phase * 1.7) * 0.0025;
    let flutter = sin(t * 3.4 + local_pos.x * 8.0) * 0.0015;
    var offset = vec3<f32>(0.92, 0.05, 0.38) * (gust * (0.25 + 0.75 * hem));
    offset += world_normal * (flutter * (0.4 + 0.6 * hem));
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

@vertex
fn vertex(vertex_no_morph: Vertex) -> ClothingVertexOutput {
    var out: ClothingVertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

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

    out.local_pos = vertex.position;
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    if (material.flags & FLAG_NO_SWAY) == 0u {
        out.world_position += vec4<f32>(clothing_sway(vertex.position, out.world_normal), 0.0);
    }
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

fn look_coord(mesh: ClothingVertexOutput) -> vec2<f32> {
#ifdef VERTEX_UVS_A
    return mesh.uv;
#else
    return mesh.world_position.xz * 0.55 + mesh.local_pos.xy * 0.35;
#endif
}

/// Fine basket weave plus a coarser vertical rib.
fn cloth_threads(uv: vec2<f32>) -> vec3<f32> {
    let count = 96.0;
    let warp_id = floor(uv.x * count);
    let weft_id = floor(uv.y * count);
    let warp_f = fract(uv.x * count);
    let weft_f = fract(uv.y * count);
    let warp_ridge = pow(1.0 - abs(warp_f * 2.0 - 1.0), 0.55);
    let weft_ridge = pow(1.0 - abs(weft_f * 2.0 - 1.0), 0.55);
    let warp_on_top = step(0.5, fract((warp_id + weft_id) * 0.5));
    let thread = mix(weft_ridge, warp_ridge, warp_on_top);
    let rib = pow(1.0 - abs(fract(uv.x * 22.0) * 2.0 - 1.0), 0.62);
    let ridge = mix(thread, rib, 0.42);
    let along = mix(uv.x, uv.y, warp_on_top);
    let fiber = value_noise_3d(vec3<f32>(along * 220.0, ridge * 12.0, warp_id * 0.13 + weft_id * 0.07));
    let slub = 0.90 + 0.14 * value_noise_3d(vec3<f32>(uv * 36.0, 3.1));
    return vec3<f32>(ridge, fiber, slub);
}

fn cloth_look(base: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    let t = cloth_threads(uv);
    let groove = mix(0.78, 1.06, t.x);
    let fiber = mix(0.94, 1.04, t.y);
    let tint = base * groove * fiber * t.z;
    return vec4<f32>(tint, mix(0.94, 0.86, t.x));
}

fn space_suit_look(base: vec3<f32>, uv: vec2<f32>, _n: vec3<f32>) -> vec4<f32> {
    let seam = smoothstep(0.44, 0.49, max(
        abs(fract(uv.x * 5.0) - 0.5),
        abs(fract(uv.y * 6.5) - 0.5),
    ));
    let wear = 0.94 + 0.08 * value_noise_3d(vec3<f32>(uv * 14.0, 2.2));
    let tint = mix(base * wear, base * 0.62, seam);
    return vec4<f32>(tint, mix(0.62, 0.78, seam));
}

/// Worn cloth: same thread pattern, stained and frayed. Holes are discarded
/// in the fragment entry (Opaque ignores alpha; helpers cannot punch through).
fn tattered_look(base: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    let cloth = cloth_look(base, uv);
    let wear = fbm(vec3<f32>(uv * 5.2, 2.2));
    let stain = value_noise_3d(vec3<f32>(uv * 3.6, 2.7));
    let dirt = mix(cloth.xyz, cloth.xyz * vec3<f32>(0.55, 0.50, 0.42), stain * 0.35);
    let fray = smoothstep(0.62, 0.84, wear);
    let tint = mix(dirt, dirt * 0.78, fray);
    return vec4<f32>(tint, mix(cloth.w, 0.96, fray));
}

/// UV-space moth-eaten blotches, same mix as Chico leaf interior cheese.
fn tatter_hole_alpha(uv: vec2<f32>) -> f32 {
    let blot = fbm(vec3<f32>(uv * 7.0, 1.4)) * 0.62
        + value_noise_3d(vec3<f32>(uv * 18.0, 6.2)) * 0.38;
    let fw = max(fwidth(blot) * 1.35, 0.01);
    return smoothstep(0.40 - fw, 0.40 + fw, blot);
}

fn hawaiian_look(base: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    let cloth = cloth_look(base, uv);
    let p = uv * vec2<f32>(3.2, 4.0);
    let cell = floor(p);
    let f = fract(p) - 0.5;
    let mark = smoothstep(0.22, 0.08, length(f * vec2<f32>(1.1, 1.4)));
    let ink = mix(base * 0.72, base * 0.55, hash21(cell));
    let tint = mix(cloth.xyz, ink, mark * 0.22);
    return vec4<f32>(tint, mix(cloth.w, 0.90, mark * 0.2));
}

fn wizards_veins_look(base: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    let cloth = cloth_look(base * 0.92, uv);
    let n1 = fbm(vec3<f32>(uv * 4.2, 2.7));
    let ridge = 1.0 - abs(n1 * 2.0 - 1.0);
    let vein = pow(saturate(ridge * 1.08), 9.0);
    let mineral = mix(base, vec3<f32>(0.42, 0.52, 0.55), 0.35);
    let tint = mix(cloth.xyz, mineral, vein * 0.45);
    return vec4<f32>(tint, mix(cloth.w, 0.80, vein));
}

fn glitter_look(base: vec3<f32>, uv: vec2<f32>, n: vec3<f32>) -> vec4<f32> {
    let cloth = cloth_look(base, uv);
    let cell = floor(uv * 64.0);
    let flake = step(0.97, hash21(cell));
    let spec = pow(saturate(dot(n, vec3<f32>(0.15, 0.85, 0.35)) * 0.5 + 0.5), 12.0);
    let tint = cloth.xyz + vec3<f32>(0.08, 0.07, 0.06) * flake * spec;
    return vec4<f32>(tint, mix(cloth.w, 0.70, flake));
}

fn lava_veins_look(base: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    let cloth = cloth_look(base * 0.55, uv);
    let n1 = fbm(vec3<f32>(uv * 4.4, 3.1));
    let ridge = 1.0 - abs(n1 * 2.0 - 1.0);
    let vein = pow(saturate(ridge * 1.1), 8.0);
    let ember = vec3<f32>(0.42, 0.16, 0.08);
    let tint = mix(cloth.xyz, ember, vein * 0.55);
    return vec4<f32>(tint, mix(0.92, 0.70, vein));
}

fn brushed_metal_look(base: vec3<f32>, uv: vec2<f32>, _n: vec3<f32>) -> vec4<f32> {
    let grain = uv.x * 42.0 + uv.y * 1.8;
    let stroke = pow(abs(sin(grain * 3.14159)), 0.45);
    let along = value_noise_3d(vec3<f32>(uv.x * 64.0, uv.y * 4.0, 1.4));
    let groove = mix(0.70, 1.04, stroke * (0.7 + 0.3 * along));
    let tint = base * groove;
    return vec4<f32>(tint, mix(0.42, 0.58, stroke));
}

fn scales_look(base: vec3<f32>, uv: vec2<f32>, _n: vec3<f32>) -> vec4<f32> {
    let p = uv * vec2<f32>(14.0, 17.0);
    let row = floor(p.y);
    let odd = step(0.5, fract(row * 0.5));
    var q = p;
    q.x += odd * 0.5;
    let cell = floor(q);
    let f = fract(q) - vec2<f32>(0.5);
    let d = length(f * vec2<f32>(1.05, 1.35));
    let rim = smoothstep(0.38, 0.50, d);
    let gap = smoothstep(0.50, 0.56, d);
    let jitter = hash21(cell);
    let shade = 0.86 + 0.10 * jitter;
    let plate = mix(base * 0.72, base * 0.98, 1.0 - rim) * shade;
    let tint = mix(base * 0.42, plate, 1.0 - gap);
    return vec4<f32>(tint, mix(0.68, 0.82, rim));
}

//---------------------------------------------------------
// Short-range UV POM. Height is 1 at the cloth face, 0 in grooves.
// March for cavity and relief normal only — do not resample look.
//---------------------------------------------------------

struct ReliefStyle {
    depth_m: f32,
    width_uv: f32,
    dark: f32,
}

const POM_FADE_START_M: f32 = 8.0;
const POM_FADE_END_M: f32 = 22.0;
const POM_MIN_STEPS: i32 = 8;
const POM_MAX_STEPS: i32 = 24;
const POM_REFINE_STEPS: i32 = 3;
const RELIEF_NORMAL_EPS_UV: f32 = 0.0015;

fn relief_style() -> ReliefStyle {
    switch material.kind {
        case KIND_SPACE_SUIT: {
            return ReliefStyle(0.0024, 0.018, 0.72);
        }
        case KIND_SCALES: {
            return ReliefStyle(0.0032, 0.016, 0.62);
        }
        case KIND_BRUSHED_METAL: {
            return ReliefStyle(0.0009, 0.010, 0.78);
        }
        case KIND_WIZARDS_VEINS, KIND_LAVA_VEINS: {
            return ReliefStyle(0.0014, 0.012, 0.70);
        }
        default: {
            return ReliefStyle(0.0011, 0.008, 0.82);
        }
    }
}

fn cloth_height(uv: vec2<f32>) -> f32 {
    return cloth_threads(uv).x;
}

fn suit_height(uv: vec2<f32>) -> f32 {
    let seam = max(
        abs(fract(uv.x * 5.0) - 0.5),
        abs(fract(uv.y * 6.5) - 0.5),
    );
    return 1.0 - smoothstep(0.44, 0.49, seam);
}

fn scale_height(uv: vec2<f32>) -> f32 {
    let p = uv * vec2<f32>(14.0, 17.0);
    let row = floor(p.y);
    var q = p;
    q.x += step(0.5, fract(row * 0.5)) * 0.5;
    let f = fract(q) - vec2<f32>(0.5);
    let d = length(f * vec2<f32>(1.05, 1.35));
    return 1.0 - smoothstep(0.42, 0.56, d);
}

fn metal_height(uv: vec2<f32>) -> f32 {
    let grain = uv.x * 42.0 + uv.y * 1.8;
    return pow(abs(sin(grain * 3.14159)), 0.45);
}

fn vein_height(uv: vec2<f32>) -> f32 {
    let n1 = fbm(vec3<f32>(uv * 4.3, 2.9));
    let ridge = 1.0 - abs(n1 * 2.0 - 1.0);
    return 1.0 - pow(saturate(ridge * 1.1), 8.0) * 0.55;
}

fn relief_height(uv: vec2<f32>) -> f32 {
    switch material.kind {
        case KIND_SPACE_SUIT: { return suit_height(uv); }
        case KIND_SCALES: { return scale_height(uv); }
        case KIND_BRUSHED_METAL: { return metal_height(uv); }
        case KIND_WIZARDS_VEINS, KIND_LAVA_VEINS: { return vein_height(uv); }
        default: { return cloth_height(uv); }
    }
}

fn safe_inv(x: f32) -> f32 {
    if (abs(x) < 1e-10) {
        return 0.0;
    }
    return 1.0 / x;
}

fn world_ray_to_uv(ray: vec3<f32>, p: vec3<f32>, uv: vec2<f32>) -> vec2<f32> {
    let dpx = dpdx(p);
    let dpy = dpdy(p);
    let dux = dpdx(uv);
    let duy = dpdy(uv);
    let a00 = dot(dpx, dpx);
    let a01 = dot(dpx, dpy);
    let a11 = dot(dpy, dpy);
    let det = a00 * a11 - a01 * a01;
    let bx = dot(ray, dpx);
    let by = dot(ray, dpy);
    let inv = safe_inv(det);
    let sx = (a11 * bx - a01 * by) * inv;
    let sy = (a00 * by - a01 * bx) * inv;
    return dux * sx + duy * sy;
}

fn uv_gradient_to_world(g: vec2<f32>, p: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let dpx = dpdx(p);
    let dpy = dpdy(p);
    let dux = dpdx(uv);
    let duy = dpdy(uv);
    let det = dux.x * duy.y - dux.y * duy.x;
    let inv = safe_inv(det);
    let dpdu = (dpx * duy.y - dpy * dux.y) * inv;
    let dpdv = (dpy * dux.x - dpx * duy.x) * inv;
    return g.x * dpdu + g.y * dpdv;
}

struct ReliefHit {
    uv: vec2<f32>,
    height: f32,
    t: f32,
}

fn trace_relief(uv0: vec2<f32>, ray_uv: vec2<f32>, style: ReliefStyle) -> ReliefHit {
    let travel = length(ray_uv);
    let requested = ceil(travel / max(style.width_uv * 0.25, 1e-5));
    let steps = i32(clamp(requested, f32(POM_MIN_STEPS), f32(POM_MAX_STEPS)));
    let h0 = relief_height(uv0);
    if (h0 >= 0.995) {
        return ReliefHit(uv0, h0, 0.0);
    }

    var lo = 0.0;
    var hi = 1.0;
    var f_lo = h0 - 1.0;
    var f_hi = 0.0;

    for (var i = 1; i <= POM_MAX_STEPS; i = i + 1) {
        if (i > steps) {
            break;
        }
        let t = f32(i) / f32(steps);
        let f = relief_height(uv0 + ray_uv * t) - (1.0 - t);
        if (f >= 0.0) {
            hi = t;
            f_hi = f;
            break;
        }
        lo = t;
        f_lo = f;
    }

    for (var i = 0; i < POM_REFINE_STEPS; i = i + 1) {
        let mid = 0.5 * (lo + hi);
        let f = relief_height(uv0 + ray_uv * mid) - (1.0 - mid);
        if (f >= 0.0) {
            hi = mid;
            f_hi = f;
        } else {
            lo = mid;
            f_lo = f;
        }
    }

    let fraction = clamp(-f_lo / max(f_hi - f_lo, 1e-7), 0.0, 1.0);
    let t = mix(lo, hi, fraction);
    let uv_hit = uv0 + ray_uv * t;
    return ReliefHit(uv_hit, relief_height(uv_hit), t);
}

fn relief_gradient(uv: vec2<f32>) -> vec2<f32> {
    let e = RELIEF_NORMAL_EPS_UV;
    let dx = relief_height(uv + vec2<f32>(e, 0.0))
        - relief_height(uv - vec2<f32>(e, 0.0));
    let dy = relief_height(uv + vec2<f32>(0.0, e))
        - relief_height(uv - vec2<f32>(0.0, e));
    return vec2<f32>(dx, dy) / (2.0 * e);
}

fn kind_look(
    kind: u32,
    base: vec3<f32>,
    uv: vec2<f32>,
    n: vec3<f32>,
) -> vec4<f32> {
    switch kind {
        case KIND_SPACE_SUIT: { return space_suit_look(base, uv, n); }
        case KIND_TATTERED: { return tattered_look(base, uv); }
        case KIND_HAWAIIAN: { return hawaiian_look(base, uv); }
        case KIND_WIZARDS_VEINS: { return wizards_veins_look(base, uv); }
        case KIND_GLITTER: { return glitter_look(base, uv, n); }
        case KIND_SCALES: { return scales_look(base, uv, n); }
        case KIND_LAVA_VEINS: { return lava_veins_look(base, uv); }
        case KIND_BRUSHED_METAL: { return brushed_metal_look(base, uv, n); }
        default: { return cloth_look(base, uv); }
    }
}

fn kind_metallic(kind: u32) -> f32 {
    switch kind {
        case KIND_SPACE_SUIT: { return 0.12; }
        case KIND_TATTERED: { return 0.02; }
        case KIND_GLITTER: { return 0.10; }
        case KIND_SCALES: { return 0.14; }
        case KIND_LAVA_VEINS: { return 0.06; }
        case KIND_BRUSHED_METAL: { return 0.68; }
        case KIND_WIZARDS_VEINS: { return 0.04; }
        default: { return 0.0; }
    }
}

fn kind_emissive(kind: u32, uv: vec2<f32>) -> vec3<f32> {
    if (kind == KIND_WIZARDS_VEINS) {
        let n1 = fbm(vec3<f32>(uv * 4.2, 2.7));
        let ridge = 1.0 - abs(n1 * 2.0 - 1.0);
        let vein = pow(saturate(ridge * 1.08), 9.0);
        return vec3<f32>(0.10, 0.16, 0.18) * vein;
    }
    if (kind == KIND_LAVA_VEINS) {
        let n1 = fbm(vec3<f32>(uv * 4.4, 3.1));
        let ridge = 1.0 - abs(n1 * 2.0 - 1.0);
        let vein = pow(saturate(ridge * 1.1), 8.0);
        return vec3<f32>(0.22, 0.06, 0.02) * vein;
    }
    return vec3<f32>(0.0);
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: ClothingVertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    let uv0 = look_coord(mesh);
    // Opaque ignores alpha; discard on the mesh UV so holes do not swim.
    if (material.kind == KIND_TATTERED && tatter_hole_alpha(uv0) < 0.08) {
        discard;
    }

    let base = material.colors[0].xyz;
    let P = mesh.world_position.xyz;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);
    let prepared_normal = fns::prepare_world_normal(mesh.world_normal, true, is_front);
    let N = normalize(prepared_normal);
    let V = pbr_input.V;

    let style = relief_style();
    let camera_distance = length(P - view.world_position.xyz);
    let footprint = max(length(dpdx(uv0)), length(dpdy(uv0)));
    let resolution_w = 1.0 - smoothstep(style.width_uv * 0.6, style.width_uv * 1.4, footprint);
    let pom_distance_w = 1.0 - smoothstep(POM_FADE_START_M, POM_FADE_END_M, camera_distance);
    let facing_w = smoothstep(0.04, 0.14, dot(N, V));
    let pom_w = pom_distance_w * resolution_w * facing_w;

    let visual_h = relief_height(uv0);
    var cavity = mix(1.0, mix(style.dark, 1.0, visual_h), facing_w * 0.65);
    var look = kind_look(material.kind, base, uv0, N);
    var n = N;

    if (pom_w > 1e-4) {
        let ray_world = -V * (style.depth_m / max(dot(N, V), 0.08));
        let ray_uv = world_ray_to_uv(ray_world, P, uv0);
        let hit = trace_relief(uv0, ray_uv, style);
        let g = relief_gradient(hit.uv);
        let world_g = uv_gradient_to_world(g, P, uv0);
        let relief_n = normalize(N - style.depth_m * world_g);
        n = normalize(mix(N, relief_n, pom_w));
        cavity = mix(cavity, mix(style.dark, 1.0, hit.height), pom_w);
    }

    look = vec4<f32>(look.xyz * cavity, look.w);

    let emissive = kind_emissive(material.kind, uv0);

    pbr_input.material.base_color = vec4<f32>(look.xyz, 1.0);
    pbr_input.material.perceptual_roughness = look.w;
    pbr_input.material.metallic = kind_metallic(material.kind);
    pbr_input.material.reflectance = vec3<f32>(0.12, 0.12, 0.12);
    pbr_input.material.flags = pbr_input.material.flags | STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb + emissive, 1.0), view.color_grading);
}
