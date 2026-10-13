//! Residential room-count program and per-rect kind distribution.

use bevy_math::bounding::Aabb2d;
use bevy_math::Vec3;
use procedural_common::{aabb2_area, NoiseConfig, NoiseParams};

use crate::usage_areas::rectangular_livable_area::RectQuarterKind;

const EPS: f32 = 1e-3;

/// Counts of program rooms for an apartment footprint.
#[derive(Debug, Clone, Copy)]
pub struct ProgramCounts {
	pub bedrooms: u8,
	pub bathrooms: u8,
	pub half_baths: u8,
	pub kitchens: u8,
	pub dining: u8,
	pub living: u8,
	pub sitting: u8,
	pub studies: u8,
}

/// Area-tier program: kitchen almost always; living always; dining from mid size.
pub fn program_from_area(area: f32, noise: NoiseParams, center: Vec3) -> ProgramCounts {
	let cfg = NoiseConfig::new(noise);
	let jitter = cfg.sample_range_f32_4d(0.0, 1.0, center.x, center.y, center.z, 44.0);
	let want_kitchen = area >= 22.0 || jitter > 0.2;
	if area < 36.0 {
		ProgramCounts {
			bedrooms: 0,
			bathrooms: 1,
			half_baths: 0,
			kitchens: if want_kitchen { 1 } else { 0 },
			dining: 0,
			living: 1,
			sitting: 0,
			studies: 0,
		}
	} else if area < 58.0 {
		ProgramCounts {
			bedrooms: 1,
			bathrooms: 1,
			half_baths: 0,
			kitchens: 1,
			dining: if jitter > 0.35 { 1 } else { 0 },
			living: 1,
			sitting: 0,
			studies: 0,
		}
	} else if area < 95.0 {
		ProgramCounts {
			bedrooms: 2,
			bathrooms: 1,
			half_baths: if jitter > 0.55 { 1 } else { 0 },
			kitchens: 1,
			dining: 1,
			living: 1,
			sitting: if jitter > 0.7 { 1 } else { 0 },
			studies: 0,
		}
	} else {
		ProgramCounts {
			bedrooms: if area > 120.0 { 3 } else { 2 },
			bathrooms: if area > 110.0 { 2 } else { 1 },
			half_baths: if jitter > 0.45 { 1 } else { 0 },
			kitchens: 1,
			dining: 1,
			living: 1,
			sitting: if jitter > 0.5 { 1 } else { 0 },
			studies: if jitter > 0.55 { 1 } else { 0 },
		}
	}
}

/// Flatten counts to a kind list (Eating before Living).
pub fn full_kind_list(p: ProgramCounts) -> Vec<RectQuarterKind> {
	let mut out = Vec::new();
	if p.kitchens > 0 || p.dining > 0 {
		out.push(RectQuarterKind::Eating);
	}
	for _ in 0..p.living {
		out.push(RectQuarterKind::Living);
	}
	for _ in 0..p.sitting {
		out.push(RectQuarterKind::Sitting);
	}
	for _ in 0..p.bedrooms {
		out.push(RectQuarterKind::Bedroom);
	}
	for _ in 0..p.bathrooms {
		out.push(RectQuarterKind::Bathroom);
	}
	for _ in 0..p.half_baths {
		out.push(RectQuarterKind::HalfBath);
	}
	for _ in 0..p.studies {
		out.push(RectQuarterKind::Study);
	}
	if out.is_empty() {
		out.push(RectQuarterKind::Living);
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::usage_areas::plan_geom::noise_for_cell;

	#[test]
	fn suite_seed_diversifies_program_jitter() {
		// Same footprint/center — only seed salt differs (as when I-apartment
		// reused parent noise for every suite).
		let center = Vec3::new(4.0, 1.75, 12.0);
		let area = 72.0;
		let parent = NoiseParams { seed: 1337, ..NoiseParams::default() };
		let a = program_from_area(area, noise_for_cell(parent, 0), center);
		let b = program_from_area(area, noise_for_cell(parent, 1), center);
		let c = program_from_area(area, noise_for_cell(parent, 2), center);
		// At least one optional slot should flip across neighboring suite salts.
		let key = |p: ProgramCounts| (p.half_baths, p.sitting, p.dining, p.studies);
		assert!(
			key(a) != key(b) || key(b) != key(c) || key(a) != key(c),
			"expected suite salts to diversify program options: {a:?} {b:?} {c:?}"
		);
	}

	#[test]
	fn eating_prefers_larger_when_alone() {
		// Former prefer-smallest penalty parked Eating on the stub; alone it
		// should land on the larger max-rect.
		use bevy_math::Vec2;
		let rects = [
			Aabb2d { min: Vec2::ZERO, max: Vec2::new(2.0, 3.0) },
			Aabb2d { min: Vec2::ZERO, max: Vec2::new(8.0, 6.0) },
		];
		let slices = distribute_program(&[RectQuarterKind::Eating], &rects, NoiseParams::default());
		assert!(
			slices[1].contains(&RectQuarterKind::Eating),
			"Eating alone should prefer the large rect: {slices:?}"
		);
	}
}

/// Assign kinds across max-rects. Living prefers large rects; within a rect,
/// Eating packs before Living so the kitchen carves compactly.
///
/// Eating is allowed on smaller rects (no size bonus/penalty) — it must not be
/// steered onto the smallest pocket. `noise` jitter breaks deterministic ties
/// across similar-area max-rects.
pub fn distribute_program(
	kinds: &[RectQuarterKind],
	rects: &[Aabb2d],
	noise: NoiseParams,
) -> Vec<Vec<RectQuarterKind>> {
	let n = rects.len();
	let mut slices = vec![Vec::new(); n];
	if n == 0 {
		return slices;
	}
	let cfg = NoiseConfig::new(noise);
	let areas: Vec<f32> = rects.iter().map(|r| aabb2_area(*r)).collect();
	let total: f32 = areas.iter().sum::<f32>().max(EPS);
	// Seeded visit order among equal priority tiers so assignment isn't index-0 biased.
	let mut ordered: Vec<RectQuarterKind> = kinds.to_vec();
	ordered.sort_by(|a, b| {
		let ka = kind_assign_tier(*a);
		let kb = kind_assign_tier(*b);
		ka.cmp(&kb).then_with(|| {
			let ja = cfg.sample_unit_4d(ka as f32, kind_noise_key(*a), 0.0, 71.0);
			let jb = cfg.sample_unit_4d(kb as f32, kind_noise_key(*b), 0.0, 71.0);
			ja.partial_cmp(&jb).unwrap_or(std::cmp::Ordering::Equal)
		})
	});
	let mut load = vec![0.0_f32; n];
	let targets: Vec<f32> = areas.iter().map(|a| a / total).collect();
	// Prefer larger max-rects first when scoring ties; jitter breaks near-ties.
	let mut rect_order: Vec<usize> = (0..n).collect();
	rect_order.sort_by(|&a, &b| {
		let sa = areas[a] + cfg.sample_unit_4d(a as f32, areas[a], 0.0, 73.0) * areas[a] * 0.1;
		let sb = areas[b] + cfg.sample_unit_4d(b as f32, areas[b], 0.0, 73.0) * areas[b] * 0.1;
		sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
	});
	for (ki, kind) in ordered.into_iter().enumerate() {
		let mut best = rect_order[0];
		let mut best_score = f32::NEG_INFINITY;
		for &i in &rect_order {
			let closed_bonus = if kind.is_closed() { areas[i] * 0.02 } else { 0.0 };
			let living_bonus = if matches!(kind, RectQuarterKind::Living | RectQuarterKind::Sitting)
			{
				areas[i] * 0.04
			} else {
				0.0
			};
			let jitter =
				cfg.sample_unit_4d(i as f32, areas[i], ki as f32, kind_noise_key(kind)) * 0.025;
			let score = targets[i] - load[i] / total.max(1.0)
				+ areas[i] * 1e-4
				+ closed_bonus
				+ living_bonus
				+ jitter;
			if score > best_score {
				best_score = score;
				best = i;
			}
		}
		slices[best].push(kind);
		load[best] += if kind.is_closed() {
			1.5
		} else if matches!(kind, RectQuarterKind::Living | RectQuarterKind::Sitting) {
			1.35
		} else {
			1.0
		};
	}
	for s in &mut slices {
		if s.is_empty() {
			s.push(RectQuarterKind::Living);
		}
		s.sort_by_key(|k| match k {
			k if k.is_closed() => 0u8,
			RectQuarterKind::Eating | RectQuarterKind::Kitchen => 1u8,
			RectQuarterKind::Living | RectQuarterKind::Sitting => 2u8,
			_ => 3u8,
		});
	}
	slices
}

fn kind_assign_tier(k: RectQuarterKind) -> u8 {
	if k.is_closed() {
		0
	} else {
		match k {
			RectQuarterKind::Living | RectQuarterKind::Sitting => 1,
			RectQuarterKind::Eating => 2,
			_ => 3,
		}
	}
}

fn kind_noise_key(k: RectQuarterKind) -> f32 {
	match k {
		RectQuarterKind::Bedroom => 1.0,
		RectQuarterKind::Living => 2.0,
		RectQuarterKind::Eating => 3.0,
		RectQuarterKind::Kitchen => 4.0,
		RectQuarterKind::Dining => 5.0,
		RectQuarterKind::Bathroom => 6.0,
		RectQuarterKind::HalfBath => 7.0,
		RectQuarterKind::Sitting => 8.0,
		RectQuarterKind::Study => 9.0,
	}
}
