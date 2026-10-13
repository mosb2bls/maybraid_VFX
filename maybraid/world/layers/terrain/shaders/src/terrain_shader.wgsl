//---------------------------------------------------------
// Durham terrain: PBR with world-space palette noise.
//
// Procedural recessed cracks:
//   - Near: slope-aware, refined world-space POM.
//   - Far: cheap procedural crack appearance using the same
//          underlying height field, with distance-broadened
//          visual cracks for stable terrain continuity.
//
// Optional world-space grass turf:
//   - Persistent underlayer, two compact blade fields,
//     anchored roots, and time-driven bend.
//   - Authored coverage scales moisture noise so barren
//     ground cannot become meadow on its own.
//   - Grass POM uses its own 35–50 m range; cracks keep
//     the 60–150 m fade plus the far pattern.
//
// Preserves original material bindings, palette, fog,
// and prepass outlines.
//
// Color-only relief: no silhouette change, displaced depth,
// or self-shadow ray.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::{view, lights, globals},
    prepass_utils::prepass_depth,
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

#ifdef DISTANCE_FOG
#import bevy_pbr::mesh_view_bindings::fog
#endif


fn with_distance_fog(
    color: vec4<f32>,
    world_position: vec3<f32>,
    frag_xy: vec2<f32>
) -> vec4<f32> {
#ifdef DISTANCE_FOG
    return fns::apply_fog(
        fog,
        color,
        world_position,
        view.world_position.xyz,
        frag_xy
    );
#else
    return color;
#endif
}


//---------------------------------------------------------
// Terrain palette
//---------------------------------------------------------

struct DurhamSwatch {
    left: vec4<f32>,
    right: vec4<f32>,
    swatch_meta: vec4<f32>,
}

struct DurhamTerrainBand {
    config: vec4<f32>,
    band_scale: vec4<f32>,
    swatches: array<DurhamSwatch, 8>,
}

struct DurhamTerrainNoise {
    regional_blend: vec4<f32>,
    global_seed: vec4<f32>,
    bands: array<DurhamTerrainBand, 4>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> terrain_noise: DurhamTerrainNoise;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> style_params: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> base_color: vec4<f32>;

struct DurhamTerrainGrass {
    params: vec4<f32>,
    lush_root: vec4<f32>,
    lush_mid: vec4<f32>,
    lush_tip: vec4<f32>,
    dry_root: vec4<f32>,
    dry_mid: vec4<f32>,
    dry_tip: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> grass: DurhamTerrainGrass;


fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}


fn spread_noise(t: f32, amount: f32) -> f32 {
    let x = saturate(t);
    return saturate(0.5 + (x - 0.5) * amount);
}


fn hash21(p: vec2<f32>) -> f32 {
    return fract(
        sin(dot(p, vec2<f32>(127.1, 311.7)))
        * 43758.5453123
    );
}


fn value_noise_2d(p: vec2<f32>, seed: f32) -> f32 {
    let i = floor(p);
    let nf = fract(p);
    let u = nf * nf * (3.0 - 2.0 * nf);

    let s = vec2<f32>(
        seed * 17.13,
        seed * 31.71
    );

    let a = hash21(i + vec2<f32>(0.0, 0.0) + s);
    let b = hash21(i + vec2<f32>(1.0, 0.0) + s);
    let c = hash21(i + vec2<f32>(0.0, 1.0) + s);
    let d = hash21(i + vec2<f32>(1.0, 1.0) + s);

    return mix(
        mix(a, b, u.x),
        mix(c, d, u.x),
        u.y
    );
}


fn fbm_2d(
    p: vec2<f32>,
    seed: f32,
    amp: f32,
    freq: f32
) -> f32 {
    var sum = 0.0;
    var a = amp;
    var f = freq;

    for (var octave = 0; octave < 2; octave = octave + 1) {
        sum += value_noise_2d(
            p * f,
            seed + f32(octave) * 19.17
        ) * a;

        f *= 2.03;
        a *= 0.5;
    }

    // Preserve approximately the four-octave amplitude
    // while evaluating half as many frequencies.
    return saturate(sum * 1.25);
}


fn domain_warp_offset_2d(
    p: vec2<f32>,
    seed: f32,
    amp: f32,
    freq: f32
) -> vec2<f32> {
    let qx = fbm_2d(
        p + vec2<f32>(17.1, 3.7),
        seed + 10.0,
        amp,
        freq * 0.35
    );

    let qy = fbm_2d(
        p + vec2<f32>(8.3, 29.4),
        seed + 20.0,
        amp,
        freq * 0.35
    );

    return (
        vec2<f32>(qx, qy)
        - vec2<f32>(0.5)
    ) * 260.0;
}


fn palette_noise_2d(
    p: vec2<f32>,
    seed: f32,
    amp: f32,
    freq: f32
) -> f32 {
    let warp = domain_warp_offset_2d(
        p,
        seed,
        amp,
        freq
    );

    let n = fbm_2d(
        p + warp,
        seed + 3.0,
        amp,
        freq
    );

    return spread_noise(n, 1.75);
}


fn swatch_linear(
    sw: DurhamSwatch,
    along: f32
) -> vec3<f32> {
    return mix(
        sw.left.xyz,
        sw.right.xyz,
        saturate(along)
    );
}


/// Half-width in continuous `u = t*8` around each
/// integer seam for joint-only blending.
const SWATCH_INDEX_SOFTNESS: f32 = 0.12;


fn swatch_sample(
    t_noise: f32,
    band: DurhamTerrainBand
) -> vec3<f32> {
    let u = min(
        saturate(t_noise) * 8.0,
        8.0 - 1e-4
    );

    let e = SWATCH_INDEX_SOFTNESS;
    let i = min(i32(floor(u)), 7);
    let f = fract(u);

    if (i > 0 && f <= e) {
        let t = smoothstep(
            0.0,
            1.0,
            (f + e) / (2.0 * e)
        );

        return mix(
            swatch_linear(
                band.swatches[i - 1],
                1.0
            ),
            swatch_linear(
                band.swatches[i],
                0.0
            ),
            t
        );
    }

    if (i < 7 && f >= 1.0 - e) {
        let t = smoothstep(
            0.0,
            1.0,
            (f - 1.0 + e) / (2.0 * e)
        );

        return mix(
            swatch_linear(
                band.swatches[i],
                1.0
            ),
            swatch_linear(
                band.swatches[i + 1],
                0.0
            ),
            t
        );
    }

    return swatch_linear(
        band.swatches[i],
        f
    );
}


fn band_color(
    p: vec2<f32>,
    band: DurhamTerrainBand
) -> vec3<f32> {
    let seed = band.config.x;

    let t = palette_noise_2d(
        p,
        seed,
        band.band_scale.y,
        band.band_scale.x
    );

    return swatch_sample(t, band);
}


const WORLD_COLOR_SHIFT: vec2<f32> =
    vec2<f32>(1298.0, 18229.0);


fn ground_color_at(p: vec2<f32>) -> vec3<f32> {
    var acc = vec3<f32>(0.0);
    var wsum = 0.0;

    for (var bi = 0; bi < 4; bi = bi + 1) {
        let band = terrain_noise.bands[bi];

        let w = spread_noise(
            max(band.band_scale.z, 0.0),
            1.35
        );

        acc += band_color(p, band) * w;
        wsum += w;
    }

    return acc / max(wsum, 1e-6);
}


//---------------------------------------------------------
// Procedural terrain relief
//---------------------------------------------------------

// Physical crack geometry.
const CRACK_CELL_M: f32 = 1.8;
const CRACK_WIDTH_M: f32 = 0.10;
const CRACK_DEPTH_M: f32 = 0.18;
const CRACK_DARK: f32 = 0.55;


//---------------------------------------------------------
// POM distance LOD
//---------------------------------------------------------

// Full POM inside this range.
const CRACK_POM_FADE_START_M: f32 = 60.0;

// POM completely gone by this range.
const CRACK_POM_FADE_END_M: f32 = 150.0;


//---------------------------------------------------------
// Far visual crack LOD
//---------------------------------------------------------

// Once true geometric cracks approach subpixel size,
// broaden their *visual* footprint to retain the same
// terrain character without ray marching.
const CRACK_FAR_WIDTH_M: f32 = 0.35;

// Begin widening the cheap representation here.
const CRACK_FAR_WIDTH_START_M: f32 = 60.0;

// Reach maximum visual width here.
const CRACK_FAR_WIDTH_END_M: f32 = 300.0;


//---------------------------------------------------------
// POM quality
//---------------------------------------------------------

const CRACK_NORMAL_EPS_M: f32 = 0.008;

const CRACK_MIN_STEPS: i32 = 24;
const CRACK_MAX_STEPS: i32 = 96;
const CRACK_REFINE_STEPS: i32 = 5;


//---------------------------------------------------------
// Relief hashing
//---------------------------------------------------------

fn relief_hash(
    p: vec2<i32>,
    seed: u32
) -> u32 {
    var v =
          bitcast<u32>(p.x) * 0x8da6b343u
        ^ bitcast<u32>(p.y) * 0xd8163841u
        ^ seed;

    v = (v ^ (v >> 16u)) * 0x7feb352du;
    v = (v ^ (v >> 15u)) * 0x846ca68bu;

    return v ^ (v >> 16u);
}


fn relief_random(
    p: vec2<i32>,
    seed: u32
) -> f32 {
    return f32(
        relief_hash(p, seed) >> 8u
    ) * (1.0 / 16777216.0);
}


//---------------------------------------------------------
// Raw Voronoi edge distance
//---------------------------------------------------------

// Returns approximate distance in metres to the
// Voronoi cell boundary.
//
// Keeping this separate lets the same procedural
// geometry drive:
//   1. physical near-field POM
//   2. broadened far-field coloration
fn relief_edge_distance(
    p: vec2<f32>
) -> f32 {
    let domain = p / CRACK_CELL_M;

    let cell = vec2<i32>(
        floor(domain)
    );

    let local = fract(domain);

    var first = 1e10;
    var second = 1e10;

    for (var z = -1; z <= 1; z = z + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let offset = vec2<i32>(x, z);
            let id = cell + offset;

            // Limited jitter keeps the local
            // neighborhood well behaved.
            let jitter = vec2<f32>(
                relief_random(id, 71u),
                relief_random(id, 193u)
            );

            let feature =
                vec2<f32>(offset)
                + vec2<f32>(0.3)
                + 0.4 * jitter;

            let delta =
                feature - local;

            let d2 =
                dot(delta, delta);

            if (d2 < first) {
                second = first;
                first = d2;
            } else {
                second =
                    min(second, d2);
            }
        }
    }

    return (
        sqrt(second)
        - sqrt(first)
    ) * CRACK_CELL_M;
}


//---------------------------------------------------------
// Near physical height field
//---------------------------------------------------------

fn relief_height(
    p: vec2<f32>
) -> f32 {
    let edge =
        relief_edge_distance(p);

    return smoothstep(
        0.0,
        CRACK_WIDTH_M,
        edge
    );
}


//---------------------------------------------------------
// Far visual-only height field
//---------------------------------------------------------

fn relief_visual_height(
    p: vec2<f32>,
    distance_m: f32
) -> f32 {
    let edge =
        relief_edge_distance(p);

    let width_t = smoothstep(
        CRACK_FAR_WIDTH_START_M,
        CRACK_FAR_WIDTH_END_M,
        distance_m
    );

    let visual_width = mix(
        CRACK_WIDTH_M,
        CRACK_FAR_WIDTH_M,
        width_t
    );

    return smoothstep(
        0.0,
        visual_width,
        edge
    );
}


//---------------------------------------------------------
// POM ray trace
//---------------------------------------------------------

struct ReliefHit {
    offset: vec3<f32>,
    height: f32,
}


// Solve:
//
// N.dot(Q-P) + D*(1-h(Q.xz)) = 0
//
// on:
//
// Q = P - V*s
//
// N is the unsoftened macro normal.
// D is fixed and never distance-scaled.
fn trace_relief(
    p: vec2<f32>,
    N: vec3<f32>,
    V: vec3<f32>
) -> ReliefHit {
    // Limit ray travel at extreme grazing angles.
    let ray =
        -V * (
            CRACK_DEPTH_M
            / max(dot(N, V), 0.06)
        );

    let horizontal_travel =
        length(ray.xz);

    // Attempt at least four samples across the
    // nominal physical crack width.
    let requested = ceil(
        horizontal_travel
        / (CRACK_WIDTH_M * 0.25)
    );

    let steps = i32(
        clamp(
            requested,
            f32(CRACK_MIN_STEPS),
            f32(CRACK_MAX_STEPS)
        )
    );

    let h0 =
        relief_height(p);

    if (h0 >= 1.0) {
        return ReliefHit(
            vec3<f32>(0.0),
            h0
        );
    }

    var lo = 0.0;
    var hi = 1.0;

    var f_lo =
        h0 - 1.0;

    var f_hi = 0.0;

    for (
        var i = 1;
        i <= CRACK_MAX_STEPS;
        i = i + 1
    ) {
        if (i > steps) {
            break;
        }

        let t =
            f32(i) / f32(steps);

        let f =
            relief_height(
                p + ray.xz * t
            )
            - (1.0 - t);

        if (f >= 0.0) {
            hi = t;
            f_hi = f;
            break;
        }

        lo = t;
        f_lo = f;
    }

    for (
        var i = 0;
        i < CRACK_REFINE_STEPS;
        i = i + 1
    ) {
        let mid =
            0.5 * (lo + hi);

        let f =
            relief_height(
                p + ray.xz * mid
            )
            - (1.0 - mid);

        if (f >= 0.0) {
            hi = mid;
            f_hi = f;
        } else {
            lo = mid;
            f_lo = f;
        }
    }

    let fraction =
        clamp(
            -f_lo
            / max(
                f_hi - f_lo,
                1e-7
            ),
            0.0,
            1.0
        );

    let t =
        mix(lo, hi, fraction);

    let offset =
        ray * t;

    return ReliefHit(
        offset,
        relief_height(
            p + offset.xz
        )
    );
}


//---------------------------------------------------------
// Relief normal
//---------------------------------------------------------

fn relief_gradient(
    p: vec2<f32>
) -> vec2<f32> {
    let e =
        CRACK_NORMAL_EPS_M;

    let dx =
        relief_height(
            p + vec2<f32>(e, 0.0)
        )
        -
        relief_height(
            p - vec2<f32>(e, 0.0)
        );

    let dz =
        relief_height(
            p + vec2<f32>(0.0, e)
        )
        -
        relief_height(
            p - vec2<f32>(0.0, e)
        );

    return vec2<f32>(
        dx,
        dz
    ) / (2.0 * e);
}


//---------------------------------------------------------
// Dense stylized turf
//---------------------------------------------------------

// Two blades per 0.12 m square (~139 / m^2). Width, length, and
// sway must stay inside one cell; do not raise GRASS_SWAY_M
// above 9 mm without neighbor-cell evaluation.
const GRASS_PATCH_SCALE_M: f32 = 7.0;
const GRASS_PATCH_LOW: f32 = 0.12;
const GRASS_PATCH_HIGH: f32 = 0.30;
const GRASS_REGION_SCALE_M: f32 = 80.0;
const GRASS_REGION_DRY: f32 = 0.28;
const GRASS_REGION_LUSH: f32 = 0.72;
const GRASS_DRY_THRESHOLD: f32 = 0.95;
const GRASS_CELL_M: f32 = 0.12;
const GRASS_HEIGHT_M: f32 = 0.095;
const GRASS_TURF_HEIGHT_M: f32 = 0.010;
const GRASS_HALF_WIDTH_M: f32 = 0.022;
const GRASS_POM_START_M: f32 = 35.0;
const GRASS_POM_END_M: f32 = 50.0;
const GRASS_MIN_STEPS: i32 = 8;
const GRASS_MAX_STEPS: i32 = 32;
const GRASS_REFINE_STEPS: i32 = 3;
const GRASS_SURFACE_REACH_M: f32 = 0.06;
const GRASS_RAY_DENOM_MIN: f32 = 0.10;
const GRASS_NORMAL_EPS_M: f32 = 0.002;
const GRASS_FAR_COVERAGE: f32 = 0.96;
const GRASS_NORMAL_STRENGTH: f32 = 0.22;
const GRASS_MAX_TILT: f32 = 0.28;
const GRASS_WIND_SPEED: f32 = 1.6;
const GRASS_SWAY_M: f32 = 0.009;
const GRASS_TURF_TIP_MIX: f32 = 0.22;
const GRASS_CRACK_SKIP: f32 = 1e-3;


struct GrassBladeCache {
    cell: vec2<i32>,
    r0: f32,
    r1: f32,
    r2: f32,
    wind: f32,
    primed: u32,
}


struct GrassPalette {
    root: vec3<f32>,
    mid: vec3<f32>,
    tip: vec3<f32>,
    turf: vec3<f32>,
}


struct GrassHit {
    offset: vec3<f32>,
    height: f32,
    found: bool,
}


var<private> grass_cache_a: GrassBladeCache;
var<private> grass_cache_b: GrassBladeCache;
var<private> grass_wind_time: f32;


fn grass_authored_coverage() -> f32 {
    return saturate(grass.params.x);
}


fn grass_tint() -> vec3<f32> {
    return mix(
        vec3<f32>(1.0),
        base_color.rgb,
        saturate(grass.params.y)
    );
}


fn grass_wind_color_amount() -> f32 {
    return grass.params.z;
}


fn grass_prepare_eval() {
    grass_wind_time = globals.time * GRASS_WIND_SPEED;
    grass_cache_a.primed = 0u;
    grass_cache_b.primed = 0u;
}


fn grass_value_noise(
    q: vec2<f32>,
    seed: u32
) -> f32 {
    let cell = vec2<i32>(floor(q));
    let f = fract(q);
    let u = f * f * (3.0 - 2.0 * f);

    let a = relief_random(cell, seed);
    let b = relief_random(cell + vec2<i32>(1, 0), seed);
    let c = relief_random(cell + vec2<i32>(0, 1), seed);
    let d = relief_random(cell + vec2<i32>(1, 1), seed);

    return mix(
        mix(a, b, u.x),
        mix(c, d, u.x),
        u.y
    );
}


fn grass_patch_noise(p: vec2<f32>) -> f32 {
    return grass_value_noise(
        p / GRASS_PATCH_SCALE_M,
        701u
    );
}


fn grass_region(p: vec2<f32>) -> f32 {
    let moisture = grass_value_noise(
        p / GRASS_REGION_SCALE_M,
        2719u
    );

    return smoothstep(
        GRASS_REGION_DRY,
        GRASS_REGION_LUSH,
        moisture
    );
}


fn grass_threshold(lushness: f32) -> f32 {
    return mix(
        GRASS_DRY_THRESHOLD,
        GRASS_PATCH_LOW,
        lushness
    );
}


// Traveling waves in world space. Mask and root positions never
// move. Time is cached per fragment in grass_prepare_eval.
fn grass_wind_at(root: vec2<f32>) -> f32 {
    let phase = dot(root, vec2<f32>(0.58, 0.34));

    return 0.72 * sin(phase - grass_wind_time)
        + 0.28 * sin(phase * 2.3 - grass_wind_time * 1.7);
}


fn grass_load_cache(field: i32) -> GrassBladeCache {
    if (field == 0) {
        return grass_cache_a;
    }

    return grass_cache_b;
}


fn grass_store_cache(field: i32, sample: GrassBladeCache) {
    if (field == 0) {
        grass_cache_a = sample;
    } else {
        grass_cache_b = sample;
    }
}


fn grass_cached_blade(
    cell: vec2<i32>,
    shift: vec2<f32>,
    seed: u32,
    field: i32
) -> GrassBladeCache {
    let cached = grass_load_cache(field);

    if (
        cached.primed == 1u
        && cached.cell.x == cell.x
        && cached.cell.y == cell.y
    ) {
        return cached;
    }

    let r0 = relief_random(cell, seed);
    let r1 = relief_random(cell, seed + 12u);
    let r2 = relief_random(cell, seed + 28u);
    let angle = r0 * 6.28318530718;
    let axis = vec2<f32>(cos(angle), sin(angle));
    let half_length = mix(0.040, 0.047, r1);
    let center = (vec2<f32>(r1, r2) - vec2<f32>(0.5)) * 0.004;

    // World-space root: cell is in the shifted lattice.
    let root = (vec2<f32>(cell) + vec2<f32>(0.5)) * GRASS_CELL_M
        - shift
        + center
        - axis * half_length;

    let sample = GrassBladeCache(
        cell,
        r0,
        r1,
        r2,
        grass_wind_at(root),
        1u
    );

    grass_store_cache(field, sample);
    return sample;
}


// One compact curved blade. Neighbor-cell search is unnecessary:
// max support radius sqrt(.047^2 + (.022+.009)^2) + .0029 < .060.
fn grass_blade_height(
    p: vec2<f32>,
    shift: vec2<f32>,
    seed: u32,
    field: i32
) -> f32 {
    let q = (p + shift) / GRASS_CELL_M;
    let cell = vec2<i32>(floor(q));
    let sample = grass_cached_blade(cell, shift, seed, field);
    let center = (vec2<f32>(sample.r1, sample.r2) - vec2<f32>(0.5)) * 0.004;
    let local = (fract(q) - vec2<f32>(0.5)) * GRASS_CELL_M - center;
    let angle = sample.r0 * 6.28318530718;
    let axis = vec2<f32>(cos(angle), sin(angle));
    let across = vec2<f32>(-axis.y, axis.x);
    let half_length = mix(0.040, 0.047, sample.r1);
    let along = (dot(local, axis) + half_length) / (2.0 * half_length);

    if (along <= 0.0 || along >= 1.0) {
        return 0.0;
    }

    let bend = GRASS_SWAY_M * sample.wind * along * along;
    let width = GRASS_HALF_WIDTH_M * mix(1.0, 0.16, along);
    let lateral = 1.0 - smoothstep(
        0.0,
        width,
        abs(dot(local, across) - bend)
    );
    let rise = smoothstep(0.0, 0.36, along);
    let tip = 1.0 - smoothstep(0.70, 1.0, along);

    return GRASS_HEIGHT_M
        * mix(0.70, 1.0, sample.r2)
        * rise
        * tip
        * lateral;
}


// Two-lattice speckle for the cheap mat. This is what remains
// when blades are subpixel; it must not depend on the march.
fn grass_cell_speckle(p: vec2<f32>) -> f32 {
    let a = relief_random(
        vec2<i32>(floor(p / GRASS_CELL_M)),
        449u
    );
    let b = relief_random(
        vec2<i32>(floor(
            (p + vec2<f32>(0.061, 0.053)) / GRASS_CELL_M
        )),
        450u
    );

    return mix(a, b, 0.5);
}


// Blades only. The 1 cm turf is a color underlayer, not a
// height floor — marching to it made every meadow ray pay
// the full step budget.
fn grass_blades(p: vec2<f32>) -> f32 {
    let a = grass_blade_height(
        p,
        vec2<f32>(0.0),
        811u,
        0
    );

    let b = grass_blade_height(
        p,
        vec2<f32>(0.061, 0.053),
        1201u,
        1
    );

    return max(a, b);
}


fn grass_height(p: vec2<f32>, meadow: f32) -> f32 {
    if (meadow <= 0.0) {
        return 0.0;
    }

    return meadow * grass_blades(p);
}


// March from above the terrain toward the original surface so
// blades rise out of the ground. Height zero is empty ground.
fn trace_grass(
    p: vec2<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    meadow: f32
) -> GrassHit {
    let denom = max(dot(N, V), GRASS_RAY_DENOM_MIN);
    let top = V * (GRASS_HEIGHT_M / denom);
    let requested = ceil(length(top.xz) / (GRASS_HALF_WIDTH_M * 0.5));
    let steps = i32(clamp(
        requested,
        f32(GRASS_MIN_STEPS),
        f32(GRASS_MAX_STEPS)
    ));

    var lo = 0.0;
    var hi = 1.0;
    var crossed = false;

    for (var i = 1; i <= GRASS_MAX_STEPS; i = i + 1) {
        if (i > steps) {
            break;
        }

        let t = f32(i) / f32(steps);
        let offset = top * (1.0 - t);
        let height = grass_height(p + offset.xz, meadow);
        let f = height - GRASS_HEIGHT_M * (1.0 - t);

        if (f >= 0.0 && height > 1e-6) {
            hi = t;
            crossed = true;
            break;
        }

        lo = t;
    }

    if (!crossed) {
        return GrassHit(vec3<f32>(0.0), 0.0, false);
    }

    for (var i = 0; i < GRASS_REFINE_STEPS; i = i + 1) {
        let t = 0.5 * (lo + hi);
        let offset = top * (1.0 - t);
        let f = grass_height(p + offset.xz, meadow)
            - GRASS_HEIGHT_M * (1.0 - t);

        if (f >= 0.0) {
            hi = t;
        } else {
            lo = t;
        }
    }

    let offset = top * (1.0 - hi);

    return GrassHit(
        offset,
        grass_height(p + offset.xz, meadow),
        true
    );
}


fn grass_gradient(p: vec2<f32>, meadow: f32) -> vec2<f32> {
    let e = GRASS_NORMAL_EPS_M;

    return vec2<f32>(
        grass_height(p + vec2<f32>(e, 0.0), meadow)
            - grass_height(p - vec2<f32>(e, 0.0), meadow),
        grass_height(p + vec2<f32>(0.0, e), meadow)
            - grass_height(p - vec2<f32>(0.0, e), meadow)
    ) / (2.0 * e);
}


fn grass_capped_normal(
    soft_n: vec3<f32>,
    gradient: vec2<f32>,
    compression: f32
) -> vec3<f32> {
    let raw = gradient * (GRASS_NORMAL_STRENGTH * compression);
    let tilt = length(raw);
    let capped = raw * (
        min(tilt, GRASS_MAX_TILT) / max(tilt, 1e-6)
    );

    let detailed = normalize(
        soft_n - vec3<f32>(capped.x, 0.0, capped.y)
    );

    return normalize(mix(soft_n, detailed, 0.65));
}


fn grass_palette_at(
    lushness: f32,
    tint: vec3<f32>,
    wind_term: f32
) -> GrassPalette {
    let root = mix(
        grass.dry_root.xyz,
        grass.lush_root.xyz,
        lushness
    );

    let mid = mix(
        grass.dry_mid.xyz,
        grass.lush_mid.xyz,
        lushness
    );

    let tip = mix(
        grass.dry_tip.xyz,
        grass.lush_tip.xyz,
        lushness
    );

    let turf = mix(
        mid,
        tip,
        GRASS_TURF_TIP_MIX + wind_term
    ) * tint;

    return GrassPalette(
        root * tint,
        mid * tint,
        tip * tint,
        turf
    );
}


//---------------------------------------------------------
// Prepass outline helpers
//---------------------------------------------------------

fn depth_at(
    pos: vec4<f32>
) -> f32 {
    return prepass_depth(
        pos,
        0
    );
}


fn depth_edge_laplacian(
    pos: vec4<f32>,
    strength: f32
) -> f32 {
    let d0 =
        depth_at(pos);

    let dR =
        depth_at(
            pos
            + vec4<f32>(
                1.0,
                0.0,
                0.0,
                0.0
            )
        );

    let dL =
        depth_at(
            pos
            + vec4<f32>(
                -1.0,
                0.0,
                0.0,
                0.0
            )
        );

    let dU =
        depth_at(
            pos
            + vec4<f32>(
                0.0,
                1.0,
                0.0,
                0.0
            )
        );

    let dD =
        depth_at(
            pos
            + vec4<f32>(
                0.0,
                -1.0,
                0.0,
                0.0
            )
        );

    let lap =
        abs(
            (dR + dL + dU + dD)
            - (4.0 * d0)
        );

    let scale =
        max(
            0.05,
            abs(1000.0 * d0)
        );

    return saturate(
        (lap / scale)
        * strength
    );
}


//---------------------------------------------------------
// Fragment
//---------------------------------------------------------

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput =
        pbr_input_new();

    let double_sided =
        (
            pbr_input.material.flags
            &
            STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT
        ) != 0u;

    pbr_input.world_normal =
        fns::prepare_world_normal(
            normalize(mesh.world_normal),
            double_sided,
            is_front
        );


    //-----------------------------------------------------
    // Macro lighting normal
    //-----------------------------------------------------

    let soften =
        saturate(style_params.x);

    let soft_n =
        normalize(
            mix(
                pbr_input.world_normal,
                vec3<f32>(0.0, 1.0, 0.0),
                soften
            )
        );


    //-----------------------------------------------------
    // View / world coordinates
    //-----------------------------------------------------

    pbr_input.is_orthographic =
        view.clip_from_view[3].w == 1.0;

    let V =
        fns::calculate_view(
            mesh.world_position,
            pbr_input.is_orthographic
        );

    let P =
        mesh.world_position.xyz;

    let macro_n =
        normalize(
            pbr_input.world_normal
        );

    let relief_p =
        P.xz;

    let camera_distance =
        length(
            P
            - view.world_position.xyz
        );


    //-----------------------------------------------------
    // Relief weights
    //-----------------------------------------------------

    // Evaluate derivatives before entering the
    // ray-marching branch.
    let footprint =
        max(
            length(dpdx(relief_p)),
            length(dpdy(relief_p))
        );

    // Physical POM loses usefulness once the real
    // 10 cm crack becomes unresolved.
    let resolution_w =
        1.0
        -
        smoothstep(
            CRACK_WIDTH_M * 0.5,
            CRACK_WIDTH_M * 1.25,
            footprint
        );

    // Explicit distance LOD for the expensive ray trace.
    let pom_distance_w =
        1.0
        -
        smoothstep(
            CRACK_POM_FADE_START_M,
            CRACK_POM_FADE_END_M,
            camera_distance
        );

    // Keep relief usable somewhat closer to grazing.
    let facing_w =
        smoothstep(
            0.025,
            0.10,
            dot(macro_n, V)
        );

    // World-XZ relief is intended for terrain,
    // not cliffs / overhangs.
    let slope_w =
        smoothstep(
            0.15,
            0.35,
            macro_n.y
        );

    // Only POM is resolution/distance limited.
    let pom_w =
        pom_distance_w
        * resolution_w
        * facing_w
        * slope_w;


    //-----------------------------------------------------
    // Base palette
    //-----------------------------------------------------

    let base_palette =
        ground_color_at(
            relief_p
            + WORLD_COLOR_SHIFT
        );

    let base_ground =
        base_palette
        * base_color.rgb;


    //-----------------------------------------------------
    // Cheap crack representation
    //
    // This stays active after POM disappears.
    //-----------------------------------------------------

    let visual_height =
        relief_visual_height(
            relief_p,
            camera_distance
        );

    let visual_cavity =
        mix(
            CRACK_DARK,
            1.0,
            visual_height
        );

    // Do not stamp terrain-style cracks onto vertical
    // cliff faces.
    let pattern_cavity =
        mix(
            1.0,
            visual_cavity,
            slope_w
        );

    var ground =
        base_ground
        * pattern_cavity;

    var n =
        soft_n;

    var grass_roughness =
        1.0;


    //-----------------------------------------------------
    // Grass coverage
    //
    // Moisture noise stays local variation. Authored
    // coverage scales it so barren ground stays barren.
    // Cracks remain on dirt; grass covers them later.
    //-----------------------------------------------------

    let authored_coverage =
        grass_authored_coverage();

    let grass_slope =
        smoothstep(
            0.40,
            0.70,
            macro_n.y
        );

    var regional_lushness = 0.0;
    var local_threshold = 0.0;
    var local_meadow_noise = 0.0;
    var surface_meadow = 0.0;
    var grass_patch_w = 0.0;

    if (authored_coverage > 1e-4) {
        grass_prepare_eval();

        regional_lushness =
            grass_region(relief_p);

        local_threshold =
            grass_threshold(regional_lushness);

        local_meadow_noise =
            grass_patch_noise(relief_p);

        surface_meadow =
            smoothstep(
                local_threshold,
                local_threshold
                    + GRASS_PATCH_HIGH
                    - GRASS_PATCH_LOW,
                local_meadow_noise
            )
            * authored_coverage;

        grass_patch_w =
            surface_meadow * grass_slope;
    }

    let grass_cover =
        grass_patch_w * GRASS_FAR_COVERAGE;

    let uncovered =
        1.0 - grass_cover;


    //-----------------------------------------------------
    // Near-field crack POM
    //
    // Skip the march only when remaining uncovered
    // contribution is negligible. Cheap far cracks stay
    // on the dirt underneath.
    //-----------------------------------------------------

    if (pom_w > 1e-4 && uncovered > GRASS_CRACK_SKIP) {
        let hit =
            trace_relief(
                relief_p,
                macro_n,
                V
            );

        let q =
            relief_p
            + hit.offset.xz;


        //-------------------------------------------------
        // POM normal
        //-------------------------------------------------

        let gradient =
            relief_gradient(q);

        // Exact local implicit-surface gradient:
        //
        // macro_n - D*(hx,0,hz)
        //
        // soft_n is substituted only for final
        // artistic lighting, never for tracing.
        let shading_gradient =
            soft_n
            -
            CRACK_DEPTH_M
            * vec3<f32>(
                gradient.x,
                0.0,
                gradient.y
            );

        let relief_n =
            normalize(
                shading_gradient
            );

        n =
            normalize(
                mix(
                    soft_n,
                    relief_n,
                    pom_w
                )
            );


        //-------------------------------------------------
        // POM displaced color lookup
        //-------------------------------------------------

        let hit_palette =
            ground_color_at(
                q
                + WORLD_COLOR_SHIFT
            );

        let cavity =
            mix(
                CRACK_DARK,
                1.0,
                hit.height
            );

        let hit_ground =
            hit_palette
            * base_color.rgb
            * cavity;

        // Crossfade from the cheap procedural
        // representation into actual parallax relief.
        //
        // The visual crack field therefore survives
        // when POM becomes too expensive or unresolved.
        ground =
            mix(
                ground,
                hit_ground,
                pom_w
            );
    }


    //-----------------------------------------------------
    // Persistent turf underlayer
    //
    // Far and near share the same mid/tip floor so
    // approaching a patch does not suddenly darken.
    // Wind color is a small term; bending is the motion.
    //-----------------------------------------------------

    if (grass_cover > 1e-4) {
        let tint = grass_tint();
        let gust = grass_wind_at(relief_p);
        let gust_resolved =
            1.0 - smoothstep(0.8, 2.0, footprint);
        let wind_term =
            grass_wind_color_amount()
            * gust
            * gust_resolved;
        let far_palette = grass_palette_at(
            regional_lushness,
            tint,
            wind_term
        );

        // Cell speckle keeps the mat readable after POM
        // drops for subpixel blades. Do not fade this with
        // the footprint gate.
        let speckle = grass_cell_speckle(relief_p);
        let far_ground =
            mix(
                far_palette.mid,
                far_palette.tip,
                saturate(
                    GRASS_TURF_TIP_MIX
                    + 0.22 * speckle
                    + wind_term
                )
            )
            * (0.90 + 0.16 * speckle);

        ground = mix(
            ground,
            far_ground,
            grass_cover
        );

        n = normalize(mix(n, soft_n, grass_cover));
        grass_roughness = mix(1.0, 0.90, grass_patch_w);
    }


    //-----------------------------------------------------
    // Near-field grass POM
    //
    // 35–50 m range. Footprint only skips the march;
    // the speckled underlayer stays. Steep views use a
    // single surface sample instead of a ray.
    //-----------------------------------------------------

    if (authored_coverage > 1e-4) {
        let grass_resolution =
            1.0
            - smoothstep(
                GRASS_HALF_WIDTH_M * 0.6,
                GRASS_HALF_WIDTH_M * 1.8,
                footprint
            );

        let grass_distance =
            1.0
            - smoothstep(
                GRASS_POM_START_M,
                GRASS_POM_END_M,
                camera_distance
            );

        let grass_facing = smoothstep(
            0.025,
            0.10,
            dot(macro_n, V)
        );

        let grass_pom_w =
            grass_resolution
            * grass_distance
            * grass_facing
            * grass_slope;

        let grass_reach =
            GRASS_HEIGHT_M
            * length(V.xz)
            / max(dot(macro_n, V), GRASS_RAY_DENOM_MIN);

        // Cubic value-noise gradient <= 1.5*sqrt(2);
        // smoothstep slope <= 1.5.
        let threshold_gradient_bound =
            (GRASS_DRY_THRESHOLD - GRASS_PATCH_LOW)
            * 1.5
            / (GRASS_REGION_LUSH - GRASS_REGION_DRY)
            * 2.122
            / GRASS_REGION_SCALE_M;

        let meadow_upper_bound =
            local_meadow_noise
            - local_threshold
            + grass_reach * (
                2.122 / GRASS_PATCH_SCALE_M
                + threshold_gradient_bound
            );

        if (grass_pom_w > 1e-4 && meadow_upper_bound > 0.0) {
            var hit = GrassHit(vec3<f32>(0.0), 0.0, false);

            if (grass_reach <= GRASS_SURFACE_REACH_M) {
                let height = grass_height(
                    relief_p,
                    surface_meadow
                );

                if (height > 1e-6) {
                    hit = GrassHit(vec3<f32>(0.0), height, true);
                }
            } else {
                hit = trace_grass(
                    relief_p,
                    macro_n,
                    V,
                    surface_meadow
                );
            }

            if (hit.found) {
                let q = relief_p + hit.offset.xz;
                let height_ratio =
                    saturate(hit.height / GRASS_HEIGHT_M);
                let tint = grass_tint();
                let hit_palette = grass_palette_at(
                    regional_lushness,
                    tint,
                    0.0
                );

                let along_blade = mix(
                    hit_palette.turf,
                    hit_palette.tip,
                    smoothstep(
                        GRASS_TURF_HEIGHT_M / GRASS_HEIGHT_M,
                        0.90,
                        height_ratio
                    )
                );

                let root_floor = mix(
                    hit_palette.root,
                    hit_palette.mid,
                    0.55
                );

                let cell_color = grass_cell_speckle(q);

                let detailed_ground =
                    mix(
                        root_floor,
                        along_blade,
                        smoothstep(0.0, 0.18, height_ratio)
                    )
                    * (0.92 + 0.12 * cell_color);

                let gradient = grass_gradient(
                    q,
                    surface_meadow
                );
                let compression =
                    max(dot(macro_n, V), 0.0)
                    / max(dot(macro_n, V), GRASS_RAY_DENOM_MIN);
                let detailed_n = grass_capped_normal(
                    soft_n,
                    gradient,
                    compression
                );

                ground = mix(
                    ground,
                    detailed_ground,
                    grass_pom_w
                );

                n = normalize(mix(
                    n,
                    detailed_n,
                    grass_pom_w
                ));

                grass_roughness = mix(
                    grass_roughness,
                    0.90,
                    grass_pom_w
                );
            }
        }
    }


    //-----------------------------------------------------
    // PBR
    //-----------------------------------------------------

    pbr_input.material.base_color =
        vec4<f32>(
            ground,
            base_color.a
        );

    pbr_input.material.metallic =
        0.0;

    pbr_input.material.perceptual_roughness =
        grass_roughness;

    pbr_input.frag_coord =
        mesh.position;

    pbr_input.world_position =
        mesh.world_position;

    pbr_input.world_normal =
        n;

    pbr_input.N =
        n;

    pbr_input.V =
        V;

    let lit_color =
        fns::apply_pbr_lighting(
            pbr_input
        );


    //-----------------------------------------------------
    // Prepass outlines
    //-----------------------------------------------------

    let edge =
        depth_edge_laplacian(
            mesh.position,
            style_params.y
        );

    let edge_ink =
        saturate(
            edge * 1000.0
        );

    let edge_intensity =
        mix(
            1.0,
            saturate(style_params.z),
            edge_ink
        );


    //-----------------------------------------------------
    // Fog / tone mapping
    //-----------------------------------------------------

    let shaded =
        lit_color.rgb
        * edge_intensity;

    let lit =
        with_distance_fog(
            vec4<f32>(
                shaded,
                1.0
            ),
            mesh.world_position.xyz,
            mesh.position.xy
        );

    let toned =
        tone_mapping(
            lit,
            view.color_grading
        );

    let unlit =
        with_distance_fog(
            vec4<f32>(
                ground
                * edge_intensity,
                1.0
            ),
            mesh.world_position.xyz,
            mesh.position.xy
        );

    let lit_mix =
        saturate(style_params.w);

    // Unlit albedo is a daylight stamp.
    // Scale it with ambient so night terrain follows
    // the key instead of remaining readable at range.
    let day =
        saturate(
            (
                dot(
                    lights.ambient_color.rgb,
                    vec3<f32>(
                        0.2126,
                        0.7152,
                        0.0722
                    )
                )
                - 15.0
            )
            / 500.0
        );

    let final_color =
        mix(
            unlit.rgb
            * mix(
                0.16,
                1.0,
                day
            ),
            toned.rgb,
            lit_mix
        );

    return vec4<f32>(
        final_color,
        1.0
    );
}