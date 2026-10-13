//! Procedural value maps: heat/shade in RGB, shape in alpha.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

pub fn paint_atlas(grid: UVec2, cell_px: u32, cell: fn(f32, f32, f32) -> [u8; 4]) -> Image {
	let width = grid.x * cell_px;
	let height = grid.y * cell_px;
	let frames = grid.x * grid.y;
	let mut data = vec![0u8; (width * height * 4) as usize];
	for frame in 0..frames {
		let t = if frames > 1 { frame as f32 / (frames - 1) as f32 } else { 0.0 };
		let col = frame % grid.x;
		let row = frame / grid.x;
		for y in 0..cell_px {
			for x in 0..cell_px {
				let u = (x as f32 + 0.5) / cell_px as f32;
				let v = (y as f32 + 0.5) / cell_px as f32;
				let px = col * cell_px + x;
				let py = row * cell_px + y;
				let i = ((py * width + px) * 4) as usize;
				data[i..i + 4].copy_from_slice(&cell(u, v, t));
			}
		}
	}
	atlas_image(width, height, data)
}

pub fn paint_spark_taper() -> Image {
	const W: u32 = 64;
	const H: u32 = 16;
	let mut data = vec![0u8; (W * H * 4) as usize];
	for y in 0..H {
		for x in 0..W {
			let u = (x as f32 + 0.5) / W as f32;
			let v = (y as f32 + 0.5) / H as f32;
			let along = 1.0 - ((u - 0.5).abs() * 2.0).powf(1.6);
			let across = 1.0 - ((v - 0.5).abs() * 2.2).clamp(0.0, 1.0);
			let alpha = (along * across.powf(1.4)).clamp(0.0, 1.0);
			let i = ((y * W + x) * 4) as usize;
			data[i..i + 4].copy_from_slice(&value(1.0, alpha));
		}
	}
	atlas_image(W, H, data)
}

pub fn fire_cell(u: f32, v: f32, t: f32) -> [u8; 4] {
	let swell = 0.72 + 0.38 * (t * std::f32::consts::PI).sin().max(0.0);
	let mut cover: f32 = 0.0;
	let mut heat: f32 = 0.0;
	for (i, &(ox, oy, w)) in
		[(0.00, 0.02, 1.00), (0.16, -0.06, 0.72), (-0.14, 0.10, 0.68), (0.04, 0.16, 0.55)]
			.iter()
			.enumerate()
	{
		let drift = (t * (1.2 + i as f32 * 0.35) + i as f32).sin() * 0.06;
		let cx = 0.5 + ox * swell + drift;
		let cy = 0.5 + oy * swell - t * 0.04;
		let r = ((u - cx) * (u - cx) + (v - cy) * (v - cy) * 0.92).sqrt();
		let radius = (0.16 + 0.10 * w) * swell * (1.0 - t * 0.22);
		let blob = (1.0 - (r / radius.max(1e-3)).clamp(0.0, 1.0)).powf(1.45);
		cover = cover.max(blob);
		heat = heat.max(blob * (1.05 - r * 1.8));
	}
	let margin = edge_margin(u, v);
	value((heat * margin * (1.0 - t * 0.2)).clamp(0.0, 1.0), cover * margin * (0.95 - t * 0.25))
}

pub fn smoke_cell(u: f32, v: f32, t: f32) -> [u8; 4] {
	let swell = 0.78 + 0.42 * t;
	let mut cover: f32 = 0.0;
	let mut lit: f32 = 0.0;
	for (i, &(ox, oy, w)) in [
		(0.00, 0.04, 1.00),
		(0.14, -0.08, 0.78),
		(-0.16, 0.02, 0.74),
		(0.08, 0.14, 0.62),
		(-0.06, -0.12, 0.58),
	]
	.iter()
	.enumerate()
	{
		let drift = (t * (0.9 + i as f32 * 0.4) + i as f32 * 1.7).sin() * 0.07;
		let cx = 0.5 + ox * swell + drift;
		let cy = 0.52 + oy * swell - t * 0.10;
		let r = ((u - cx) * (u - cx) * 0.88 + (v - cy) * (v - cy) * 1.05).sqrt();
		let radius = (0.18 + 0.11 * w) * swell;
		let blob = (1.0 - (r / radius.max(1e-3)).clamp(0.0, 1.0)).powf(1.7);
		cover = cover.max(blob);
		lit = lit.max(blob * (0.55 + 0.45 * (1.0 - (v - cy).abs() * 2.2).clamp(0.0, 1.0)));
	}
	let margin = edge_margin(u, v);
	value(soft_band(lit * margin, 3.0), cover * margin * (0.62 - t * 0.18).max(0.28))
}

fn atlas_image(width: u32, height: u32, data: Vec<u8>) -> Image {
	let mut image = Image::new(
		Extent3d { width, height, depth_or_array_layers: 1 },
		TextureDimension::D2,
		data,
		TextureFormat::Rgba8UnormSrgb,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.sampler = ImageSampler::linear();
	image
}

fn edge_margin(u: f32, v: f32) -> f32 {
	let mx = (u.min(1.0 - u) * 8.0).clamp(0.0, 1.0);
	let my = (v.min(1.0 - v) * 8.0).clamp(0.0, 1.0);
	mx.min(my)
}

fn soft_band(v: f32, steps: f32) -> f32 {
	let s = steps.max(2.0);
	let x = v.clamp(0.0, 1.0) * s;
	let q = x.floor();
	let f = x.fract();
	let t = ((f - 0.32) / 0.36).clamp(0.0, 1.0);
	let t = t * t * (3.0 - 2.0 * t);
	(q + t) / s
}

fn value(heat: f32, alpha: f32) -> [u8; 4] {
	let v = (heat.clamp(0.0, 1.0) * 255.0) as u8;
	[v, v, v, (alpha.clamp(0.0, 1.0) * 255.0) as u8]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fire_first_frame_is_a_value_map() {
		let px = fire_cell(0.5, 0.5, 0.0);
		assert_eq!(px[0], px[1]);
		assert!(px[0] > 80 && px[3] > 80, "{px:?}");
		assert_eq!(fire_cell(0.02, 0.02, 0.0)[3], 0);
	}

	#[test]
	fn smoke_last_frame_holds_a_puff() {
		let center = smoke_cell(0.5, 0.48, 1.0);
		assert_eq!(center[0], center[2]);
		assert!(center[3] > 40, "{center:?}");
		assert_eq!(smoke_cell(0.02, 0.02, 1.0)[3], 0);
	}

	#[test]
	fn spark_mask_tapers() {
		let image = paint_spark_taper();
		let data = image.data.as_ref().expect("spark bytes");
		let mid = ((8u32 * 64 + 32) * 4) as usize;
		let tip = ((8u32 * 64 + 2) * 4) as usize;
		assert!(data[mid + 3] > 160);
		assert!(data[tip + 3] < 40);
	}
}
