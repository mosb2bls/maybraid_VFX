//! Shared façade / wall-run bay packing (doors and windows).
//!
//! Typologies sample a catalog of [`BaySpec`]s, then pack them along free wall
//! runs with [`fit_bays_on_run`] / [`fit_windows_on_run`].

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

/// One bay size to try when packing along a wall run (doors or windows).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaySpec {
	/// Preferred clear leaf / aperture width (meters).
	pub door_width: f32,
	/// Minimum jamb / reveal on each side of the leaf.
	pub jamb_min: f32,
	/// Allowed over/undershoot on the packed span (and leaf) in meters.
	pub allowed_error: f32,
}

/// A bay placed on a straight wall run (coordinates along the run).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedBay {
	/// Distance from the run start to the leaf’s start edge.
	pub along: f32,
	/// Leaf width used.
	pub width: f32,
	/// Remaining truncate budget for end-of-run / corner fit (from catalog).
	pub allowed_error: f32,
}

/// Pack catalog bays along a run of `run_length` meters.
///
/// Walks `bays` in order; each size is placed if the remaining run can host
/// `door_width + 2·jamb_min` within `allowed_error`, otherwise that size is
/// skipped. When `force_one`, retries with the smallest feasible size.
pub fn fit_bays_on_run(bays: &[BaySpec], run_length: f32, force_one: bool) -> Vec<PlacedBay> {
	let run_length = run_length.max(0.0);
	let mut placed = pack_bays(bays, run_length);
	if placed.is_empty() && force_one {
		placed = force_one_bay(bays, run_length);
	}
	placed
}

/// Pack exterior windows along a run using a density-scaled prefix of `windows`.
///
/// Does not force a window when nothing fits (sparse facades are allowed).
pub fn fit_windows_on_run(
	windows: &[BaySpec],
	opening_density: f32,
	run_length: f32,
) -> Vec<PlacedBay> {
	if windows.is_empty() || opening_density < 0.08 {
		return Vec::new();
	}
	let n = windows.len();
	let take = ((n as f32) * opening_density.clamp(0.15, 1.0)).ceil().max(1.0) as usize;
	let take = take.min(n);
	fit_bays_on_run(&windows[..take], run_length, false)
}

/// Noise-perturbed stall-door catalog (larger shop openings preferred).
pub fn generate_stall_doors(cfg: &NoiseConfig, center: Vec3) -> Vec<BaySpec> {
	generate_bay_catalog(
		cfg,
		center,
		5.0,
		&[
			(5.25, 0.3, 0.4),
			(4.5, 0.28, 0.35),
			(4.0, 0.25, 0.3),
			(3.5, 0.25, 0.3),
			(3.0, 0.22, 0.25),
			(2.5, 0.2, 0.25),
			(2.125, 0.18, 0.2),
			(1.75, 0.15, 0.2),
		],
	)
}

/// Noise-perturbed exterior aperture catalog.
pub fn generate_windows(cfg: &NoiseConfig, center: Vec3) -> Vec<BaySpec> {
	generate_bay_catalog(
		cfg,
		center,
		6.0,
		&[
			(3.2, 0.35, 0.35),
			(2.6, 0.3, 0.3),
			(2.2, 0.28, 0.25),
			(1.8, 0.25, 0.25),
			(1.5, 0.22, 0.2),
			(1.2, 0.2, 0.2),
			(1.0, 0.18, 0.15),
			(0.9, 0.15, 0.15),
		],
	)
}

/// Build a bay catalog from `(width, jamb, allowed_error)` bases with noise jitter.
pub fn generate_bay_catalog(
	cfg: &NoiseConfig,
	center: Vec3,
	salt0: f32,
	bases: &[(f32, f32, f32)],
) -> Vec<BaySpec> {
	bases
		.iter()
		.enumerate()
		.map(|(i, &(w, j, e))| {
			let salt = salt0 + i as f32;
			let dw = cfg.sample_range_f32_4d(
				(w - 0.25).max(0.8),
				w + 0.35,
				center.x,
				center.y,
				center.z,
				salt,
			);
			let jamb = cfg.sample_range_f32_4d(
				(j - 0.05).max(0.1),
				j + 0.1,
				center.x,
				center.y,
				center.z,
				salt + 0.5,
			);
			BaySpec { door_width: dw, jamb_min: jamb, allowed_error: e }
		})
		.collect()
}

fn pack_bays(bays: &[BaySpec], run_length: f32) -> Vec<PlacedBay> {
	let mut cursor = 0.0_f32;
	let mut placed = Vec::new();
	for spec in bays {
		let rem = run_length - cursor;
		let Some((pack, door_w, jamb)) = pack_span(*spec, rem) else {
			continue;
		};
		placed.push(PlacedBay {
			along: cursor + jamb,
			width: door_w,
			allowed_error: spec.allowed_error,
		});
		cursor += pack;
	}
	placed
}

fn force_one_bay(bays: &[BaySpec], run_length: f32) -> Vec<PlacedBay> {
	let mut best: Option<BaySpec> = None;
	for spec in bays {
		let min_pack =
			(spec.door_width - spec.allowed_error).max(0.4) + 2.0 * spec.jamb_min.min(0.05);
		if run_length + 1e-4 < min_pack {
			continue;
		}
		best = Some(match best {
			None => *spec,
			Some(prev) => {
				if spec.door_width < prev.door_width {
					*spec
				} else {
					prev
				}
			}
		});
	}
	let Some(spec) = best else {
		let w = (run_length * 0.5).clamp(0.8, 2.0).min(run_length.max(0.8));
		if run_length < 0.8 {
			return Vec::new();
		}
		let jamb = ((run_length - w) * 0.5).max(0.05);
		return vec![PlacedBay { along: jamb, width: w, allowed_error: 0.25 }];
	};
	if let Some((_pack, door_w, jamb)) = pack_span(spec, run_length) {
		vec![PlacedBay { along: jamb, width: door_w, allowed_error: spec.allowed_error }]
	} else {
		Vec::new()
	}
}

fn pack_span(spec: BaySpec, remaining: f32) -> Option<(f32, f32, f32)> {
	let door_lo = (spec.door_width - spec.allowed_error).max(0.4);
	let door_hi = spec.door_width + spec.allowed_error;
	let jamb = spec.jamb_min.max(0.0);
	let min_pack = door_lo + 2.0 * jamb - spec.allowed_error.max(0.0);
	let min_pack = min_pack.max(door_lo + 0.05);
	if remaining + 1e-4 < min_pack {
		return None;
	}
	let nominal = spec.door_width + 2.0 * jamb;
	let pack = if remaining >= nominal { nominal } else { remaining };
	let door_w = (pack - 2.0 * jamb)
		.clamp(door_lo, door_hi)
		.min(pack - 0.05)
		.max(door_lo.min(pack * 0.8));
	let jamb_each = ((pack - door_w) * 0.5).max(0.0);
	Some((pack, door_w, jamb_each))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fit_bays_places_multiple_on_long_run() {
		let bays = [
			BaySpec { door_width: 3.5, jamb_min: 0.25, allowed_error: 0.35 },
			BaySpec { door_width: 3.0, jamb_min: 0.25, allowed_error: 0.3 },
			BaySpec { door_width: 2.4, jamb_min: 0.2, allowed_error: 0.25 },
		];
		let placed = fit_bays_on_run(&bays, 14.0, true);
		assert!(placed.len() >= 2);
	}

	#[test]
	fn fit_windows_respects_low_density() {
		let windows = [BaySpec { door_width: 2.0, jamb_min: 0.2, allowed_error: 0.2 }];
		assert!(fit_windows_on_run(&windows, 0.05, 20.0).is_empty());
	}
}
