//! GPU readback of the presented 3D surface (not `screencapture`, not the
//! Metal swapchain).
//!
//! Bevy `Screenshot::primary_window()` reads the window surface. On macOS that
//! texture is often not `COPY_SRC`, so earlier in-engine shots came out black.
//! The offscreen 3D target already has `COPY_SRC`; this path forces that blit
//! apparatus when Native is drawing straight to the window, waits until the
//! follow camera has a handle, then readbacks after render.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::gpu_readback::{GpuReadbackPlugin, Readback, ReadbackComplete};
use screen_capture::{bgra_to_rgba, encode_rgba_png, write_all_atomic};

use crate::render_scale::{target::AppliedRenderScale, RequestForceOffscreen};

const SETTLE_FRAMES: u32 = 3;
const TIMEOUT_FRAMES: u32 = 240;
const BLACK_MEAN: f32 = 0.012;
const BLACK_MAX: f32 = 0.05;

/// Queue a gameplay PNG of the 3D surface after the next settled frames.
#[derive(Component, Clone, Debug)]
pub struct RequestGameplayScreenshot {
	pub path: PathBuf,
	pub command_id: Option<u32>,
}

#[derive(Message, Clone, Debug)]
pub struct GameplayScreenshotFinished {
	pub path: PathBuf,
	pub command_id: Option<u32>,
	pub ok: bool,
	pub detail: String,
}

#[derive(Component)]
struct GameplayReadback {
	path: PathBuf,
	command_id: Option<u32>,
	width: u32,
	height: u32,
}

struct PendingCapture {
	path: PathBuf,
	command_id: Option<u32>,
	frames: u32,
	forced_offscreen: bool,
	readback: Option<Entity>,
}

#[derive(Resource, Default)]
struct GameplayScreenshotJobs {
	pending: Vec<PendingCapture>,
}

pub(crate) fn configure(app: &mut App) {
	if !app.is_plugin_added::<GpuReadbackPlugin>() {
		app.add_plugins(GpuReadbackPlugin::default());
	}
	app.add_message::<GameplayScreenshotFinished>()
		.init_resource::<GameplayScreenshotJobs>()
		.add_systems(Update, (accept_requests, drive_jobs).chain());
}

fn accept_requests(
	mut commands: Commands,
	mut jobs: ResMut<GameplayScreenshotJobs>,
	requests: Query<(Entity, &RequestGameplayScreenshot)>,
) {
	for (entity, request) in &requests {
		info!("gameplay screenshot {}: queued", request.path.display());
		jobs.pending.push(PendingCapture {
			path: request.path.clone(),
			command_id: request.command_id,
			frames: 0,
			forced_offscreen: false,
			readback: None,
		});
		commands.entity(entity).despawn();
	}
}

fn drive_jobs(
	mut commands: Commands,
	mut jobs: ResMut<GameplayScreenshotJobs>,
	applied: Res<AppliedRenderScale>,
	mut finished: MessageWriter<GameplayScreenshotFinished>,
) {
	let mut i = 0;
	while i < jobs.pending.len() {
		if jobs.pending[i].readback.is_some() {
			i += 1;
			continue;
		}
		jobs.pending[i].frames = jobs.pending[i].frames.saturating_add(1);
		if jobs.pending[i].frames > TIMEOUT_FRAMES {
			let job = jobs.pending.remove(i);
			warn!(
				"gameplay screenshot {}: timed out waiting for a readable surface",
				job.path.display()
			);
			finished.write(GameplayScreenshotFinished {
				path: job.path,
				command_id: job.command_id,
				ok: false,
				detail: "timed out waiting for a readable 3D surface".into(),
			});
			continue;
		}
		if applied.capture_source().is_none() {
			if !jobs.pending[i].forced_offscreen {
				commands.spawn(RequestForceOffscreen(true));
				jobs.pending[i].forced_offscreen = true;
				info!(
					"gameplay screenshot {}: forcing offscreen so the 3D target is COPY_SRC",
					jobs.pending[i].path.display()
				);
			}
			i += 1;
			continue;
		}
		if jobs.pending[i].frames < SETTLE_FRAMES {
			i += 1;
			continue;
		}
		let Some((handle, size)) = applied.capture_source() else {
			i += 1;
			continue;
		};
		let entity = commands
			.spawn((
				Readback::texture(handle),
				GameplayReadback {
					path: jobs.pending[i].path.clone(),
					command_id: jobs.pending[i].command_id,
					width: size.x,
					height: size.y,
				},
				Name::new("gameplay_screenshot_readback"),
			))
			.observe(on_readback_complete)
			.id();
		jobs.pending[i].readback = Some(entity);
		i += 1;
	}
}

fn on_readback_complete(
	event: On<ReadbackComplete>,
	mut commands: Commands,
	readbacks: Query<&GameplayReadback>,
	mut jobs: ResMut<GameplayScreenshotJobs>,
	mut finished: MessageWriter<GameplayScreenshotFinished>,
) {
	let completed = event.event();
	let entity = completed.entity;
	let Ok(readback) = readbacks.get(entity) else {
		return;
	};
	jobs.pending.retain(|job| job.readback != Some(entity));
	let result = write_capture(readback, &completed.data);
	match &result {
		Ok(detail) => info!("gameplay screenshot {}: {detail}", readback.path.display()),
		Err(detail) => warn!("gameplay screenshot {}: {detail}", readback.path.display()),
	}
	finished.write(GameplayScreenshotFinished {
		path: readback.path.clone(),
		command_id: readback.command_id,
		ok: result.is_ok(),
		detail: result.unwrap_or_else(|detail| detail),
	});
	commands.entity(entity).despawn();
}

fn write_capture(readback: &GameplayReadback, bgra: &[u8]) -> Result<String, String> {
	let rgba = bgra_to_rgba(bgra, readback.width, readback.height)?;
	let (mean, max) = luma_stats(&rgba);
	if mean <= BLACK_MEAN && max <= BLACK_MAX {
		let png = encode_rgba_png(readback.width, readback.height, &rgba)?;
		let _ = write_all_atomic(&readback.path, &png);
		return Err(format!(
			"capture was black ({}x{} mean_luma={mean:.4} max={max:.4})",
			readback.width, readback.height
		));
	}
	let png = encode_rgba_png(readback.width, readback.height, &rgba)?;
	write_all_atomic(&readback.path, &png)?;
	Ok(format!("wrote {}x{} mean_luma={mean:.3} max={max:.3}", readback.width, readback.height))
}

fn luma_stats(rgba: &[u8]) -> (f32, f32) {
	if rgba.len() < 4 {
		return (0.0, 0.0);
	}
	let mut sum = 0.0f32;
	let mut max = 0.0f32;
	let mut n = 0.0f32;
	for pixel in rgba.chunks_exact(4) {
		let r = pixel[0] as f32 / 255.0;
		let g = pixel[1] as f32 / 255.0;
		let b = pixel[2] as f32 / 255.0;
		let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
		sum += luma;
		max = max.max(luma);
		n += 1.0;
	}
	if n <= 0.0 {
		(0.0, 0.0)
	} else {
		(sum / n, max)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn black_buffer_is_rejected() {
		let rgba = vec![0u8; 16];
		let (mean, max) = luma_stats(&rgba);
		assert!(mean <= BLACK_MEAN);
		assert!(max <= BLACK_MAX);
	}

	#[test]
	fn lit_buffer_is_accepted() {
		let rgba = vec![40, 80, 120, 255, 10, 10, 10, 255, 200, 180, 90, 255, 0, 0, 0, 255];
		let (mean, max) = luma_stats(&rgba);
		assert!(mean > BLACK_MEAN);
		assert!(max > BLACK_MAX);
	}
}
