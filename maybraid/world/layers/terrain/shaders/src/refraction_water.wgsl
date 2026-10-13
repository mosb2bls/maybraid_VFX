//---------------------------------------------------------
// Durham refraction water (ported from naturescapes).
//---------------------------------------------------------
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    mesh_view_bindings::view_transmission_texture,
    mesh_view_bindings::view_transmission_sampler,
    mesh_view_bindings::depth_prepass_texture,
    mesh_view_bindings::globals,
    utils::coords_to_viewport_uv,
    view_transformations::depth_ndc_to_view_z,
}
#ifdef DISTANCE_FOG
#import bevy_pbr::mesh_view_bindings::fog
#import bevy_pbr::pbr_functions as fns
#endif

fn with_distance_fog(color: vec4<f32>, world_position: vec3<f32>, frag_xy: vec2<f32>) -> vec4<f32> {
#ifdef DISTANCE_FOG
    return fns::apply_fog(fog, color, world_position, view.world_position.xyz, frag_xy);
#else
    return color;
#endif
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> water_color: vec4<f32>; // rgb tint, a = opacity

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> shallow_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> deep_color: vec4<f32>;

// x = distortion_strength, y = depth_scale, z = close_fade_strength
@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> water_params: vec4<f32>;

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn posterize3(c: vec3<f32>, steps: f32) -> vec3<f32> {
    let s = max(steps, 1.0);
    return floor(c * s) / s;
}

fn tri_wave(x: f32) -> f32 {
    let f = fract(x);
    return 1.0 - abs(f * 2.0 - 1.0);
}

fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 345.45));
    let d = dot(q, q + 34.345);
    return fract(q.x * q.y + d);
}

fn rot2(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let distortion_strength = water_params.x;
    let depth_scale = water_params.y;
    let close_fade_strength = water_params.z;

    var viewport_uv = coords_to_viewport_uv(in.position.xy, view.viewport);

    let visual_depth = textureLoad(depth_prepass_texture, vec2<i32>(in.position.xy), 0);
    let volume_depth = depth_ndc_to_view_z(in.position.z) - depth_ndc_to_view_z(visual_depth);

    let cam_pos = view.world_position.xyz;
    let world_pos = in.world_position.xyz;
    let n = normalize(in.world_normal);

    let dist_to_cam = length(world_pos - cam_pos);
    let close_fade = saturate((dist_to_cam - 2.0) / 6.0);
    let fade = mix(1.0, close_fade, saturate(close_fade_strength));

    viewport_uv += n.xz * distortion_strength * fade;

    let scene_col = textureSample(
        view_transmission_texture,
        view_transmission_sampler,
        viewport_uv
    ).rgb;

    let deepness = saturate((volume_depth + visual_depth) / depth_scale);
    let tint = mix(shallow_color.rgb, deep_color.rgb, deepness);

    let base_opacity = saturate(water_color.a);
    let opacity = saturate(base_opacity * deepness);

    var col = mix(scene_col, tint, opacity);
    col = mix(col, col * water_color.rgb, 0.45);
    col = posterize3(col, 40.0);

    let uv = world_pos.xz;
    let t = globals.time / 4.0;
    let cell_scale = 0.45;
    let cell = floor(uv * cell_scale);
    let r = hash21(cell);
    let warp = (r - 0.5) * 0.75;
    let warped_uv = uv + warp;
    let angle = hash21(cell) * 6.2831853;
    let cell_uv = fract(warped_uv * cell_scale);
    let local = cell_uv - 0.5;
    let pr = rot2(local, angle);

    let u1 = pr.x * 3.0 + t * 0.6;
    let w1 = tri_wave(u1);
    let line1 = smoothstep(0.90, 0.985, w1);
    var caustic = line1;

    let cell_r = hash21(cell);
    let presence = smoothstep(0.65, 0.75, cell_r);
    caustic *= presence;

    let cell_strength = mix(0.35, 1.25, hash21(cell + vec2<f32>(7.0, 13.0)));
    caustic *= cell_strength;

    col += vec3<f32>(1.0) * caustic * (0.12 + (deepness * 0.25));

    return with_distance_fog(vec4(col, 1.0), world_pos, in.position.xy);
}
