//---------------------------------------------------------
// Sci-fi grenade: gunmetal shell, chrome equator, gadget POM.
//
// Height lives in object-space spherical UV so the belt stays
// equatorial as the body tumbles. Durham-style implicit POM
// (UV jacobian) shades cavity and tilts the normal. Albedo
// stays on the sphere domain; the march does not resample look.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
    pbr_types::{PbrInput, pbr_input_new},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct GrenadeMaterialUniform {
    shell: vec4<f32>,
    chrome: vec4<f32>,
    led: vec4<f32>,
    extras: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: GrenadeMaterialUniform;

struct GrenadeVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) local_pos: vec3<f32>,
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
}

struct ReliefStyle {
    depth_m: f32,
    width_uv: f32,
    dark: f32,
}

struct ReliefHit {
    uv: vec2<f32>,
    height: f32,
    t: f32,
}

const TAU: f32 = 6.28318530718;
const POM_FADE_START_M: f32 = 6.0;
const POM_FADE_END_M: f32 = 16.0;
const POM_MIN_STEPS: i32 = 10;
const POM_MAX_STEPS: i32 = 28;
const POM_REFINE_STEPS: i32 = 3;
const RELIEF_NORMAL_EPS_UV: f32 = 0.0018;

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn hash21(p: vec2<f32>) -> f32 {
    return hash13(vec3<f32>(p, 17.13));
}

fn sphere_uv(n: vec3<f32>) -> vec2<f32> {
    return vec2<f32>(atan2(n.x, n.z) / TAU + 0.5, n.y * 0.5 + 0.5);
}

fn sphere_dir(uv: vec2<f32>) -> vec3<f32> {
    let lon = (uv.x - 0.5) * TAU;
    let lat = uv.y * 2.0 - 1.0;
    let ring = sqrt(max(1.0 - lat * lat, 0.0));
    return vec3<f32>(sin(lon) * ring, lat, cos(lon) * ring);
}

fn gadget_height(uv: vec2<f32>) -> f32 {
    let n = sphere_dir(uv);
    let equator = abs(n.y);
    let ring = smoothstep(0.085, 0.048, equator) * smoothstep(0.016, 0.038, equator);
    let belt = 1.0 - smoothstep(0.0, 0.022, equator);

    let tiles = vec2<f32>(uv.x * 10.0, uv.y * 8.0);
    let row = floor(tiles.y);
    var q = tiles;
    q.x += step(0.5, fract(row * 0.5)) * 0.5;
    let cell = floor(q);
    let f = fract(q) - vec2<f32>(0.5);
    let box = max(abs(f.x), abs(f.y));
    let panel = 1.0 - smoothstep(0.36, 0.46, box);
    let groove = smoothstep(0.40, 0.48, box);

    let mark = hash21(cell);
    let port = select(0.0, smoothstep(0.18, 0.10, length(f)), mark > 0.68);
    let lens = select(0.0, smoothstep(0.08, 0.04, length(f)), mark > 0.86);

    let vents = abs(fract(uv.x * 22.0) - 0.5);
    let vent_band = smoothstep(0.12, 0.17, equator) * smoothstep(0.30, 0.23, equator);
    let slat = (1.0 - smoothstep(0.10, 0.18, vents)) * vent_band;

    let rivet_d = length(f - vec2<f32>(0.28, 0.28) * sign(f + vec2<f32>(1e-4)));
    let rivet = (1.0 - smoothstep(0.05, 0.09, rivet_d)) * step(0.42, mark) * (1.0 - ring);

    var height = 0.74 + panel * 0.14 - groove * 0.22 - port * 0.40 - lens * 0.12;
    height = mix(height, 0.42, belt);
    height = mix(height, 1.0, ring);
    height -= slat * 0.24;
    height = mix(height, 0.96, rivet);
    return saturate(height);
}

fn gadget_look(uv: vec2<f32>, local_n: vec3<f32>, world_n: vec3<f32>, v: vec3<f32>) -> vec4<f32> {
    let equator = abs(local_n.y);
    let band = smoothstep(0.095, 0.042, equator);
    let tiles = vec2<f32>(uv.x * 10.0, uv.y * 8.0);
    let row = floor(tiles.y);
    var q = tiles;
    q.x += step(0.5, fract(row * 0.5)) * 0.5;
    let cell = floor(q);
    let f = fract(q) - vec2<f32>(0.5);
    let mark = hash21(cell);
    let port = select(0.0, smoothstep(0.18, 0.10, length(f)), mark > 0.68);

    let fresnel = pow(1.0 - saturate(dot(world_n, v)), 3.2);
    let shell = material.shell.xyz * (0.88 + 0.12 * (1.0 - port));
    let chrome = material.chrome.xyz + vec3<f32>(0.08, 0.10, 0.14) * fresnel;
    let tint = mix(shell, chrome, band);
    let roughness = mix(material.shell.w, material.chrome.w, band);
    return vec4<f32>(tint, roughness);
}

fn gadget_metallic(n: vec3<f32>) -> f32 {
    return mix(0.42, 0.98, smoothstep(0.095, 0.042, abs(n.y)));
}

fn gadget_emissive(uv: vec2<f32>) -> vec3<f32> {
    let tiles = vec2<f32>(uv.x * 10.0, uv.y * 8.0);
    let row = floor(tiles.y);
    var q = tiles;
    q.x += step(0.5, fract(row * 0.5)) * 0.5;
    let cell = floor(q);
    let f = fract(q) - vec2<f32>(0.5);
    let mark = hash21(cell);
    if mark <= 0.68 {
        return vec3<f32>(0.0);
    }
    let ring = smoothstep(0.16, 0.12, length(f)) * smoothstep(0.07, 0.11, length(f));
    let pulse = 0.55 + 0.45 * sin(globals.time * 5.4 + mark * 18.0);
    return material.led.xyz * material.led.w * ring * pulse;
}

fn relief_style() -> ReliefStyle {
    return ReliefStyle(max(material.extras.x, 0.008), 0.014, 0.58);
}

fn safe_inv(x: f32) -> f32 {
    if abs(x) < 1e-10 {
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

fn trace_relief(uv0: vec2<f32>, ray_uv: vec2<f32>, style: ReliefStyle) -> ReliefHit {
    let travel = length(ray_uv);
    let requested = ceil(travel / max(style.width_uv * 0.25, 1e-5));
    let steps = i32(clamp(requested, f32(POM_MIN_STEPS), f32(POM_MAX_STEPS)));
    let h0 = gadget_height(uv0);
    if h0 >= 0.995 {
        return ReliefHit(uv0, h0, 0.0);
    }

    var lo = 0.0;
    var hi = 1.0;
    var f_lo = h0 - 1.0;
    var f_hi = 0.0;

    for (var i = 1; i <= POM_MAX_STEPS; i = i + 1) {
        if i > steps {
            break;
        }
        let t = f32(i) / f32(steps);
        let f = gadget_height(uv0 + ray_uv * t) - (1.0 - t);
        if f >= 0.0 {
            hi = t;
            f_hi = f;
            break;
        }
        lo = t;
        f_lo = f;
    }

    for (var i = 0; i < POM_REFINE_STEPS; i = i + 1) {
        let mid = 0.5 * (lo + hi);
        let f = gadget_height(uv0 + ray_uv * mid) - (1.0 - mid);
        if f >= 0.0 {
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
    return ReliefHit(uv_hit, gadget_height(uv_hit), t);
}

fn relief_gradient(uv: vec2<f32>) -> vec2<f32> {
    let e = RELIEF_NORMAL_EPS_UV;
    let dx = gadget_height(uv + vec2<f32>(e, 0.0)) - gadget_height(uv - vec2<f32>(e, 0.0));
    let dy = gadget_height(uv + vec2<f32>(0.0, e)) - gadget_height(uv - vec2<f32>(0.0, e));
    return vec2<f32>(dx, dy) / (2.0 * e);
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> GrenadeVertexOutput {
    var out: GrenadeVertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    out.local_pos = vertex.position;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index
    );
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: GrenadeVertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    let P = mesh.world_position.xyz;
    let local_n = normalize(mesh.local_pos);
    let uv0 = sphere_uv(local_n);

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);
    let prepared_normal = fns::prepare_world_normal(mesh.world_normal, false, is_front);
    let N = normalize(prepared_normal);
    let V = pbr_input.V;

    let style = relief_style();
    let camera_distance = length(P - view.world_position.xyz);
    let footprint = max(length(dpdx(uv0)), length(dpdy(uv0)));
    let resolution_w = 1.0 - smoothstep(style.width_uv * 0.6, style.width_uv * 1.4, footprint);
    let pom_distance_w = 1.0 - smoothstep(POM_FADE_START_M, POM_FADE_END_M, camera_distance);
    let facing_w = smoothstep(0.04, 0.14, dot(N, V));
    let pom_w = pom_distance_w * resolution_w * facing_w;

    let visual_h = gadget_height(uv0);
    var cavity = mix(1.0, mix(style.dark, 1.0, visual_h), facing_w * 0.70);
    var look = gadget_look(uv0, local_n, N, V);
    var n = N;

    if pom_w > 1e-4 {
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
    let emissive = gadget_emissive(uv0);

    pbr_input.material.base_color = vec4<f32>(look.xyz, 1.0);
    pbr_input.material.perceptual_roughness = look.w;
    pbr_input.material.metallic = gadget_metallic(local_n);
    pbr_input.material.reflectance = mix(
        vec3<f32>(0.18, 0.18, 0.18),
        vec3<f32>(0.55, 0.55, 0.58),
        smoothstep(0.095, 0.042, abs(local_n.y)),
    );
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb + emissive, 1.0), view.color_grading);
}
