//---------------------------------------------------------
// Chico bump outs: terrain-topology displacement, profile
// interpolation, and opaque stochastic fragment dropout.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::Vertex,
    mesh_functions,
    mesh_view_bindings::{view, lights},
    pbr_functions as fns,
    view_transformations::position_world_to_clip,
}
#import bevy_core_pipeline::tonemapping::tone_mapping
#ifdef DISTANCE_FOG
#import bevy_pbr::mesh_view_bindings::fog
#endif

fn with_distance_fog(color: vec4<f32>, world_position: vec3<f32>, frag_xy: vec2<f32>) -> vec4<f32> {
#ifdef DISTANCE_FOG
    return fns::apply_fog(fog, color, world_position, view.world_position.xyz, frag_xy);
#else
    return color;
#endif
}

struct BumpOutUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
}

const RASTER_DENSITY: u32 = 0u;
const RASTER_BITE_SIZE: u32 = 1u;
const RASTER_BITE_SIZE_DEVIATION: u32 = 2u;
const RASTER_AVERAGE_HEIGHT: u32 = 3u;
const RASTER_HEIGHT_DEVIATION: u32 = 4u;

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> bump: BumpOutUniform;

struct BumpOutVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
    @location(8) density: f32,
    @location(9) bite_size: f32,
    @location(10) bite_size_deviation: f32,
}

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn hash21(p: vec2<f32>) -> f32 {
    let q = vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973);
    let f = fract(q);
    let d = dot(f, f.yzx + vec3<f32>(33.33));
    return fract((f.x + f.y) * f.z + d);
}

fn value_noise_2d(p: vec2<f32>, seed: f32) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (vec2<f32>(3.0) - 2.0 * f0);
    let s = vec2<f32>(seed * 17.13, seed * 31.71);
    let a = hash21(i + vec2<f32>(0.0, 0.0) + s);
    let b = hash21(i + vec2<f32>(1.0, 0.0) + s);
    let c = hash21(i + vec2<f32>(0.0, 1.0) + s);
    let d = hash21(i + vec2<f32>(1.0, 1.0) + s);
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn fbm_2d_3(p: vec2<f32>, seed: f32) -> f32 {
    let a = value_noise_2d(p, seed) * 0.5;
    let b = value_noise_2d(p * 2.03, seed + 19.17) * 0.25;
    let c = value_noise_2d(p * 4.11, seed + 47.31) * 0.125;
    return (a + b + c) / 0.875;
}

fn sample_row(row: vec4<f32>, x: f32) -> f32 {
    if (x < 1.0) {
        return mix(row.x, row.y, x);
    }
    return mix(row.y, row.z, x - 1.0);
}

fn sample_grid(rows: array<vec4<f32>, 3>, uv: vec2<f32>) -> f32 {
    // UV 0/1 lies halfway between this cell's center sample and its neighbor's center sample.
    // Adjacent cells carrying reciprocal neighborhoods therefore agree on their shared edge.
    let p = clamp(uv + vec2<f32>(0.5), vec2<f32>(0.0), vec2<f32>(2.0));
    if (p.y < 1.0) {
        return mix(sample_row(rows[0], p.x), sample_row(rows[1], p.x), p.y);
    }
    return mix(sample_row(rows[1], p.x), sample_row(rows[2], p.x), p.y - 1.0);
}

@vertex
fn vertex(vertex: Vertex) -> BumpOutVertexOutput {
    var out: BumpOutVertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var profile_uv = vec2<f32>(0.5);

#ifdef VERTEX_UVS_A
    profile_uv = vertex.uv;
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.density = saturate(sample_grid(bump.rasters[RASTER_DENSITY], profile_uv));
    out.bite_size = max(sample_grid(bump.rasters[RASTER_BITE_SIZE], profile_uv), 0.01);
    out.bite_size_deviation = max(
        sample_grid(bump.rasters[RASTER_BITE_SIZE_DEVIATION], profile_uv),
        0.0,
    );
    let average_height = sample_grid(bump.rasters[RASTER_AVERAGE_HEIGHT], profile_uv);
    let height_deviation = max(
        sample_grid(bump.rasters[RASTER_HEIGHT_DEVIATION], profile_uv),
        0.0,
    );
    let displacement_noise = fbm_2d_3(
        out.world_position.xz * bump.noise.x,
        bump.noise.z,
    );
    out.world_position.y += average_height
        + (displacement_noise - 0.5) * 2.0 * height_deviation;
    out.position = position_world_to_clip(out.world_position.xyz);

#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index,
    );
#else
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex.instance_index,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex.instance_index,
        world_from_local[3],
    );
#endif
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: BumpOutVertexOutput,
) -> @location(0) vec4<f32> {
    let world = mesh.world_position.xyz;
    if (mesh.density <= 0.001) {
        discard;
    }

    // Foliage-style swiss cheese: broad FBM bites mixed with a smaller breakup field.
    let bite_scale_noise = value_noise_2d(
        world.xz * bump.noise.x * 0.23,
        bump.noise.z + 73.1,
    );
    let bite_size = mesh.bite_size * exp2(
        (bite_scale_noise - 0.5) * 2.0 * mesh.bite_size_deviation,
    );
    let cheese_position = world.xz / max(bite_size, 0.01) * bump.scalars[1].x;
    let broad_bites = fbm_2d_3(cheese_position, bump.noise.z + 91.7);
    let fine_bites = value_noise_2d(cheese_position * 3.7, bump.noise.z + 151.3);
    let bite_field = mix(
        broad_bites,
        broad_bites * 0.62 + fine_bites * 0.38,
        bump.scalars[0].w,
    );
    let bite_threshold = mix(0.88, 0.12, mesh.density)
        + bump.scalars[0].w * mix(0.08, 0.04, mesh.density);
    let softness = bump.scalars[0].x;
    let bite_width = max(
        fwidth(bite_field) * (1.0 + softness * 8.0),
        max(softness * 0.2, 0.005),
    );
    let bite_alpha = smoothstep(
        bite_threshold - bite_width,
        bite_threshold + bite_width,
        bite_field,
    );
    if (bite_alpha < 0.08) {
        discard;
    }

    let tint_noise = value_noise_2d(world.xz * bump.noise.x * 0.31, bump.noise.z + 37.0);
    let color01 = mix(bump.colors[0].rgb, bump.colors[1].rgb, saturate(tint_noise * 2.0));
    let color12 = mix(bump.colors[1].rgb, bump.colors[2].rgb, saturate(tint_noise * 2.0 - 1.0));
    var albedo = mix(color01, color12, step(0.5, tint_noise));
    let warm_cool = mix(
        vec3<f32>(0.94, 1.02, 0.96),
        vec3<f32>(1.08, 1.04, 0.94),
        tint_noise,
    );
    let brightness = mix(0.94, 1.08, tint_noise);
    let wash = mix(0.90, 1.0, saturate(bite_alpha * 1.25));
    albedo *= warm_cool * brightness * wash;

    // Static fragment-scale apparent height adds sub-triangle relief without extra vertices.
    let detail_height = (
        fbm_2d_3(
            world.xz * bump.noise.x * bump.scalars[1].y,
            bump.noise.z + 211.9,
        ) - 0.5
    ) * 2.0 * bump.scalars[1].z * mesh.density;
    let apparent_position = world + vec3<f32>(0.0, detail_height, 0.0);
    var geometric_normal = normalize(cross(dpdx(apparent_position), dpdy(apparent_position)));
    if (dot(geometric_normal, mesh.world_normal) < 0.0) {
        geometric_normal = -geometric_normal;
    }
    let prepared = fns::prepare_world_normal(geometric_normal, true, is_front);
    let normal = normalize(mix(prepared, vec3<f32>(0.0, 1.0, 0.0), bump.scalars[0].z));

    var ndl = 0.55;
    if (lights.n_directional_lights > 0u) {
        let direction_to_light = lights.directional_lights[0].direction_to_light;
        ndl = saturate(dot(normal, direction_to_light));
    }
    let day = saturate((dot(lights.ambient_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) - 20.0) / 600.0);
    let sun = ndl * mix(0.22, 0.95, day) + mix(0.02, 0.14, day);
    let sky = mix(0.06, 0.38, day) + mix(0.04, 0.17, day) * saturate(normal.y);
    let sky_rgb = mix(vec3<f32>(0.16, 0.08, 0.30), vec3<f32>(0.78, 0.88, 1.0), day);
    let lifted = albedo * sun + albedo * sky * sky_rgb;
    let fogged = with_distance_fog(
        vec4<f32>(lifted, 1.0),
        world,
        mesh.position.xy,
    );
    return tone_mapping(fogged, view.color_grading);
}
