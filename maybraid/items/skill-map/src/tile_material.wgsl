//---------------------------------------------------------
// Skill-map tiles: world-space shade, sway, blobby water.
// Kind: 0 fire land, 1 water, 2 fire mark, 3 wave mark, 4 cursor, 5 wave land,
// 6 rock land, 7 cosmos land, 8 rock mark, 9 cosmos mark.
//---------------------------------------------------------

#import bevy_sprite::{
    mesh2d_functions as mesh_functions,
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::{view, globals},
}

struct TileParams {
    // rgb + unused
    tint: vec4<f32>,
    // x kind, y intensity, z seed, w unused
    style: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: TileParams;

const KIND_LAND: f32 = 0.0;
const KIND_WATER: f32 = 1.0;
const KIND_FIRE: f32 = 2.0;
const KIND_WAVE: f32 = 3.0;
const KIND_CURSOR: f32 = 4.0;
const KIND_LAND_WAVE: f32 = 5.0;
const KIND_LAND_ROCK: f32 = 6.0;
const KIND_LAND_COSMO: f32 = 7.0;
const KIND_ROCK: f32 = 8.0;
const KIND_COSMO: f32 = 9.0;

fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = p3 + vec3<f32>(dot(p3, p3.yzx + 33.33));
    return fract((d.x + d.y) * d.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * f0 * (f0 * (f0 * 6.0 - 15.0) + 10.0);
    let a = hash12(i);
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    return value_noise(p) * 0.5
        + value_noise(p * 1.87) * 0.28
        + value_noise(p * 3.41) * 0.15
        + value_noise(p * 6.13) * 0.07;
}

fn tile_kind() -> f32 { return material.style.x; }
fn tile_seed() -> f32 { return material.style.z; }

/// Frequencies stay off the 16-unit grid so value-noise cells do not reprint the tiles.
fn tile_sway(world: vec2<f32>, t: f32) -> vec2<f32> {
    let n = fbm(world * 0.053);
    let n2 = fbm(world * 0.119 + vec2<f32>(19.2, 4.8));
    let n3 = fbm(world * 0.203 + vec2<f32>(3.3, 11.1));
    let phase = dot(world, vec2<f32>(0.07, 0.11));
    let gust = sin(t * 0.65 + phase) * sin(t * 0.23 + phase * 1.7);
    let flutter = sin(t * 1.35 + world.x * 0.22 + world.y * 0.18);
    var offset = vec2<f32>(n - 0.5, n2 - 0.5) * 6.4;
    offset += vec2<f32>(n3 - 0.5, 0.5 - n) * 2.4;
    offset += vec2<f32>(0.92, 0.38) * (gust * 1.15);
    offset += vec2<f32>(flutter, -flutter * 0.65) * 0.7;
    return offset;
}

fn water_pinch(origin: vec2<f32>, world: vec2<f32>, uv: vec2<f32>) -> vec2<f32> {
    let dx = world.x - origin.x;
    let dy = world.y - origin.y;
    let corner = saturate(length(uv - vec2<f32>(0.5, 0.5)) * 2.0);
    let n = fbm(origin * 0.083);
    let n2 = fbm(origin * 0.14 + vec2<f32>(4.2, 1.1));
    let shrink = (0.12 + 0.1 * n) * corner;
    return vec2<f32>(
        origin.x + dx * (1.0 - shrink + (n2 - 0.5) * 0.1),
        origin.y + dy * (1.0 - shrink - (n2 - 0.5) * 0.08),
    );
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_POSITIONS
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var world_pos = mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    let kind = tile_kind();
    let t = globals.time;
    let origin = world_from_local[3].xy;
    if kind > 3.5 && kind < 4.5 {
        let pulse = 0.5 + 0.5 * sin(t * 5.2);
        let scale = 1.0 + (pulse - 0.5) * 0.08;
        world_pos.x = origin.x + (world_pos.x - origin.x) * scale;
        world_pos.y = origin.y + (world_pos.y - origin.y) * scale;
    } else {
#ifdef VERTEX_UVS
        let uv = vertex.uv;
#else
        let uv = vec2<f32>(0.5, 0.5);
#endif
        if kind > 0.5 && kind < 1.5 {
            let pinched = water_pinch(origin, world_pos.xy, uv);
            world_pos.x = pinched.x;
            world_pos.y = pinched.y;
        }
        let sway = tile_sway(world_pos.xy, t);
        world_pos.x += sway.x;
        world_pos.y += sway.y;
    }
    out.world_position = world_pos;
    out.position = mesh_functions::mesh2d_position_world_to_clip(world_pos);
#endif
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh2d_normal_local_to_world(
        vertex.normal,
        vertex.instance_index,
    );
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh2d_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
    return out;
}

fn shade_glimmer(world: vec2<f32>) -> vec3<f32> {
    let t = globals.time;
    let n = fbm(world * 0.024);
    let n2 = fbm(world * 0.048 + vec2<f32>(t * 0.03, -t * 0.025));
    let void_c = vec3<f32>(0.055, 0.03, 0.045);
    let coal = vec3<f32>(0.14, 0.045, 0.03);
    let ember = vec3<f32>(0.38, 0.1, 0.04);
    var color = mix(void_c, coal, n);
    color = mix(color, ember, smoothstep(0.55, 0.9, n2) * 0.4);
    let glint = pow(saturate(n2), 10.0) * (0.45 + 0.55 * sin(t * 1.6 + n * 5.0));
    color += vec3<f32>(1.0, 0.74, 0.38) * glint * 0.35;
    let haze = 0.5 + 0.5 * sin(world.x * 0.05 + world.y * 0.07 + t * 0.7 + n * 3.2);
    color += vec3<f32>(0.5, 0.14, 0.04) * smoothstep(0.8, 1.0, haze) * 0.1;
    return color;
}

fn shade_mist(world: vec2<f32>) -> vec3<f32> {
    let t = globals.time;
    let n = fbm(world * 0.022 + vec2<f32>(t * 0.018, -t * 0.012));
    let n2 = fbm(world * 0.046 + vec2<f32>(-t * 0.02, t * 0.016));
    let void_c = vec3<f32>(0.03, 0.05, 0.08);
    let tide = vec3<f32>(0.07, 0.14, 0.2);
    let bloom = vec3<f32>(0.16, 0.36, 0.4);
    var color = mix(void_c, tide, n);
    color = mix(color, bloom, smoothstep(0.52, 0.88, n2) * 0.38);
    let swell = 0.5 + 0.5 * sin(world.x * 0.055 + world.y * 0.04 + t * 0.85 + n * 2.4);
    let cross = 0.5 + 0.5 * sin(world.x * -0.03 + world.y * 0.07 + t * 0.55 + n2 * 2.0);
    color += vec3<f32>(0.22, 0.62, 0.7) * smoothstep(0.72, 1.0, swell) * 0.16;
    color += vec3<f32>(0.4, 0.75, 0.82) * smoothstep(0.82, 1.0, cross) * 0.08;
    let glint = pow(saturate(n2), 9.0) * (0.4 + 0.6 * sin(t * 1.15 + n * 4.0));
    color += vec3<f32>(0.65, 0.92, 0.98) * glint * 0.22;
    return color;
}

fn shade_water(world: vec2<f32>) -> vec3<f32> {
    let t = globals.time;
    let n = fbm(world * 0.047 + vec2<f32>(t * 0.11, -t * 0.08));
    let n2 = fbm(world * 0.13 + vec2<f32>(-t * 0.07, t * 0.09));
    let band = 0.5 + 0.5 * sin(world.x * 0.19 + world.y * 0.14 + t * 1.15 + n * 3.4);
    let deep = vec3<f32>(0.04, 0.1, 0.26);
    let sheen = vec3<f32>(0.22, 0.48, 0.72);
    let ink = vec3<f32>(0.08, 0.2, 0.42);
    var color = mix(deep, ink, n);
    color = mix(color, sheen, smoothstep(0.7, 0.94, band) * 0.55 * n2);
    return color;
}

fn water_blob(uv: vec2<f32>, world: vec2<f32>) -> f32 {
    let q = uv - vec2<f32>(0.5, 0.5);
    let r = length(q);
    let ang = atan2(q.y, q.x);
    let n = fbm(world * 0.07);
    let n2 = fbm(world * 0.13 + vec2<f32>(2.4, 9.1));
    let radius = 0.4 + 0.07 * sin(ang * 3.0 + n * 5.0) + 0.045 * sin(ang * 5.0 - n2 * 4.0);
    let edge = max(fwidth(r), 0.012);
    return 1.0 - smoothstep(radius - edge * 2.4, radius + edge * 0.8, r);
}

fn shade_fire(uv: vec2<f32>, world: vec2<f32>) -> vec3<f32> {
    var color = shade_glimmer(world);
    let q = uv - vec2<f32>(0.5, 0.5);
    let t = globals.time + tile_seed() * 0.02;
    let warp = fbm(world * 0.17 + vec2<f32>(t * 0.4, -t * 0.3));
    let r = length(q + vec2<f32>(warp - 0.5) * 0.14);
    let ember = 1.0 - smoothstep(0.16, 0.38, r);
    let core = 1.0 - smoothstep(0.0, 0.16, r);
    let coal = vec3<f32>(0.18, 0.05, 0.02);
    let mid = vec3<f32>(0.95, 0.32, 0.06);
    let hot = vec3<f32>(1.0, 0.86, 0.45);
    color = mix(color, coal, ember * 0.85);
    color = mix(color, mid, ember);
    color = mix(color, hot, core * core);
    return color * (1.0 + core * 0.35);
}

fn shade_wave(uv: vec2<f32>, world: vec2<f32>) -> vec3<f32> {
    var color = shade_mist(world);
    let q = uv - vec2<f32>(0.5, 0.5);
    let t = globals.time * 1.15 + tile_seed() * 0.03;
    let warp = fbm(world * 0.14 + vec2<f32>(t * 0.22, -t * 0.18));
    let r = length(q + vec2<f32>(warp - 0.5) * 0.12);
    let pulse = fract(t * 0.22);
    let ring_a = abs(r - (0.12 + pulse * 0.28));
    let ring_b = abs(r - (0.08 + fract(pulse + 0.45) * 0.3));
    let band = 1.0 - smoothstep(0.0, 0.055, ring_a);
    let band2 = 1.0 - smoothstep(0.0, 0.04, ring_b);
    let disk = 1.0 - smoothstep(0.14, 0.48, r);
    let blank = 1.0 - smoothstep(0.0, 0.14, r);
    let glow = vec3<f32>(0.2, 0.72, 0.82);
    let mist = vec3<f32>(0.78, 0.94, 0.98);
    let core = vec3<f32>(0.9, 0.97, 1.0);
    color = mix(color, glow, disk * 0.5);
    color = mix(color, mist, max(band, band2) * disk);
    color = mix(color, core, blank * 0.75);
    return color * (1.0 + blank * 0.2);
}

fn rot45(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(p.x + p.y, p.y - p.x) * 0.70710678;
}

fn lattice(p: vec2<f32>) -> vec2<f32> {
    return abs(fract(p) - 0.5);
}

/// Islamic-tile mosaic: two square lattices, 8-fold stars, terracotta/teal/gold.
fn shade_mosaic(world: vec2<f32>) -> vec3<f32> {
    let t = globals.time;
    let p = world * 0.078;
    let g1 = lattice(p);
    let g2 = lattice(rot45(p));
    let line1 = min(g1.x, g1.y);
    let line2 = min(g2.x, g2.y);
    let star = min(line1, line2);
    let cell = hash12(floor(p * 2.0));
    let cell2 = hash12(floor(rot45(p) * 2.0 + vec2<f32>(3.1, 1.7)));
    let ink = vec3<f32>(0.08, 0.06, 0.05);
    let terracotta = vec3<f32>(0.62, 0.28, 0.16);
    let teal = vec3<f32>(0.12, 0.38, 0.36);
    let cream = vec3<f32>(0.86, 0.78, 0.62);
    let gold = vec3<f32>(0.78, 0.58, 0.22);
    var fill = mix(terracotta, teal, step(0.5, cell));
    fill = mix(fill, cream, step(0.82, cell2) * 0.85);
    fill = mix(fill, gold, step(0.92, cell) * 0.7);
    let grout = 1.0 - smoothstep(0.0, 0.045, star);
    var color = mix(fill, ink, grout * 0.92);
    let diamond = max(g2.x, g2.y);
    let star_fill = 1.0 - smoothstep(0.18, 0.26, diamond);
    color = mix(color, gold, star_fill * step(0.7, cell2) * 0.55);
    let n = fbm(world * 0.03 + vec2<f32>(t * 0.01, -t * 0.008));
    color *= 0.88 + n * 0.18;
    return color;
}

/// Deep purple celestial field: nebula bands and smooth star glints.
fn shade_cosmos(world: vec2<f32>) -> vec3<f32> {
    let t = globals.time;
    let n = fbm(world * 0.018 + vec2<f32>(t * 0.012, -t * 0.01));
    let n2 = fbm(world * 0.042 + vec2<f32>(-t * 0.016, t * 0.014));
    let void_c = vec3<f32>(0.04, 0.02, 0.09);
    let nebula = vec3<f32>(0.22, 0.06, 0.38);
    let bloom = vec3<f32>(0.42, 0.16, 0.62);
    var color = mix(void_c, nebula, n);
    color = mix(color, bloom, smoothstep(0.55, 0.9, n2) * 0.42);
    let band = 0.5 + 0.5 * sin(world.x * 0.04 + world.y * 0.09 + t * 0.35 + n * 2.0);
    color += vec3<f32>(0.55, 0.28, 0.72) * smoothstep(0.75, 1.0, band) * 0.14;
    let meridian = 0.5 + 0.5 * sin(world.x * 0.11 - world.y * 0.03 + t * 0.2);
    color += vec3<f32>(0.35, 0.18, 0.55) * smoothstep(0.88, 1.0, meridian) * 0.08;
    let glint = pow(saturate(n2), 12.0) * (0.5 + 0.5 * sin(t * 1.4 + n * 6.0));
    color += vec3<f32>(0.95, 0.82, 1.0) * glint * 0.55;
    let glint2 = pow(saturate(n), 14.0);
    color += vec3<f32>(0.7, 0.9, 1.0) * glint2 * 0.25;
    return color;
}

fn shade_rock(uv: vec2<f32>, world: vec2<f32>) -> vec3<f32> {
    var color = shade_mosaic(world);
    let q = uv - vec2<f32>(0.5, 0.5);
    let diamond = abs(q.x) + abs(q.y);
    let square = max(abs(q.x), abs(q.y));
    let plate = 1.0 - smoothstep(0.2, 0.34, diamond);
    let rim = smoothstep(0.16, 0.22, diamond) * (1.0 - smoothstep(0.3, 0.44, diamond));
    let nest = 1.0 - smoothstep(0.07, 0.14, square);
    let ink = vec3<f32>(0.04, 0.02, 0.01);
    let gold = vec3<f32>(1.0, 0.84, 0.16);
    let hot = vec3<f32>(1.0, 0.96, 0.72);
    color = mix(color, ink, plate * 0.95);
    color = mix(color, gold, plate * 0.92);
    color = mix(color, hot, nest);
    color += gold * rim * 1.2;
    return color * (1.0 + nest * 0.5);
}

fn shade_cosmo(uv: vec2<f32>, world: vec2<f32>) -> vec3<f32> {
    var color = shade_cosmos(world);
    let q = uv - vec2<f32>(0.5, 0.5);
    let r = length(q);
    let ang = atan2(q.y, q.x);
    let spoke = abs(fract(ang / 6.2831853 * 6.0) - 0.5);
    let star = 1.0 - smoothstep(0.0, 0.16, r + spoke * 0.28);
    let flare = 1.0 - smoothstep(0.08, 0.42, r);
    let core = 1.0 - smoothstep(0.0, 0.12, r);
    let violet = vec3<f32>(0.62, 0.28, 1.0);
    let gold = vec3<f32>(1.0, 0.78, 0.35);
    let white = vec3<f32>(0.96, 0.9, 1.0);
    color = mix(color, violet, flare * 0.55);
    color = mix(color, gold, star * 0.7);
    color = mix(color, white, core * 0.85);
    return color * (1.0 + core * 0.35);
}

fn shade_cursor(uv: vec2<f32>) -> vec3<f32> {
    let q = uv - vec2<f32>(0.5, 0.5);
    let r = length(q);
    let core = 1.0 - smoothstep(0.0, 0.34, r);
    let ring = smoothstep(0.42, 0.5, r) * (1.0 - smoothstep(0.54, 0.7, r));
    let ivory = vec3<f32>(1.0, 0.93, 0.78);
    let gold = vec3<f32>(1.0, 0.62, 0.18);
    return ivory * (0.35 + core * 0.9) + gold * ring;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let kind = tile_kind();
    let world = in.world_position.xy;
    var color: vec3<f32>;
    var alpha = 1.0;
    // Kind is a material uniform (one draw per tile class). Do not pay
    // shade_glimmer's two FBMs as a discarded default — fire marks already
    // call it inside shade_fire.
    if kind > 0.5 && kind < 1.5 {
        color = shade_water(world);
        alpha = water_blob(in.uv, world);
    } else if kind > 1.5 && kind < 2.5 {
        color = shade_fire(in.uv, world);
    } else if kind > 2.5 && kind < 3.5 {
        color = shade_wave(in.uv, world);
    } else if kind > 3.5 && kind < 4.5 {
        color = shade_cursor(in.uv);
    } else if kind > 4.5 && kind < 5.5 {
        color = shade_mist(world);
    } else if kind > 5.5 && kind < 6.5 {
        color = shade_mosaic(world);
    } else if kind > 6.5 && kind < 7.5 {
        color = shade_cosmos(world);
    } else if kind > 7.5 && kind < 8.5 {
        color = shade_rock(in.uv, world);
    } else if kind > 8.5 {
        color = shade_cosmo(in.uv, world);
    } else {
        color = shade_glimmer(world);
    }
    color *= material.tint.xyz * material.style.y;
    return vec4<f32>(color, alpha);
}
