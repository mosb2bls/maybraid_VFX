//! Layer 1: 15° wall sectors, solid merges, and cut-sector slice strips.

use bevy_math::Vec3;
use building_components::partitions::{Partition, PartitionNode, PartitionStyle, SLICE_KIT_HEIGHT};
use building_components::Placement;

use super::ring::{aabb3d_intersects, EPS, SECTORS, SEG_DEG};
use super::ArcFloorParams;

/// Drop footer/header strips thinner than this fraction of storey height.
const MIN_STRIP_HEIGHT_FRAC: f32 = 0.05;

/// Per-sector open \(Y\) intervals (storey space) after Layer 1.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct SectorCuts {
	/// Merged half-open ranges \([y_0, y_1)\) cut out of the wall.
	opens: Vec<(f32, f32)>,
}

impl SectorCuts {
	pub(super) fn is_solid(&self) -> bool {
		self.opens.is_empty()
	}

	fn add_open(&mut self, y0: f32, y1: f32) {
		if y1 - y0 <= EPS {
			return;
		}
		self.opens.push((y0, y1));
		merge_y_ranges(&mut self.opens);
	}
}

impl ArcFloorParams {
	/// Resolve Layer 1 wall partitions from the opening plan.
	pub(super) fn resolve_wall_sweeps(
		&self,
	) -> ([SectorCuts; SECTORS as usize], Vec<PartitionNode>) {
		let y0 = self.center_xz.y;
		let y1 = y0 + self.storey_height;
		let mut sectors = std::array::from_fn(|_| SectorCuts::default());

		for (_id, opening) in self.openings.iter() {
			let open_lo = opening.bounds.min.y.max(y0);
			let open_hi = opening.bounds.max.y.min(y1);
			if open_hi - open_lo <= EPS {
				continue;
			}
			for i in 0..SECTORS {
				let sector = self.sector_aabb(i);
				if !aabb3d_intersects(&opening.bounds, &sector) {
					continue;
				}
				sectors[i as usize].add_open(open_lo, open_hi);
			}
		}

		let partitions = emit_wall_partitions(self, &sectors);
		(sectors, partitions)
	}
}

fn emit_wall_partitions(
	params: &ArcFloorParams,
	sectors: &[SectorCuts; SECTORS as usize],
) -> Vec<PartitionNode> {
	let mut partitions = Vec::new();
	let y0 = params.center_xz.y;
	let y1 = y0 + params.storey_height;
	let min_h = MIN_STRIP_HEIGHT_FRAC * params.storey_height;

	let mut i = 0u32;
	while i < SECTORS {
		if sectors[i as usize].is_solid() {
			let mut run = 1u32;
			while i + run < SECTORS && sectors[(i + run) as usize].is_solid() {
				run += 1;
			}
			emit_solid_run(
				&mut partitions,
				params.center_xz,
				params.radius,
				params.storey_height,
				i,
				run,
				params.style,
			);
			i += run;
		} else {
			emit_cut_sector(&mut partitions, params, i, &sectors[i as usize], y0, y1, min_h);
			i += 1;
		}
	}
	partitions
}

fn emit_cut_sector(
	partitions: &mut Vec<PartitionNode>,
	params: &ArcFloorParams,
	sector: u32,
	cuts: &SectorCuts,
	y0: f32,
	y1: f32,
	min_h: f32,
) {
	let yaw_deg = sector as f32 * SEG_DEG;
	for (band_lo, band_hi) in solid_y_bands(y0, y1, &cuts.opens) {
		let h = band_hi - band_lo;
		if h < min_h {
			continue;
		}
		// Slice kits span SLICE_KIT_HEIGHT in unit Y; scale so world height = h.
		let y_scale = h / SLICE_KIT_HEIGHT;
		let origin = Vec3::new(params.center_xz.x, band_lo, params.center_xz.z);
		push_slice(
			partitions,
			origin,
			Vec3::new(params.radius, y_scale, params.radius),
			yaw_deg,
			SEG_DEG,
			params.style,
		);
	}
}

fn solid_y_bands(y0: f32, y1: f32, opens: &[(f32, f32)]) -> Vec<(f32, f32)> {
	let mut bands = Vec::new();
	let mut cursor = y0;
	for &(o0, o1) in opens {
		if o0 > cursor + EPS {
			bands.push((cursor, o0.min(y1)));
		}
		cursor = cursor.max(o1);
	}
	if y1 > cursor + EPS {
		bands.push((cursor, y1));
	}
	bands
}

fn merge_y_ranges(ranges: &mut Vec<(f32, f32)>) {
	if ranges.is_empty() {
		return;
	}
	ranges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
	let mut merged = Vec::with_capacity(ranges.len());
	let mut cur = ranges[0];
	for &(a, b) in ranges.iter().skip(1) {
		if a <= cur.1 + EPS {
			cur.1 = cur.1.max(b);
		} else {
			merged.push(cur);
			cur = (a, b);
		}
	}
	merged.push(cur);
	*ranges = merged;
}

fn emit_solid_run(
	partitions: &mut Vec<PartitionNode>,
	center_xz: Vec3,
	radius: f32,
	storey_height: f32,
	start_sector: u32,
	run: u32,
	style: PartitionStyle,
) {
	let ring_scale = Vec3::new(radius, storey_height, radius);
	let mut remaining = run;
	let mut at = start_sector;
	// Prefer 180°, then 90°, then 15° leftovers.
	// Decompose offsets increase from the parent yaw, so a run of sectors
	// `at .. at+chunk` is covered by a kit whose yaw is the *last* sector's yaw
	// when each piece occupies the authored +X→+Z wedge.
	while remaining > 0 {
		let chunk = if remaining >= 12 {
			12 // 180°
		} else if remaining >= 6 {
			6 // 90°
		} else {
			1 // 15°
		};
		let yaw_sector = at + chunk - 1;
		push_solid(
			partitions,
			center_xz,
			ring_scale,
			yaw_sector as f32 * SEG_DEG,
			chunk as f32 * SEG_DEG,
			style,
		);
		at += chunk;
		remaining -= chunk;
	}
}

fn push_solid(
	partitions: &mut Vec<PartitionNode>,
	origin: Vec3,
	ring_scale: Vec3,
	start_deg: f32,
	sweep_deg: f32,
	style: PartitionStyle,
) {
	if sweep_deg > 1e-2 && ring_scale.y > EPS {
		partitions.push(PartitionNode::new(
			style,
			Partition::arc(sweep_deg),
			Placement::new(origin, start_deg.to_radians()).with_scale(ring_scale),
		));
	}
}

fn push_slice(
	partitions: &mut Vec<PartitionNode>,
	origin: Vec3,
	ring_scale: Vec3,
	start_deg: f32,
	sweep_deg: f32,
	style: PartitionStyle,
) {
	if sweep_deg > 1e-2 && ring_scale.y > EPS {
		partitions.push(PartitionNode::new(
			style,
			Partition::slice_arc(sweep_deg),
			Placement::new(origin, start_deg.to_radians()).with_scale(ring_scale),
		));
	}
}
