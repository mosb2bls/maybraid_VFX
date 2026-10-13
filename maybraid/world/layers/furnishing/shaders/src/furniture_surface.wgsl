//---------------------------------------------------------
// Furniture surfaces: wood / cloth / lacquer / metal / marble /
// ornate / lava / cosmos / scales.
//
// Kind: 0 wood, 1 cloth, 2 soft, 3 marble, 4 ornate,
//       5 lava, 6 cosmos, 7 scales, 8 lacquer, 9 metal, 10 rockadder.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::{view, globals},
    pbr_fragment::pbr_input_from_vertex_output,
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct FurnitureSurfaceUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    kind: u32,
    _pad: vec3<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: FurnitureSurfaceUniform;

const KIND_WOOD: u32 = 0u;
const KIND_CLOTH: u32 = 1u;
const KIND_SOFT: u32 = 2u;
const KIND_MARBLE: u32 = 3u;
const KIND_ORNATE: u32 = 4u;
const KIND_LAVA: u32 = 5u;
const KIND_COSMOS: u32 = 6u;
const KIND_SCALES: u32 = 7u;
const KIND_LACQUER: u32 = 8u;
const KIND_METAL: u32 = 9u;
const KIND_ROCKADDER: u32 = 10u;

struct Look {
    rgb: vec3<f32>,
    roughness: f32,
    metallic: f32,
    reflectance: f32,
}

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
    for (var i = 0; i < 5; i++) {
        s += a * value_noise_3d(q);
        q *= 2.11;
        a *= 0.5;
    }
    return s;
}

fn look_coord(mesh: VertexOutput) -> vec3<f32> {
    let freq = max(material.noise.x, 1e-4);
    return mesh.world_position.xyz * freq;
}

fn palette(i: u32) -> vec3<f32> {
    return material.colors[i].xyz;
}

fn palette_or(i: u32, fallback: vec3<f32>) -> vec3<f32> {
    let c = palette(i);
    if (dot(c, c) < 1e-6) {
        return fallback;
    }
    return c;
}

fn hemi(n: vec3<f32>) -> vec3<f32> {
    let t = saturate(normalize(n).y * 0.5 + 0.5);
    return mix(vec3<f32>(0.55, 0.48, 0.46), vec3<f32>(1.02, 0.98, 0.94), t);
}

fn rim_color(kind: u32) -> vec3<f32> {
    if (kind == KIND_LAVA) {
        return vec3<f32>(1.0, 0.42, 0.10);
    }
    if (kind == KIND_COSMOS) {
        return vec3<f32>(0.72, 0.42, 1.0);
    }
    if (kind == KIND_SCALES) {
        return palette_or(2u, vec3<f32>(0.35, 0.88, 0.72));
    }
    if (kind == KIND_METAL) {
        return palette_or(1u, vec3<f32>(0.85, 0.78, 0.62));
    }
    if (kind == KIND_ROCKADDER) {
        return palette_or(2u, vec3<f32>(1.0, 0.84, 0.16));
    }
    return vec3<f32>(1.0, 0.78, 0.42);
}

fn finish(kind: u32, rgb: vec3<f32>, n: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    var tint = rgb;
    tint *= mix(vec3<f32>(1.0), hemi(n), 0.12);
    let v = normalize(view.world_position.xyz - world_pos);
    let rim = pow(1.0 - saturate(abs(dot(normalize(n), v))), 2.4);
    tint += rim_color(kind) * rim * 0.16;
    return tint;
}

/// BotW painted wood: large hue islands, no rings or smudged grain.
fn wood_look(p: vec3<f32>) -> Look {
    let base = palette(0u);
    let accent = palette_or(1u, base * vec3<f32>(1.12, 0.78, 0.55));
    let shade = palette_or(2u, base * vec3<f32>(0.62, 0.58, 0.52));
    let stain = fbm(p * 0.72);
    let wash = fbm(p * 0.28 + vec3<f32>(3.1, 1.7, 8.2));
    var tint = mix(base, accent, smoothstep(0.34, 0.66, stain));
    tint = mix(tint, shade, smoothstep(0.42, 0.78, wash) * 0.42);
    return Look(tint, 0.48, 0.0, 0.18);
}

fn rot45(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(p.x + p.y, p.y - p.x) * 0.70710678;
}

fn lattice(p: vec2<f32>) -> vec2<f32> {
    return abs(fract(p) - 0.5);
}

fn cloth_look(p: vec3<f32>) -> Look {
    let base = palette(0u);
    let accent = palette_or(1u, base * vec3<f32>(1.15, 0.7, 1.05));
    let scale = mix(3.2, 5.6, saturate(material.scalars[0].y));
    let weave = abs(sin(p.x * scale * 28.0)) * abs(sin(p.z * scale * 28.0));
    let pile = fbm(p * scale * 0.55);
    var tint = mix(base, accent, pile * 0.55 + weave * 0.2);
    tint *= mix(0.88, 1.16, weave);
    return Look(tint, mix(0.68, 0.88, 1.0 - weave), 0.0, 0.14);
}

fn soft_look(p: vec3<f32>) -> Look {
    let base = palette(0u);
    let accent = palette_or(1u, base * vec3<f32>(1.02, 0.96, 0.88));
    let cotton = fbm(p * 4.2);
    let speckle = value_noise_3d(p * 14.0);
    var tint = mix(base, accent, cotton * 0.28);
    tint *= 0.94 + 0.08 * speckle;
    return Look(tint, mix(0.86, 0.96, cotton), 0.0, 0.10);
}

fn marble_look(p: vec3<f32>) -> Look {
    let field = palette(0u);
    let vein = palette_or(1u, vec3<f32>(0.72, 0.52, 0.28));
    let spark = palette_or(2u, vec3<f32>(0.98, 0.92, 0.78));
    let scale = mix(1.6, 3.0, saturate(material.scalars[0].y));
    let q = p * scale;
    let warp = vec3<f32>(fbm(q), fbm(q + 19.2), fbm(q + 4.8));
    let body = fbm(q * 0.65 + warp * 1.7);
    let hair = fbm(q * 2.6 + warp * 0.55);
    let major = smoothstep(0.44, 0.52, body) * (1.0 - smoothstep(0.52, 0.74, body));
    let minor = smoothstep(0.48, 0.56, hair) * 0.45;
    var tint = mix(field, vein, saturate(major + minor));
    tint = mix(tint, spark, major * 0.22);
    let roughness = mix(0.12, 0.28, 1.0 - major);
    return Look(tint, roughness, 0.04, 0.42);
}

fn ornate_look(p: vec3<f32>) -> Look {
    let wood = wood_look(p);
    let gold = palette_or(1u, vec3<f32>(0.95, 0.72, 0.22));
    let gem = palette_or(2u, vec3<f32>(0.22, 0.48, 0.62));
    let uv = fract(vec2<f32>(p.x, p.z) * 0.85) - vec2<f32>(0.5);
    let diamond = abs(uv.x) + abs(uv.y);
    let panel = 1.0 - smoothstep(0.26, 0.33, max(abs(uv.x), abs(uv.y)));
    let frame = saturate(
        (1.0 - smoothstep(0.30, 0.37, max(abs(uv.x), abs(uv.y)))) - panel
    );
    let lattice = smoothstep(0.045, 0.018, abs(fract((uv.x + uv.y) * 7.0) - 0.5))
        * smoothstep(0.40, 0.22, diamond);
    let hy = fract(p.y * 1.65 + 0.12);
    let strap = 1.0 - smoothstep(0.07, 0.13, abs(hy - 0.28));
    let rivet = smoothstep(0.08, 0.03, length(fract(vec2<f32>(p.x, p.y) * 4.4) - vec2<f32>(0.5)));
    let metal = saturate(strap * 0.95 + frame + lattice * 0.75 + rivet * strap);
    var tint = mix(wood.rgb, gem * 1.18, panel * 0.88);
    tint = mix(tint, gold * 1.22, metal);
    let roughness = mix(wood.roughness, 0.18, metal);
    return Look(tint, roughness, metal * 0.82, mix(wood.reflectance, 0.46, metal));
}

fn lava_look(p: vec3<f32>) -> Look {
    let coal = palette(0u);
    let glow = palette_or(1u, vec3<f32>(1.0, 0.32, 0.05));
    let hot = palette_or(2u, vec3<f32>(1.0, 0.86, 0.40));
    let t = globals.time;
    let q = p * 1.85;
    let warp = vec3<f32>(fbm(q + vec3<f32>(t * 0.16, 0.0, 0.0)), fbm(q + 19.2), fbm(q + 4.8));
    let body = fbm(q * 0.68 + warp * 1.65);
    let vein = smoothstep(0.45, 0.54, body) * (1.0 - smoothstep(0.54, 0.72, body));
    let pulse = 0.72 + 0.28 * sin(t * 2.3 + body * 7.0);
    var tint = mix(coal * 0.42, glow, vein);
    tint = mix(tint, hot, pow(vein, 2.2) * pulse);
    tint *= 1.0 + vein * 0.65;
    return Look(tint, mix(0.64, 0.16, vein), 0.06, mix(0.14, 0.40, vein));
}

fn cosmos_look(p: vec3<f32>) -> Look {
    let void_c = palette(0u);
    let nebula = palette_or(1u, vec3<f32>(0.22, 0.06, 0.38));
    let bloom = palette_or(2u, vec3<f32>(0.42, 0.16, 0.62));
    let t = globals.time;
    let n = fbm(p * 0.55 + vec3<f32>(t * 0.03, -t * 0.02, 0.0));
    let n2 = fbm(p * 1.25 + vec3<f32>(-t * 0.04, t * 0.03, 2.1));
    var tint = mix(void_c, nebula, n);
    tint = mix(tint, bloom, smoothstep(0.55, 0.9, n2) * 0.42);
    let band = 0.5 + 0.5 * sin(p.x * 2.2 + p.y * 1.4 + t * 0.4 + n * 2.0);
    tint += bloom * smoothstep(0.75, 1.0, band) * 0.12;
    let glint = pow(saturate(n2), 14.0) * (0.5 + 0.5 * sin(t * 1.4 + n * 6.0));
    tint += vec3<f32>(0.85, 0.78, 0.95) * glint * 0.32;
    return Look(tint, 0.52, 0.04, 0.20);
}

fn scales_look(p: vec3<f32>) -> Look {
    let belly = palette(0u);
    let edge = palette_or(1u, belly * 0.42);
    let irid = palette_or(2u, vec3<f32>(0.32, 0.86, 0.70));
    let q = vec2<f32>(p.x, p.y) * 6.2;
    let row = floor(q.y);
    let odd = select(0.0, 0.5, (i32(row) & 1) == 1);
    let cell = fract(vec2<f32>(q.x + odd, q.y)) - vec2<f32>(0.5);
    let d = length(cell * vec2<f32>(1.0, 1.18));
    let body = 1.0 - smoothstep(0.30, 0.44, d);
    let rim = smoothstep(0.28, 0.36, d) * (1.0 - smoothstep(0.40, 0.50, d));
    var tint = mix(edge, belly, body);
    tint = mix(tint, irid, rim * 0.7);
    return Look(tint, mix(0.44, 0.26, body), 0.18 + rim * 0.28, 0.30);
}

fn lacquer_look(p: vec3<f32>) -> Look {
    let base = palette(0u);
    let accent = palette_or(1u, base * 1.12);
    let peel = fbm(p * 5.8);
    var tint = mix(base, accent, peel * 0.16);
    tint *= 0.90 + 0.14 * peel;
    return Look(tint, mix(0.20, 0.36, peel), 0.05, 0.34);
}

/// Rockadder: terracotta / teal mosaic, ink grout, gold diamond inlay.
fn rockadder_look(p: vec3<f32>) -> Look {
    let terracotta = palette(0u);
    let teal = palette_or(1u, vec3<f32>(0.12, 0.38, 0.36));
    let gold = palette_or(2u, vec3<f32>(0.90, 0.68, 0.22));
    let ink = terracotta * vec3<f32>(0.16, 0.14, 0.12);
    let cream = mix(terracotta, vec3<f32>(0.86, 0.78, 0.62), 0.72);
    let uv = vec2<f32>(p.x + p.z * 0.35, p.y) * 2.6;
    let g1 = lattice(uv);
    let g2 = lattice(rot45(uv));
    let star = min(min(g1.x, g1.y), min(g2.x, g2.y));
    let cell = hash13(floor(vec3<f32>(uv * 2.0, 0.4)));
    let cell2 = hash13(floor(vec3<f32>(rot45(uv) * 2.0, 1.7)));
    var fill = mix(terracotta, teal, step(0.5, cell));
    fill = mix(fill, cream, step(0.82, cell2) * 0.85);
    fill = mix(fill, gold, step(0.92, cell) * 0.7);
    let grout = 1.0 - smoothstep(0.0, 0.045, star);
    var tint = mix(fill, ink, grout * 0.92);
    let star_fill = 1.0 - smoothstep(0.18, 0.26, max(g2.x, g2.y));
    tint = mix(tint, gold, star_fill * step(0.7, cell2) * 0.55);
    let q = fract(uv * 0.45) - vec2<f32>(0.5);
    let diamond = abs(q.x) + abs(q.y);
    let square = max(abs(q.x), abs(q.y));
    let plate = 1.0 - smoothstep(0.20, 0.34, diamond);
    let rim = smoothstep(0.16, 0.22, diamond) * (1.0 - smoothstep(0.30, 0.44, diamond));
    let nest = 1.0 - smoothstep(0.07, 0.14, square);
    let hot = vec3<f32>(1.0, 0.96, 0.72);
    tint = mix(tint, ink, plate * 0.55);
    tint = mix(tint, gold, plate * 0.88);
    tint = mix(tint, hot, nest * 0.85);
    tint += gold * rim * 0.55;
    let metal = saturate(plate * 0.9 + rim);
    return Look(tint, mix(0.42, 0.18, metal), metal * 0.55, mix(0.16, 0.44, metal));
}

fn metal_look(p: vec3<f32>) -> Look {
    let base = palette(0u);
    let accent = palette_or(1u, base * 1.2);
    let streak = abs(sin(p.y * 22.0 + fbm(p) * 3.0));
    let speckle = value_noise_3d(p * 18.0);
    var tint = mix(base, accent, streak * 0.32);
    tint *= 0.72 + 0.38 * speckle;
    return Look(tint, mix(0.20, 0.46, 1.0 - streak), 0.80, 0.50);
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input = pbr_input_from_vertex_output(mesh, is_front, false);
    let p = look_coord(mesh);
    var look = Look(palette(0u), 0.55, 0.0, 0.22);

    switch material.kind {
        case KIND_CLOTH: { look = cloth_look(p); }
        case KIND_SOFT: { look = soft_look(p); }
        case KIND_MARBLE: { look = marble_look(p); }
        case KIND_ORNATE: { look = ornate_look(p); }
        case KIND_LAVA: { look = lava_look(p); }
        case KIND_COSMOS: { look = cosmos_look(p); }
        case KIND_SCALES: { look = scales_look(p); }
        case KIND_LACQUER: { look = lacquer_look(p); }
        case KIND_METAL: { look = metal_look(p); }
        case KIND_ROCKADDER: { look = rockadder_look(p); }
        default: { look = wood_look(p); }
    }

    let rgb = finish(material.kind, look.rgb, mesh.world_normal, mesh.world_position.xyz);
    pbr_input.material.base_color = vec4<f32>(rgb, 1.0);
    pbr_input.material.perceptual_roughness = look.roughness;
    pbr_input.material.metallic = look.metallic;
    pbr_input.material.reflectance = vec3<f32>(look.reflectance);

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb, 1.0), view.color_grading);
}
