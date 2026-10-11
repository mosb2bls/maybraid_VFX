//---------------------------------------------------------
// Doric column shaft: palette tint + stylized fluting POM.
//
// Relief lives in mesh UV (see COLUMN_SURFACE.md):
//   U wraps once around the shaft, seam at U = 0/1 (local -Z)
//   V runs 0..1 up the authored shaft
//   On a taper, circumference shrinks up V (πD → ~0.8πD);
//   U stays angular so flute count is constant. The UV
//   Jacobian maps a metre-depth ray into more ΔU at the neck.
// Color-only relief: no silhouette, displaced depth, or
// self-shadow ray. Shaft outline stays real geometry.
//
// Sampling is bounded (min/max steps) and fades with
// distance, screen footprint, and grazing angle.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_vertex_output,
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct ColumnSurfaceUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    kind: u32,
    _pad: vec3<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: ColumnSurfaceUniform;

const KIND_DORIC: u32 = 0u;

const DEFAULT_FLUTE_COUNT: f32 = 20.0;
const MIN_FLUTE_COUNT: f32 = 8.0;
const MAX_FLUTE_COUNT: f32 = 32.0;
const DEFAULT_DEPTH_M: f32 = 0.09;
const MIN_DEPTH_M: f32 = 0.008;
const MAX_DEPTH_M: f32 = 0.12;
const DEFAULT_FADE_START_M: f32 = 28.0;
const DEFAULT_FADE_END_M: f32 = 85.0;
const POM_MIN_STEPS: i32 = 12;
const POM_MAX_STEPS: i32 = 48;
const POM_REFINE_STEPS: i32 = 4;
const RELIEF_NORMAL_EPS_UV: f32 = 0.0015;
const FLUTE_FLOOR: f32 = 0.08;
const ARRIS_WIDTH: f32 = 0.055;
const TAU: f32 = 6.28318530718;

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

fn flute_count() -> f32 {
    let override_count = material.scalars[0].w;
    if (override_count >= 1.0) {
        return clamp(override_count, MIN_FLUTE_COUNT, MAX_FLUTE_COUNT);
    }
    return DEFAULT_FLUTE_COUNT;
}

fn fade_start_m() -> f32 {
    let override_start = material.scalars[1].x;
    if (override_start > 1e-4) {
        return override_start;
    }
    return DEFAULT_FADE_START_M;
}

fn fade_end_m() -> f32 {
    let override_end = material.scalars[1].y;
    if (override_end > 1e-4) {
        return max(override_end, fade_start_m() + 1.0);
    }
    return DEFAULT_FADE_END_M;
}

fn pom_max_steps() -> i32 {
    let override_steps = material.scalars[1].z;
    if (override_steps >= 1.0) {
        return i32(clamp(override_steps, f32(POM_MIN_STEPS), f32(POM_MAX_STEPS)));
    }
    return POM_MAX_STEPS;
}

fn relief_depth_m() -> f32 {
    let override_depth = material.scalars[1].w;
    if (override_depth > 1e-4) {
        return clamp(override_depth, MIN_DEPTH_M, MAX_DEPTH_M);
    }
    return DEFAULT_DEPTH_M;
}

/// Scalar 8. Flat limestone: no march, no flute cavity.
fn is_flat() -> bool {
    return material.scalars[2].x >= 0.5;
}

struct ReliefStyle {
    depth_m: f32,
    width_uv: f32,
    dark: f32,
}

fn relief_style() -> ReliefStyle {
    return ReliefStyle(relief_depth_m(), 1.0 / flute_count(), 0.12);
}

/// Periodic in U so the U = 0/1 seam does not jump.
fn flute_height(uv: vec2<f32>) -> f32 {
    let count = flute_count();
    let u = fract(uv.x);
    let t = fract(u * count);
    let channel = pow(0.5 - 0.5 * cos(t * TAU), 0.65);
    let d = min(t, 1.0 - t);
    let arris = 1.0 - smoothstep(0.0, ARRIS_WIDTH, d);
    var h = mix(1.0, FLUTE_FLOOR, channel);
    h = mix(h, 1.0, arris * 0.55);
    let pit = hash13(vec3<f32>(floor(u * count), floor(uv.y * 18.0), 9.0));
    return saturate(h - 0.06 * pow(pit, 3.0));
}

fn column_look(world: vec3<f32>, height: f32) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let scale = mix(1.2, 2.6, saturate(scalar1()));
    let p = world * max(material.noise.x, 1e-4) * scale;
    let grain = fbm(p);
    let pit = value_noise_3d(p * 5.5);
    let wear = saturate(scalar2());
    var tint = mix(base, accent, grain * 0.22 + (1.0 - height) * 0.42);
    tint *= 0.82 + 0.22 * pit;
    let lichen = smoothstep(0.55, 0.82, fbm(p * 0.35 + vec3<f32>(2.1, 0.0, 1.4)))
        * (1.0 - height);
    tint = mix(tint, tint * vec3<f32>(0.78, 0.86, 0.70), lichen * wear);
    let roughness = mix(0.78, 0.96, grain * 0.55 + (1.0 - height) * 0.28);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
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
    let steps = i32(clamp(requested, f32(POM_MIN_STEPS), f32(pom_max_steps())));
    let h0 = flute_height(uv0);
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
        let f = flute_height(uv0 + ray_uv * t) - (1.0 - t);
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
        let f = flute_height(uv0 + ray_uv * mid) - (1.0 - mid);
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
    return ReliefHit(uv_hit, flute_height(uv_hit), t);
}

fn relief_gradient(uv: vec2<f32>) -> vec2<f32> {
    let e = RELIEF_NORMAL_EPS_UV;
    let dx = flute_height(uv + vec2<f32>(e, 0.0))
        - flute_height(uv - vec2<f32>(e, 0.0));
    let dy = flute_height(uv + vec2<f32>(0.0, e))
        - flute_height(uv - vec2<f32>(0.0, e));
    return vec2<f32>(dx, dy) / (2.0 * e);
}

/// Mesh UV when the unwrap has a screen gradient; otherwise cylindrical U
/// from the outward normal (Y-up shaft). Seam matches the placeholder: U = 0
/// on local −Z.
fn column_uv(mesh: VertexOutput, n: vec3<f32>) -> vec2<f32> {
#ifdef VERTEX_UVS_A
    let mesh_uv = mesh.uv;
    if (length(dpdx(mesh_uv)) + length(dpdy(mesh_uv)) > 1e-7) {
        return mesh_uv;
    }
#endif
    let radial = vec2<f32>(n.x, n.z);
    let u = fract(atan2(radial.x, radial.y) / TAU + 0.5);
    return vec2<f32>(u, mesh.world_position.y);
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input = pbr_input_from_vertex_output(mesh, is_front, false);

    let P = mesh.world_position.xyz;
    let N = normalize(pbr_input.world_normal);
    let V = pbr_input.V;
    let uv0 = column_uv(mesh, N);
    let style = relief_style();
    let camera_distance = length(P - view.world_position.xyz);
    let flat = is_flat();

    let footprint = max(length(dpdx(uv0)), length(dpdy(uv0)));
    let resolution_w = 1.0 - smoothstep(style.width_uv * 0.55, style.width_uv * 1.35, footprint);
    let pom_distance_w = 1.0 - smoothstep(fade_start_m(), fade_end_m(), camera_distance);
    let facing_w = smoothstep(0.03, 0.12, dot(N, V));
    let pom_w = select(0.0, pom_distance_w * resolution_w * facing_w, !flat);

    let visual_h = select(1.0, flute_height(uv0), !flat);
    let visual_cavity = mix(style.dark, 1.0, visual_h);
    var look = column_look(P, visual_h);
    look = vec4<f32>(look.xyz * mix(1.0, visual_cavity, facing_w), look.w);
    var n = N;

    if (pom_w > 1e-4) {
        let ray_world = -V * (style.depth_m / max(dot(N, V), 0.07));
        let ray_uv = world_ray_to_uv(ray_world, P, uv0);
        let hit = trace_relief(uv0, ray_uv, style);
        let g = relief_gradient(hit.uv);
        let world_g = uv_gradient_to_world(g, P, uv0);
        let relief_n = normalize(N - style.depth_m * world_g);
        n = normalize(mix(N, relief_n, pom_w));

        let hit_look = column_look(P + ray_world * hit.t, hit.height);
        let cavity = mix(style.dark, 1.0, hit.height);
        look = mix(look, vec4<f32>(hit_look.xyz * cavity, hit_look.w), pom_w);
    }

    pbr_input.material.base_color = vec4<f32>(look.xyz, 1.0);
    pbr_input.material.perceptual_roughness = look.w;
    pbr_input.material.metallic = 0.0;
    pbr_input.material.reflectance = vec3<f32>(0.16, 0.16, 0.16);
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb, 1.0), view.color_grading);
}
