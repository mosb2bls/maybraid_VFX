// Presentation upscale shaders for visual styles.
//
// These shaders run at output resolution during the Camera2d sprite blit.
// Keep them inexpensive: texture-sample counts are design constraints.
//
// Material2d loads this file as one Shader asset and picks an @fragment
// entry in specialize. Do not address entries as asset-path #labels.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var source_texture: texture_2d<f32>;
@group(2) @binding(1) var source_sampler: sampler;

@group(2) @binding(2) var<uniform> treatment_params: TreatmentParams;

struct TreatmentParams {
    source_size: vec2<f32>,
    target_size: vec2<f32>,
}

// IMPRESSIONIST: Linear upscale with painterly softening.
// Still 5 samples (center + 4 cardinals). Wider radius + weaker center
// + a cheap hue pull; no extra taps. Linear filtering already averages
// four source texels per tap, so the larger offset is mostly free.
@fragment
fn impressionist_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let texel_size = 1.0 / treatment_params.source_size;

    let center = textureSample(source_texture, source_sampler, uv);
    // ~2.4 source texels ≈ 9 output pixels at 700×394 → 2560×1440.
    let offset = texel_size * 2.4;

    let n1 = textureSample(source_texture, source_sampler, uv + vec2<f32>(-offset.x, 0.0));
    let n2 = textureSample(source_texture, source_sampler, uv + vec2<f32>(offset.x, 0.0));
    let n3 = textureSample(source_texture, source_sampler, uv + vec2<f32>(0.0, -offset.y));
    let n4 = textureSample(source_texture, source_sampler, uv + vec2<f32>(0.0, offset.y));

    let d1 = color_distance(center, n1);
    let d2 = color_distance(center, n2);
    let d3 = color_distance(center, n3);
    let d4 = color_distance(center, n4);

    // Higher threshold lets near-colors wash together; silhouettes still drop out.
    let threshold = 0.28;
    let w1 = smoothstep(threshold, 0.0, d1);
    let w2 = smoothstep(threshold, 0.0, d2);
    let w3 = smoothstep(threshold, 0.0, d3);
    let w4 = smoothstep(threshold, 0.0, d4);

    let center_w = 0.5;
    let total_weight = center_w + w1 + w2 + w3 + w4;
    let washed = (center * center_w + n1 * w1 + n2 * w2 + n3 * w3 + n4 * w4) / total_weight;

    // ALU-only: nudge toward the 5-tap mean so neighboring hues bleed a little.
    let mean5 = (center + n1 + n2 + n3 + n4) * 0.2;
    let luma = dot(washed.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let softened = mix(washed.rgb, vec3<f32>(luma), 0.06);
    let rgb = mix(softened, mean5.rgb, 0.16);
    return vec4<f32>(rgb, washed.a);
}

fn color_distance(a: vec4<f32>, b: vec4<f32>) -> f32 {
    let diff = a.rgb - b.rgb;
    return length(diff);
}

// PRE-POST-MODERN: Misregistered color-print effect.
// Directional displacement in source texels, ~3 texture samples.
// Stable ordered dithering and gentle color banding.
@fragment
fn pre_post_modern_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let texel_size = 1.0 / treatment_params.source_size;
    
    // Directional color layer displacements (in source texels)
    let red_offset = vec2<f32>(0.6, 0.3) * texel_size;
    let green_offset = vec2<f32>(-0.4, 0.5) * texel_size;
    let blue_offset = vec2<f32>(0.2, -0.5) * texel_size;
    
    let r = textureSample(source_texture, source_sampler, uv + red_offset).r;
    let g = textureSample(source_texture, source_sampler, uv + green_offset).g;
    let b = textureSample(source_texture, source_sampler, uv + blue_offset).b;
    
    var color = vec3<f32>(r, g, b);
    
    // Gentle color banding (reduce precision slightly)
    let bands = 48.0;
    color = floor(color * bands + 0.5) / bands;
    
    // Stable ordered dithering
    let dither = ordered_dither(in.position.xy);
    color = color + dither / 255.0;
    
    return vec4<f32>(color, 1.0);
}

// 4x4 Bayer matrix for ordered dithering
fn ordered_dither(screen_pos: vec2<f32>) -> f32 {
    let bayer = array<f32, 16>(
        0.0, 8.0, 2.0, 10.0,
        12.0, 4.0, 14.0, 6.0,
        3.0, 11.0, 1.0, 9.0,
        15.0, 7.0, 13.0, 5.0
    );
    
    let x = i32(screen_pos.x) % 4;
    let y = i32(screen_pos.y) % 4;
    let index = y * 4 + x;
    
    return (bayer[index] / 16.0 - 0.5) * 2.0;
}

// NEAREST-NEIGHBOR: Single nearest sample, no filtering.
// Used for Retro and Antediluvian modes.
@fragment
fn nearest_neighbor_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert UV to source texel coordinates
    let source_coord = in.uv * treatment_params.source_size;
    let texel_coord = floor(source_coord) + 0.5;
    let adjusted_uv = texel_coord / treatment_params.source_size;
    
    return textureSample(source_texture, source_sampler, adjusted_uv);
}
