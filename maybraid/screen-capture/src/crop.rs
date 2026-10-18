//! Image-space fit: crop a supersampled render to its alpha bounds.
//!
//! Mesh AABBs are bind-pose for skinned kits, so the 3D fit only gets close.
//! The rendered alpha is the ground truth for centering and fill.

/// Alpha at or below this is treated as background.
const ALPHA_EMPTY: u8 = 8;

/// Tightly packed RGBA rect in source pixels, `[x0, x1) × [y0, y1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AlphaBounds {
	pub x0: u32,
	pub y0: u32,
	pub x1: u32,
	pub y1: u32,
}

pub(crate) fn alpha_bounds(rgba: &[u8], width: u32, height: u32) -> Option<AlphaBounds> {
	let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
	for y in 0..height {
		for x in 0..width {
			let alpha = rgba[((y * width + x) * 4 + 3) as usize];
			if alpha > ALPHA_EMPTY {
				x0 = x0.min(x);
				y0 = y0.min(y);
				x1 = x1.max(x + 1);
				y1 = y1.max(y + 1);
			}
		}
	}
	(x1 > x0 && y1 > y0).then_some(AlphaBounds { x0, y0, x1, y1 })
}

/// Center the alpha bounds in an `out_w × out_h` image, filling `1 - padding`
/// of the limiting axis. Area-averaged with premultiplied alpha.
pub(crate) fn crop_to_alpha(
	rgba: &[u8],
	width: u32,
	height: u32,
	out_w: u32,
	out_h: u32,
	padding: f32,
) -> Vec<u8> {
	let bounds = alpha_bounds(rgba, width, height).unwrap_or(AlphaBounds {
		x0: 0,
		y0: 0,
		x1: width,
		y1: height,
	});
	let fill = (1.0 - padding.clamp(0.0, 0.9)).max(0.05);
	let content_w = (bounds.x1 - bounds.x0) as f32;
	let content_h = (bounds.y1 - bounds.y0) as f32;
	let aspect = out_w.max(1) as f32 / out_h.max(1) as f32;
	let mut crop_w = content_w / fill;
	let mut crop_h = content_h / fill;
	if crop_w / crop_h > aspect {
		crop_h = crop_w / aspect;
	} else {
		crop_w = crop_h * aspect;
	}
	let cx = (bounds.x0 + bounds.x1) as f32 * 0.5;
	let cy = (bounds.y0 + bounds.y1) as f32 * 0.5;
	let left = cx - crop_w * 0.5;
	let top = cy - crop_h * 0.5;
	let step_x = crop_w / out_w.max(1) as f32;
	let step_y = crop_h / out_h.max(1) as f32;
	let taps_x = step_x.ceil().clamp(1.0, 8.0) as u32;
	let taps_y = step_y.ceil().clamp(1.0, 8.0) as u32;
	let taps = (taps_x * taps_y) as f32;

	let mut out = vec![0u8; (out_w * out_h * 4) as usize];
	for oy in 0..out_h {
		for ox in 0..out_w {
			let mut acc = [0.0_f32; 4];
			for ty in 0..taps_y {
				let sy = top + (oy as f32 + (ty as f32 + 0.5) / taps_y as f32) * step_y;
				for tx in 0..taps_x {
					let sx = left + (ox as f32 + (tx as f32 + 0.5) / taps_x as f32) * step_x;
					if sx < 0.0 || sy < 0.0 {
						continue;
					}
					let (px, py) = (sx as u32, sy as u32);
					if px >= width || py >= height {
						continue;
					}
					let i = ((py * width + px) * 4) as usize;
					let alpha = rgba[i + 3] as f32;
					acc[0] += rgba[i] as f32 * alpha;
					acc[1] += rgba[i + 1] as f32 * alpha;
					acc[2] += rgba[i + 2] as f32 * alpha;
					acc[3] += alpha;
				}
			}
			let o = ((oy * out_w + ox) * 4) as usize;
			if acc[3] > 0.0 {
				out[o] = (acc[0] / acc[3]).round().clamp(0.0, 255.0) as u8;
				out[o + 1] = (acc[1] / acc[3]).round().clamp(0.0, 255.0) as u8;
				out[o + 2] = (acc[2] / acc[3]).round().clamp(0.0, 255.0) as u8;
				out[o + 3] = (acc[3] / taps).round().clamp(0.0, 255.0) as u8;
			}
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn canvas(width: u32, height: u32, rect: AlphaBounds) -> Vec<u8> {
		let mut rgba = vec![0u8; (width * height * 4) as usize];
		for y in rect.y0..rect.y1 {
			for x in rect.x0..rect.x1 {
				let i = ((y * width + x) * 4) as usize;
				rgba[i..i + 4].copy_from_slice(&[200, 200, 200, 255]);
			}
		}
		rgba
	}

	#[test]
	fn small_off_center_subject_fills_and_centers() -> anyhow::Result<()> {
		let rect = AlphaBounds { x0: 150, y0: 60, x1: 210, y1: 80 };
		let rgba = canvas(300, 120, rect);
		let out = crop_to_alpha(&rgba, 300, 120, 100, 40, 0.1);
		let bounds = alpha_bounds(&out, 100, 40).ok_or_else(|| anyhow::anyhow!("empty"))?;
		let width = bounds.x1 - bounds.x0;
		assert!(width >= 88, "width {width}");
		let mid = (bounds.x0 + bounds.x1) as i32;
		assert!((mid - 100).abs() <= 2, "mid {mid}");
		let mid_y = (bounds.y0 + bounds.y1) as i32;
		assert!((mid_y - 40).abs() <= 2, "mid_y {mid_y}");
		Ok(())
	}

	#[test]
	fn empty_render_stays_transparent() {
		let rgba = vec![0u8; 16 * 16 * 4];
		let out = crop_to_alpha(&rgba, 16, 16, 8, 8, 0.05);
		assert!(out.chunks_exact(4).all(|pixel| pixel[3] == 0));
	}
}
