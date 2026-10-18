//---------------------------------------------------------
// Additive energy looks: hex cells, pulsing rings, long tail.
// Hex uses a triplanar domain so a sphere does not pin at the poles.
// Pulse is UV rings + hash boil. Tail pinches object -Y.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> core: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> limb: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> grout: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> extras: vec4<f32>;

const KIND_PULSE: f32 = 1.0;
const KIND_TAIL: f32 = 2.0;

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn hex_height(p: vec2<f32>) -> f32 {
    let row = floor(p.y);
    var q = p;
    q.x += step(0.5, fract(row * 0.5)) * 0.5;
    let f = fract(q) - vec2<f32>(0.5);
    let dist = length(f * vec2<f32>(1.05, 1.18));
    return 1.0 - smoothstep(0.36, 0.52, dist);
}

fn relief_domain(p: vec3<f32>, n: vec3<f32>) -> vec2<f32> {
    let an = abs(n);
    if an.y >= an.x && an.y >= an.z {
        return p.xz;
    }
    if an.x >= an.z {
        return p.zy;
    }
    return p.xy;
}

fn displace_tail(local: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let t = globals.time;
    let boil = hash21(local.xy * 3.2 + vec2<f32>(t * 1.5, local.z));
    var p = local + normal * ((boil - 0.5) * 0.08);
    if p.y > 0.0 {
        return p;
    }
    let depth = max(extras.x, 0.15);
    let aft = saturate(-p.y / depth);
    p.y -= aft * aft * depth;
    let pinch = 1.0 - aft * 0.45;
    p.x *= pinch;
    p.z *= pinch;
    return p;
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    var local = vertex.position;
    if extras.y > 1.5 {
        local = displace_tail(local, vertex.normal);
    }
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index,
    );
#endif
#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(local, 1.0),
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
        vertex_no_morph.instance_index,
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
        world_from_local[3],
    );
#endif
    return out;
}

fn shade_hex(mesh: VertexOutput, view_dir: vec3<f32>) -> vec4<f32> {
    let n = normalize(mesh.world_normal);
    let facing = saturate(abs(dot(n, view_dir)));
    let scale = max(limb.w, 1.0);
    let p = relief_domain(mesh.world_position.xyz, n) * scale;
    var height = hex_height(p);
    let ray = -view_dir * (max(extras.x, 0.04) / max(facing, 0.08));
    let domain_ray = relief_domain(mesh.world_position.xyz + ray, n) * scale - p;
    var t = 0.0;
    var hit = height;
    for (var i = 0; i < 5; i++) {
        t += 0.2;
        let sample = hex_height(p + domain_ray * t);
        if sample < (1.0 - t) {
            hit = sample;
            break;
        }
        hit = sample;
    }
    let flicker = 0.92 + 0.08 * sin(globals.time * grout.w * 6.0 + hash21(floor(p)) * 12.0);
    let cell = saturate(hit);
    let cavity = mix(0.22, 1.0, cell);
    let color = mix(grout.xyz, mix(limb.xyz, core.xyz, cell * cell), cavity);
    let pom_w = smoothstep(0.04, 0.18, facing);
    let lit = mix(limb.xyz * 0.55, color, pom_w) * flicker;
    let alpha = mix(0.35, 1.0, cell) * mix(0.55, 1.0, facing);
    return vec4<f32>(lit * max(core.w, 1.0), alpha);
}

fn shade_pulse(mesh: VertexOutput) -> vec4<f32> {
#ifdef VERTEX_UVS_A
    let uv = mesh.uv;
#else
    let uv = vec2<f32>(0.5, 0.5);
#endif
    let q = uv - vec2<f32>(0.5);
    let r = length(q);
    let rate = max(grout.w, 0.2);
    let t = globals.time * rate;
    let warp = hash21(uv * max(limb.w, 1.0) + vec2<f32>(t * 0.7, r * 3.0));
    let pulse = fract(t * 0.28);
    let ring_a = abs(r - (0.08 + pulse * 0.42));
    let ring_b = abs(r - (0.04 + fract(pulse + 0.48) * 0.46));
    let band = 1.0 - smoothstep(0.0, 0.045, ring_a);
    let band2 = 1.0 - smoothstep(0.0, 0.03, ring_b);
    let disk = 1.0 - smoothstep(0.12, 0.52, r);
    let boil = (warp - 0.5) * 0.28;
    let hot = saturate(max(band, band2) * disk + boil + (1.0 - smoothstep(0.0, 0.16, r)) * 0.65);
    let flicker = 0.88 + 0.12 * sin(globals.time * 28.0 + r * 14.0 + warp * 8.0);
    let color = mix(grout.xyz, mix(limb.xyz, core.xyz, hot), disk);
    let alpha = mix(0.2, 1.0, hot) * disk;
    return vec4<f32>(color * max(core.w, 1.0) * flicker, alpha);
}

fn shade_tail(mesh: VertexOutput, view_dir: vec3<f32>) -> vec4<f32> {
#ifdef VERTEX_UVS_A
    let uv = mesh.uv;
#else
    let uv = vec2<f32>(0.5, 0.5);
#endif
    let along = saturate(uv.y);
    let n = normalize(mesh.world_normal);
    let facing = saturate(abs(dot(n, view_dir)));
    let rate = max(grout.w, 0.2);
    let scroll = hash21(vec2<f32>(uv.x * max(limb.w, 1.0), uv.y * 5.0 - globals.time * rate));
    let core_w = 1.0 - smoothstep(0.12, 0.78, length(uv - vec2<f32>(0.5, along)));
    let flow = saturate(core_w + (scroll - 0.5) * 0.35);
    let aft = 1.0 - along;
    let color = mix(grout.xyz, mix(limb.xyz, core.xyz, flow * facing), 1.0 - aft * 0.35);
    let fade = smoothstep(0.0, 0.18, along) * mix(0.4, 1.0, facing);
    let flicker = 0.9 + 0.1 * sin(globals.time * 18.0 + along * 10.0);
    return vec4<f32>(color * max(core.w, 1.0) * flicker, fade);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(view.world_position.xyz - mesh.world_position.xyz);
    var shaded: vec4<f32>;
    if extras.y > 1.5 {
        shaded = shade_tail(mesh, view_dir);
    } else if extras.y > 0.5 {
        shaded = shade_pulse(mesh);
    } else {
        shaded = shade_hex(mesh, view_dir);
    }
    let mapped = tone_mapping(vec4<f32>(shaded.xyz, 1.0), view.color_grading);
    return vec4<f32>(mapped.rgb, shaded.w);
}
