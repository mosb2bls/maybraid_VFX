//---------------------------------------------------------
// Stylized explosion lobes: broad bulges, displaced normals,
// view-facing silhouette, opacity breakup from the start.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> params: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> tint: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> extras: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> color_hot: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var<uniform> color_mid: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var<uniform> color_cool: vec4<f32>;

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn hash31(p: vec3<f32>) -> f32 {
    return hash21(p.xy + vec2<f32>(p.z * 1.37, p.z * 0.61));
}

fn value_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n000 = hash31(i);
    let n100 = hash31(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash31(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash31(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash31(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash31(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash31(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash31(i + vec3<f32>(1.0, 1.0, 1.0));
    let x00 = mix(n000, n100, u.x);
    let x10 = mix(n010, n110, u.x);
    let x01 = mix(n001, n101, u.x);
    let x11 = mix(n011, n111, u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var n = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < 4; i++) {
        n += value_noise(q) * a;
        q = q * 2.03 + vec3<f32>(11.2, 3.7, 7.1);
        a *= 0.5;
    }
    return n;
}

fn soft_band(v: f32, steps: f32) -> f32 {
    let s = max(steps, 2.0);
    let x = saturate(v) * s;
    let q = floor(x);
    let f = fract(x);
    return (q + smoothstep(0.32, 0.68, f)) / s;
}

fn rotate_y(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}

fn object_pos(vertex: Vertex) -> vec3<f32> {
#ifdef VERTEX_COLORS
    return vertex.color.xyz;
#else
    return vertex.position;
#endif
}

fn deform(local: vec3<f32>, age_n: f32) -> vec3<f32> {
    let seed = params.z;
    let q = rotate_y(local, age_n * extras.y);
    let dir = normalize(q + vec3<f32>(1e-4, 0.0, 0.0));
    let b0 = normalize(vec3<f32>(sin(seed * 2.1), 0.38, cos(seed * 1.4)));
    let b1 = normalize(vec3<f32>(cos(seed * 3.3), -0.48, sin(seed * 0.9)));
    let b2 = normalize(vec3<f32>(sin(seed * 0.6 + 1.7), 0.72, cos(seed * 2.8)));
    let large = pow(saturate(dot(dir, b0)), 2.2) * 0.34
        + pow(saturate(dot(dir, b1)), 3.0) * 0.24
        + pow(saturate(dot(dir, b2)), 1.8) * 0.14;
    let fine = (fbm(q * 4.6 + vec3<f32>(seed, 2.0, age_n * 0.7)) - 0.48) * 0.12;
    let amount = extras.z * (0.85 + 0.35 * age_n);
    return rotate_y(q + dir * ((large + fine) * amount), -age_n * extras.y);
}

fn deform_normal(local: vec3<f32>, age_n: f32, n: vec3<f32>) -> vec3<f32> {
    let eps = 0.03;
    var t1 = cross(n, vec3<f32>(0.0, 1.0, 0.0));
    if length(t1) < 1e-3 {
        t1 = cross(n, vec3<f32>(1.0, 0.0, 0.0));
    }
    t1 = normalize(t1);
    let t2 = normalize(cross(n, t1));
    let p0 = deform(local, age_n);
    let p1 = deform(local + t1 * eps, age_n);
    let p2 = deform(local + t2 * eps, age_n);
    return normalize(cross(p1 - p0, p2 - p0));
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    let age_n = saturate(params.x / max(params.y, 1e-3));
    let local0 = object_pos(vertex);
    let local = deform(local0, age_n);
    let n_local = deform_normal(local0, age_n, vertex.normal);
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        n_local,
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
    out.color = vec4<f32>(local, 1.0);
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

fn silhouette_alpha(facing: f32, breakup: f32, age_n: f32) -> f32 {
    let rim = smoothstep(0.02, 0.38, facing);
    let holes = smoothstep(0.18, 0.52, breakup);
    let dissolve = 1.0 - smoothstep(0.55, 1.0, age_n + (1.0 - breakup) * 0.28);
    return rim * mix(0.45, 1.0, holes) * dissolve;
}

fn shade_fire(local: vec3<f32>, n: vec3<f32>, view_dir: vec3<f32>, age_n: f32) -> vec4<f32> {
    let facing = saturate(dot(n, view_dir));
    let blotch = fbm(local * 2.5 + vec3<f32>(params.z, age_n * 0.4, 1.6));
    let heat = saturate((1.0 - age_n) * mix(0.28, 1.05, blotch));
    var rgb = mix(color_cool.xyz, mix(color_mid.xyz, color_hot.xyz, saturate(heat * 1.25 - 0.12)), soft_band(heat, extras.w));
    rgb *= tint.xyz;
    let n_lit = saturate(dot(n, normalize(vec3<f32>(0.28, 0.86, 0.32))) * 0.22 + 0.78);
    rgb *= n_lit * extras.x;
    let alpha = silhouette_alpha(facing, blotch, age_n);
    return vec4<f32>(rgb, alpha);
}

fn shade_smoke(local: vec3<f32>, n: vec3<f32>, view_dir: vec3<f32>, age_n: f32) -> vec4<f32> {
    let facing = saturate(dot(n, view_dir));
    let blotch = fbm(local * 1.9 + vec3<f32>(params.z * 0.7, age_n * 0.85, 3.1));
    let n_lit = saturate(dot(n, normalize(vec3<f32>(0.32, 0.84, 0.22))) * 0.2 + 0.8);
    let lift = soft_band(blotch * 0.72 + n_lit * 0.28, extras.w);
    let rgb = mix(color_cool.xyz, color_hot.xyz, lift) * tint.xyz * extras.x;
    let alpha = silhouette_alpha(facing, blotch, age_n) * 0.92;
    return vec4<f32>(rgb, alpha);
}

fn shade_flash(local: vec3<f32>, n: vec3<f32>, view_dir: vec3<f32>, age_n: f32) -> vec4<f32> {
    let facing = saturate(dot(n, view_dir));
    let blotch = fbm(local * 3.2 + vec3<f32>(params.z, 0.0, 0.5));
    let rgb = mix(color_mid.xyz, color_hot.xyz, facing * mix(0.6, 1.0, blotch)) * tint.xyz;
    let fade = 1.0 - smoothstep(0.12, 1.0, age_n);
    let alpha = smoothstep(0.04, 0.32, facing) * fade * mix(0.45, 1.0, blotch);
    return vec4<f32>(rgb * extras.x * fade, alpha);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(view.world_position.xyz - mesh.world_position.xyz);
    let n = normalize(mesh.world_normal);
#ifdef VERTEX_COLORS
    let local = mesh.color.xyz;
#else
    let local = vec3<f32>(0.0);
#endif
    let age_n = saturate(params.x / max(params.y, 1e-3));
    var shaded: vec4<f32>;
    if params.w > 1.5 {
        shaded = shade_flash(local, n, view_dir, age_n);
    } else if params.w > 0.5 {
        shaded = shade_smoke(local, n, view_dir, age_n);
    } else {
        shaded = shade_fire(local, n, view_dir, age_n);
    }
    let mapped = tone_mapping(vec4<f32>(shaded.xyz, 1.0), view.color_grading);
    return vec4<f32>(mapped.rgb, shaded.w);
}
