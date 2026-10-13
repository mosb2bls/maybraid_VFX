//---------------------------------------------------------
// Richmond urban surfaces: palette tint + named recipe kind.
//
// Kind: 0 stucco, 1 terracotta, 2 wood, 3 hay, 4 iron, 5 stone.
//
// Near: implicit-surface POM against a per-kind height field
// sampled in the dominant face plane (walls and roofs).
// Far: the same field as cavity tint / cheap normals.
// Color-only relief: no silhouette, displaced depth, or
// self-shadow ray.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_vertex_output,
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct UrbanSurfaceUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    kind: u32,
    _pad: vec3<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: UrbanSurfaceUniform;

const KIND_STUCCO: u32 = 0u;
const KIND_TERRACOTTA: u32 = 1u;
const KIND_WOOD: u32 = 2u;
const KIND_HAY: u32 = 3u;
const KIND_IRON: u32 = 4u;
const KIND_STONE: u32 = 5u;

const STONE_COURSE_M: f32 = 0.46;
const STONE_BLOCK_M: f32 = 0.92;

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (3.0 - 2.0 * f0);

    let n000 = hash13(i);
    let n100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
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

fn look_coord(world: vec3<f32>) -> vec3<f32> {
    let freq = max(material.noise.x, 1e-4);
    return world * freq;
}

fn palette_base() -> vec3<f32> {
    return material.colors[0].xyz;
}

fn palette_accent() -> vec3<f32> {
    let accent = material.colors[1].xyz;
    if (dot(accent, accent) < 1e-6) {
        return palette_base() * 0.72;
    }
    return accent;
}

fn scalar0() -> f32 {
    return material.scalars[0].x;
}

fn scalar1() -> f32 {
    return material.scalars[0].y;
}

fn scalar2() -> f32 {
    return material.scalars[0].z;
}

/// rgb + roughness. `w` is perceptual roughness.
fn stucco_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let scale = mix(1.1, 2.4, saturate(scalar1()));
    let mottling = fbm(p * scale);
    let pits = value_noise_3d(p * scale * 4.2);
    let wear = saturate(scalar2());
    var tint = mix(base, accent, mottling * 0.35 + wear * 0.2);
    tint *= 0.88 + 0.18 * pits;
    let roughness = mix(0.86, 0.98, mottling);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

fn terracotta_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let scale = mix(1.4, 3.2, saturate(scalar1()));
    let tile = vec2<f32>(p.x, p.z) * scale * 1.6;
    let grout = max(
        abs(fract(tile.x) - 0.5),
        abs(fract(tile.y) - 0.5),
    );
    let line = smoothstep(0.44, 0.5, grout);
    let clay = fbm(p * scale);
    var tint = mix(base, accent, clay * 0.4);
    tint = mix(tint, tint * 0.55, line);
    let roughness = mix(0.72, 0.9, clay);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

fn wood_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let scale = mix(1.1, 2.2, saturate(scalar1()));
    let mottling = fbm(p * scale);
    var tint = mix(base, accent, mottling * 0.28);
    let roughness = mix(0.64, 0.80, mottling);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

fn hay_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let scale = mix(2.2, 4.4, saturate(scalar1()));
    let strand_id = floor(p.x * scale * 7.0 + p.z * scale * 1.4);
    let strand = fract(p.x * scale * 7.0 + fbm(vec3<f32>(strand_id, p.y, p.z)) * 0.4);
    let ridge = pow(1.0 - abs(strand * 2.0 - 1.0), 0.55);
    let clump = fbm(p * scale * 0.7);
    var tint = mix(base, accent, clump * 0.5);
    tint *= mix(0.72, 1.12, ridge);
    let roughness = mix(0.9, 0.99, 1.0 - ridge);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

fn iron_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let rust = palette_accent();
    let scale = mix(1.2, 2.8, saturate(scalar1()));
    let blot = fbm(p * scale);
    let speckle = value_noise_3d(p * scale * 5.5);
    let rust_amt = smoothstep(0.42, 0.78, blot) * mix(0.25, 0.7, saturate(scalar2()));
    var tint = mix(base * (0.78 + 0.22 * speckle), rust, rust_amt);
    let roughness = mix(0.28, 0.72, rust_amt);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

/// `x` horizontal metres along the running bond, `y` metres up the course.
fn stone_block_id(uv: vec2<f32>) -> vec2<f32> {
    let row = floor(uv.y / STONE_COURSE_M);
    let shift = fract(row * 0.5) * STONE_BLOCK_M + hash13(vec3<f32>(row, 7.0, 3.0)) * 0.18;
    return vec2<f32>(floor((uv.x + shift) / STONE_BLOCK_M), row);
}

/// Set per fragment so looks share the relief grid's face-plane coordinates.
var<private> look_domain: ReliefDomain;

fn stone_look(p: vec3<f32>, world: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let id = stone_block_id(relief_uv(world, look_domain));
    let block = hash13(vec3<f32>(id, 19.0));
    let grain = fbm(p * mix(1.6, 3.2, saturate(scalar1())));
    let lichen = smoothstep(0.58, 0.82, fbm(p * 0.35 + vec3<f32>(3.1, 0.0, 1.7)));
    var tint = mix(base, accent, saturate(block * 0.7 + grain * 0.35));
    tint *= 0.82 + 0.26 * value_noise_3d(p * 9.0);
    tint = mix(tint, tint * vec3<f32>(0.78, 0.86, 0.72), lichen * saturate(scalar2()));
    let roughness = mix(0.82, 0.96, grain);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

fn surface_look(world: vec3<f32>) -> vec4<f32> {
    let p = look_coord(world);
    switch material.kind {
        case KIND_TERRACOTTA: { return terracotta_look(p); }
        case KIND_WOOD: { return wood_look(p); }
        case KIND_HAY: { return hay_look(p); }
        case KIND_IRON: { return iron_look(p); }
        case KIND_STONE: { return stone_look(p, world); }
        default: { return stucco_look(p); }
    }
}

fn surface_metallic(look: vec4<f32>) -> f32 {
    if (material.kind == KIND_IRON) {
        return mix(0.72, 0.18, saturate((look.w - 0.28) / 0.5));
    }
    if (material.kind == KIND_TERRACOTTA) {
        return 0.02;
    }
    return 0.0;
}

//---------------------------------------------------------
// Relief domain: stable metres in the dominant face plane.
// Roofs use XZ (same as Durham). Walls use ZY or XY.
//---------------------------------------------------------

struct ReliefDomain {
    a: vec3<f32>,
    b: vec3<f32>,
}

struct ReliefStyle {
    depth_m: f32,
    width_m: f32,
    dark: f32,
}

const POM_FADE_START_M: f32 = 35.0;
const POM_FADE_END_M: f32 = 90.0;
const POM_MIN_STEPS: i32 = 16;
const POM_MAX_STEPS: i32 = 64;
const POM_REFINE_STEPS: i32 = 5;
const RELIEF_NORMAL_EPS_M: f32 = 0.006;

fn relief_hash(p: vec2<i32>, seed: u32) -> u32 {
    var v = bitcast<u32>(p.x) * 0x8da6b343u
          ^ bitcast<u32>(p.y) * 0xd8163841u ^ seed;
    v = (v ^ (v >> 16u)) * 0x7feb352du;
    v = (v ^ (v >> 15u)) * 0x846ca68bu;
    return v ^ (v >> 16u);
}

fn relief_random(p: vec2<i32>, seed: u32) -> f32 {
    return f32(relief_hash(p, seed) >> 8u) * (1.0 / 16777216.0);
}

fn relief_domain(n: vec3<f32>) -> ReliefDomain {
    let an = abs(n);
    if (an.y >= an.x && an.y >= an.z) {
        return ReliefDomain(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0));
    }
    if (an.x >= an.z) {
        return ReliefDomain(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 0.0));
    }
    return ReliefDomain(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0));
}

fn relief_uv(p: vec3<f32>, domain: ReliefDomain) -> vec2<f32> {
    return vec2<f32>(dot(p, domain.a), dot(p, domain.b));
}

fn relief_style() -> ReliefStyle {
    switch material.kind {
        case KIND_TERRACOTTA: {
            return ReliefStyle(0.014, 0.016, 0.62);
        }
        case KIND_WOOD: {
            return ReliefStyle(0.006, 0.010, 0.70);
        }
        case KIND_HAY: {
            return ReliefStyle(0.016, 0.012, 0.68);
        }
        case KIND_IRON: {
            return ReliefStyle(0.008, 0.018, 0.55);
        }
        case KIND_STONE: {
            return ReliefStyle(0.045, 0.030, 0.42);
        }
        default: {
            return ReliefStyle(0.022, 0.016, 0.58);
        }
    }
}

fn voronoi_edge_m(uv: vec2<f32>, cell_m: f32) -> f32 {
    let domain = uv / cell_m;
    let cell = vec2<i32>(floor(domain));
    let local = fract(domain);
    var first = 1e10;
    var second = 1e10;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let offset = vec2<i32>(x, y);
            let id = cell + offset;
            let jitter = vec2<f32>(relief_random(id, 71u), relief_random(id, 193u));
            let feature = vec2<f32>(offset) + vec2<f32>(0.3) + 0.4 * jitter;
            let d2 = dot(feature - local, feature - local);
            if (d2 < first) {
                second = first;
                first = d2;
            } else {
                second = min(second, d2);
            }
        }
    }
    return (sqrt(second) - sqrt(first)) * cell_m;
}

fn grid_edge_m(uv: vec2<f32>, tile_m: f32) -> f32 {
    let fx = fract(uv.x / tile_m);
    let fy = fract(uv.y / tile_m);
    let dx = min(fx, 1.0 - fx) * tile_m;
    let dy = min(fy, 1.0 - fy) * tile_m;
    return min(dx, dy);
}

fn stucco_height(uv: vec2<f32>, width_m: f32) -> f32 {
    let crack = smoothstep(0.0, width_m, voronoi_edge_m(uv, 0.42));
    let pit_n = relief_random(vec2<i32>(floor(uv * 7.0)), 29u);
    let pit = 1.0 - 0.4 * pow(saturate((pit_n - 0.72) / 0.28), 2.0);
    return min(crack, pit);
}

fn terracotta_height(uv: vec2<f32>, width_m: f32) -> f32 {
    return smoothstep(0.0, width_m, grid_edge_m(uv, 0.24));
}

fn wood_height(uv: vec2<f32>, width_m: f32) -> f32 {
    let plank = 0.13;
    let fy = fract(uv.y / plank);
    let edge = min(fy, 1.0 - fy) * plank;
    let groove = smoothstep(0.0, width_m, edge);
    let grain = (relief_random(vec2<i32>(floor(vec2<f32>(uv.x * 2.0, uv.y * 14.0))), 11u) - 0.5) * 0.08;
    return saturate(groove + grain);
}

fn hay_height(uv: vec2<f32>, width_m: f32) -> f32 {
    let strand = 0.04;
    let cell = vec2<i32>(i32(floor(uv.x / 0.35)), i32(floor(uv.y / strand)));
    let wobble = (relief_random(cell, 53u) - 0.5) * 0.35;
    let s = fract(uv.y / strand + wobble);
    let ridge = pow(1.0 - abs(s * 2.0 - 1.0), 0.55);
    let gap = 1.0 - smoothstep(0.0, width_m, min(s, 1.0 - s) * strand);
    return mix(0.28, 1.0, ridge) * mix(1.0, 0.55, gap * 0.35);
}

fn iron_height(uv: vec2<f32>, width_m: f32) -> f32 {
    let seam = smoothstep(0.0, width_m, grid_edge_m(uv, 0.85));
    let id = vec2<i32>(floor(uv * 5.5));
    let pit_n = relief_random(id, 101u);
    let pit = 1.0 - 0.55 * pow(saturate((pit_n - 0.78) / 0.22), 2.0);
    return min(seam, pit);
}

/// Mortar joints recessed below pillowed, chiselled block faces.
fn stone_height(uv: vec2<f32>, width_m: f32) -> f32 {
    let id = stone_block_id(uv);
    let row = id.y;
    let shift = fract(row * 0.5) * STONE_BLOCK_M + hash13(vec3<f32>(row, 7.0, 3.0)) * 0.18;
    let local = vec2<f32>(uv.x + shift - id.x * STONE_BLOCK_M, uv.y - row * STONE_COURSE_M);
    let edge = min(
        min(local.x, STONE_BLOCK_M - local.x),
        min(local.y, STONE_COURSE_M - local.y),
    );
    let joint = smoothstep(0.0, width_m, edge);
    let pillow = smoothstep(0.0, 0.12, edge);
    let chisel = relief_random(vec2<i32>(floor(uv * 11.0)), 61u);
    let face = mix(0.72, 1.0, pillow) - 0.1 * chisel;
    return saturate(min(joint, face));
}

fn relief_height(uv: vec2<f32>, width_m: f32) -> f32 {
    switch material.kind {
        case KIND_TERRACOTTA: { return terracotta_height(uv, width_m); }
        case KIND_WOOD: { return wood_height(uv, width_m); }
        case KIND_HAY: { return hay_height(uv, width_m); }
        case KIND_IRON: { return iron_height(uv, width_m); }
        case KIND_STONE: { return stone_height(uv, width_m); }
        default: { return stucco_height(uv, width_m); }
    }
}

struct ReliefHit {
    offset: vec3<f32>,
    height: f32,
    t: f32,
}

// N·(Q-P) + D*(1-h(uv(Q))) = 0 on Q = P - V*s.
// Same solver as Durham terrain; uv is the face-plane projection.
fn trace_relief(
    uv0: vec2<f32>,
    ray_uv: vec2<f32>,
    ray_world: vec3<f32>,
    style: ReliefStyle,
) -> ReliefHit {
    let travel = length(ray_uv);
    let requested = ceil(travel / max(style.width_m * 0.25, 1e-4));
    let steps = i32(clamp(requested, f32(POM_MIN_STEPS), f32(POM_MAX_STEPS)));
    let h0 = relief_height(uv0, style.width_m);
    if (h0 >= 1.0) {
        return ReliefHit(vec3<f32>(0.0), h0, 0.0);
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
        let f = relief_height(uv0 + ray_uv * t, style.width_m) - (1.0 - t);
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
        let f = relief_height(uv0 + ray_uv * mid, style.width_m) - (1.0 - mid);
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
    return ReliefHit(
        ray_world * t,
        relief_height(uv0 + ray_uv * t, style.width_m),
        t,
    );
}

fn relief_gradient(uv: vec2<f32>, width_m: f32) -> vec2<f32> {
    let e = RELIEF_NORMAL_EPS_M;
    let dx = relief_height(uv + vec2<f32>(e, 0.0), width_m)
        - relief_height(uv - vec2<f32>(e, 0.0), width_m);
    let dy = relief_height(uv + vec2<f32>(0.0, e), width_m)
        - relief_height(uv - vec2<f32>(0.0, e), width_m);
    return vec2<f32>(dx, dy) / (2.0 * e);
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    // Copy mesh flags (shadow receiver, etc.) plus view / frag / prepared N.
    // `pbr_input_new()` leaves `flags = 0`, so `apply_pbr_lighting` skipped shadow maps.
    var pbr_input = pbr_input_from_vertex_output(mesh, is_front, false);

    let P = mesh.world_position.xyz;
    let N = normalize(pbr_input.world_normal);
    let V = pbr_input.V;
    let domain = relief_domain(N);
    look_domain = domain;
    let uv0 = relief_uv(P, domain);
    let style = relief_style();
    let camera_distance = length(P - view.world_position.xyz);

    let footprint = max(length(dpdx(uv0)), length(dpdy(uv0)));
    let resolution_w = 1.0 - smoothstep(style.width_m * 0.5, style.width_m * 1.25, footprint);
    let pom_distance_w = 1.0 - smoothstep(POM_FADE_START_M, POM_FADE_END_M, camera_distance);
    let facing_w = smoothstep(0.025, 0.10, dot(N, V));
    let pom_w = pom_distance_w * resolution_w * facing_w;

    let visual_width = mix(style.width_m, style.width_m * 2.4, smoothstep(35.0, 120.0, camera_distance));
    let visual_h = relief_height(uv0, visual_width);
    let visual_cavity = mix(style.dark, 1.0, visual_h);

    var look = surface_look(P);
    look = vec4<f32>(look.xyz * mix(1.0, visual_cavity, facing_w), look.w);
    var n = N;

    if (pom_w > 1e-4) {
        let ray_world = -V * (style.depth_m / max(dot(N, V), 0.06));
        let ray_uv = vec2<f32>(dot(ray_world, domain.a), dot(ray_world, domain.b));
        let hit = trace_relief(uv0, ray_uv, ray_world, style);
        let q = P + hit.offset;
        let g = relief_gradient(uv0 + ray_uv * hit.t, style.width_m);
        let world_g = g.x * domain.a + g.y * domain.b;
        let relief_n = normalize(N - style.depth_m * world_g);
        n = normalize(mix(N, relief_n, pom_w));

        let hit_look = surface_look(q);
        let cavity = mix(style.dark, 1.0, hit.height);
        look = mix(look, vec4<f32>(hit_look.xyz * cavity, hit_look.w), pom_w);
    }

    pbr_input.material.base_color = vec4<f32>(look.xyz, 1.0);
    pbr_input.material.perceptual_roughness = look.w;
    pbr_input.material.metallic = surface_metallic(look);
    pbr_input.material.reflectance = vec3<f32>(0.18, 0.18, 0.18);
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb, 1.0), view.color_grading);
}
