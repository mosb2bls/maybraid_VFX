//! Write example shaded + silhouette PNGs through the disk cache (no GPU).

use screen_capture::{
	CaptureCache, CaptureCacheKey, CaptureLighting, CaptureMode, CaptureRequest, CaptureResolution,
	CaptureRevision, MeshCaptureSubject,
};
use std::path::PathBuf;

fn main() -> Result<(), String> {
	let out = artifact_dir();
	std::fs::create_dir_all(&out).map_err(|error| error.to_string())?;
	let cache = CaptureCache::new(out.join("cache"));
	let subject = MeshCaptureSubject::cuboid([0.82, 0.18, 0.16, 1.0]);
	for (mode, pixels) in [
		(CaptureMode::Shaded, shaded_cube_rgba(128)),
		(CaptureMode::Silhouette, silhouette_cube_rgba(128)),
	] {
		let request = CaptureRequest::from_subject(
			&subject,
			mode,
			CaptureResolution::new(128, 128),
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		let key = CaptureCacheKey::from_request(&request);
		cache.save_rgba_blocking(&key, 128, 128, &pixels)?;
		let dest = out.join(format!("cube_{}.png", mode.as_str()));
		std::fs::copy(cache.path_for(&key), &dest).map_err(|error| error.to_string())?;
		println!("wrote {}", dest.display());
	}
	Ok(())
}

fn artifact_dir() -> PathBuf {
	if let Ok(path) = std::env::var("SCREEN_CAPTURE_ARTIFACTS") {
		return PathBuf::from(path);
	}
	if PathBuf::from("/opt/cursor/artifacts").is_dir() {
		return PathBuf::from("/opt/cursor/artifacts/screen-capture");
	}
	PathBuf::from("target/screen-capture-artifacts")
}

fn shaded_cube_rgba(size: u32) -> Vec<u8> {
	draw_cube(size, [210, 46, 41, 255], [120, 24, 22, 255], [255, 130, 110, 255], false)
}

fn silhouette_cube_rgba(size: u32) -> Vec<u8> {
	draw_cube(size, [184, 184, 184, 255], [184, 184, 184, 255], [184, 184, 184, 255], true)
}

fn draw_cube(
	size: u32,
	front: [u8; 4],
	side: [u8; 4],
	top: [u8; 4],
	transparent_bg: bool,
) -> Vec<u8> {
	let mut rgba = vec![0_u8; (size as usize) * (size as usize) * 4];
	if !transparent_bg {
		for pixel in rgba.chunks_exact_mut(4) {
			pixel.copy_from_slice(&[32, 34, 40, 255]);
		}
	}
	let s = size as i32;
	let fill = |rgba: &mut [u8], x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4]| {
		for y in y0.max(0)..y1.min(s) {
			for x in x0.max(0)..x1.min(s) {
				let i = ((y * s + x) as usize) * 4;
				rgba[i..i + 4].copy_from_slice(&color);
			}
		}
	};
	// Simple 3/4 cube: front, right, top.
	fill(&mut rgba, s / 4, s / 3, s / 4 + s / 3, s / 3 + s / 3, front);
	fill(&mut rgba, s / 4 + s / 3, s / 3 - s / 10, s / 4 + s / 3 + s / 6, s / 3 + s / 3, side);
	fill(&mut rgba, s / 4 + s / 10, s / 3 - s / 8, s / 4 + s / 3 + s / 8, s / 3, top);
	rgba
}
