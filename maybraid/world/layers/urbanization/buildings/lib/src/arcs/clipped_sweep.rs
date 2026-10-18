//! Circular [`ClippedArcSweep`]: solid sub-sweeps with angular clip openings.

use bevy_math::Vec3;
use building_components::partitions::{Partition, PartitionNode, PartitionStyle};
use building_components::{BuildingComponents, Layers, Placement};
use lod::scene::LodSceneLevel;

use crate::portals::SLICE_Y_FRAC;

/// Fitted circular arc with sorted non-overlapping clip intervals in \(t \in [0, 1]\).
///
/// Clips punch openings: solid [`Partition::arc`] on the complement, lintel
/// [`Partition::slice_arc`] bands over each clip. Portal assignment lives in
/// [`crate::arcs::portal_ring`].
///
/// Ellipses are out of scope — see [`crate::arcs`] module docs.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedArcSweep {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	pub sweep_degrees: f32,
	pub start_yaw: f32,
	pub style: PartitionStyle,
	/// Normalized clips \((t_0, t_1)\) with \(0 \le t_0 < t_1 \le 1\), non-overlapping, sorted.
	pub clip_intervals: Vec<(f32, f32)>,
	pub partitions: Vec<PartitionNode>,
}

impl ClippedArcSweep {
	pub fn new(
		center_xz: Vec3,
		radius: f32,
		storey_height: f32,
		sweep_degrees: f32,
		start_yaw: f32,
		style: PartitionStyle,
		clip_intervals: impl IntoIterator<Item = (f32, f32)>,
	) -> Self {
		let radius = radius.max(1e-4);
		let storey_height = storey_height.max(1e-4);
		let sweep_degrees = sweep_degrees.clamp(1e-2, 360.0);
		let clips = normalize_clips(clip_intervals);
		let partitions =
			tessellate(center_xz, radius, storey_height, sweep_degrees, start_yaw, style, &clips);
		Self {
			center_xz,
			radius,
			storey_height,
			sweep_degrees,
			start_yaw,
			style,
			clip_intervals: clips,
			partitions,
		}
	}

	pub fn rough_stone(
		center_xz: Vec3,
		radius: f32,
		storey_height: f32,
		sweep_degrees: f32,
		start_yaw: f32,
		clip_intervals: impl IntoIterator<Item = (f32, f32)>,
	) -> Self {
		Self::new(
			center_xz,
			radius,
			storey_height,
			sweep_degrees,
			start_yaw,
			PartitionStyle::RoughStonework,
			clip_intervals,
		)
	}
}

impl BuildingComponents for ClippedArcSweep {
	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
		Layers::from_free(self.partitions.clone())
	}
}

fn normalize_clips(clips: impl IntoIterator<Item = (f32, f32)>) -> Vec<(f32, f32)> {
	let mut out: Vec<(f32, f32)> = clips
		.into_iter()
		.filter_map(|(a, b)| {
			let lo = a.min(b).clamp(0.0, 1.0);
			let hi = a.max(b).clamp(0.0, 1.0);
			if hi - lo > 1e-4 {
				Some((lo, hi))
			} else {
				None
			}
		})
		.collect();
	out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
	let mut merged = Vec::new();
	for (t0, t1) in out {
		if let Some((_, last_end)) = merged.last() {
			if t0 < *last_end - 1e-5 {
				// Skip overlapping / nested; keep the earlier interval.
				continue;
			}
		}
		merged.push((t0, t1));
	}
	merged
}

fn tessellate(
	center_xz: Vec3,
	radius: f32,
	storey_height: f32,
	sweep_degrees: f32,
	start_yaw: f32,
	style: PartitionStyle,
	clips: &[(f32, f32)],
) -> Vec<PartitionNode> {
	let ring_scale = Vec3::new(radius, storey_height, radius);
	let lintel = center_xz + Vec3::Y * (SLICE_Y_FRAC * storey_height);
	let mut partitions = Vec::new();

	if clips.is_empty() {
		push_solid(&mut partitions, center_xz, ring_scale, start_yaw, 0.0, sweep_degrees, style);
		return partitions;
	}

	let closed = (sweep_degrees - 360.0).abs() < 0.5;

	if closed {
		for i in 0..clips.len() {
			let (t0, t1) = clips[i];
			push_slice(
				&mut partitions,
				lintel,
				ring_scale,
				start_yaw,
				t0 * sweep_degrees,
				(t1 - t0) * sweep_degrees,
				style,
			);
			let solid_start = t1 * sweep_degrees;
			let next_t0 = clips[(i + 1) % clips.len()].0;
			let solid_end = if i + 1 < clips.len() {
				next_t0 * sweep_degrees
			} else {
				next_t0 * sweep_degrees + sweep_degrees
			};
			push_solid(
				&mut partitions,
				center_xz,
				ring_scale,
				start_yaw,
				solid_start,
				(solid_end - solid_start).max(0.0),
				style,
			);
		}
		return partitions;
	}

	// Open sweep: solid → (slice → solid)* covering [0, sweep].
	push_solid(
		&mut partitions,
		center_xz,
		ring_scale,
		start_yaw,
		0.0,
		clips[0].0 * sweep_degrees,
		style,
	);
	for (i, &(t0, t1)) in clips.iter().enumerate() {
		push_slice(
			&mut partitions,
			lintel,
			ring_scale,
			start_yaw,
			t0 * sweep_degrees,
			(t1 - t0) * sweep_degrees,
			style,
		);
		let solid_start = t1 * sweep_degrees;
		let solid_end =
			if i + 1 < clips.len() { clips[i + 1].0 * sweep_degrees } else { sweep_degrees };
		push_solid(
			&mut partitions,
			center_xz,
			ring_scale,
			start_yaw,
			solid_start,
			(solid_end - solid_start).max(0.0),
			style,
		);
	}

	partitions
}

fn push_solid(
	partitions: &mut Vec<PartitionNode>,
	center_xz: Vec3,
	ring_scale: Vec3,
	start_yaw: f32,
	start_deg: f32,
	sweep_deg: f32,
	style: PartitionStyle,
) {
	if sweep_deg > 1e-2 {
		partitions.push(PartitionNode::new(
			style,
			Partition::arc(sweep_deg),
			Placement::new(center_xz, start_yaw + start_deg.to_radians()).with_scale(ring_scale),
		));
	}
}

fn push_slice(
	partitions: &mut Vec<PartitionNode>,
	lintel: Vec3,
	ring_scale: Vec3,
	start_yaw: f32,
	start_deg: f32,
	sweep_deg: f32,
	style: PartitionStyle,
) {
	if sweep_deg > 1e-2 {
		partitions.push(PartitionNode::new(
			style,
			Partition::slice_arc(sweep_deg),
			Placement::new(lintel, start_yaw + start_deg.to_radians()).with_scale(ring_scale),
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn no_clips_is_solid() {
		let a = ClippedArcSweep::rough_stone(Vec3::ZERO, 4.0, 3.0, 180.0, 0.0, []);
		assert_eq!(a.partitions.len(), 1);
		assert!(matches!(a.partitions[0].geometry, Partition::Arc(_)));
	}

	#[test]
	fn middle_clip_yields_slice_and_solids() {
		let a = ClippedArcSweep::rough_stone(Vec3::ZERO, 4.0, 3.0, 180.0, 0.0, [(0.25, 0.4)]);
		let arcs = a.partitions.iter().filter(|p| matches!(p.geometry, Partition::Arc(_))).count();
		let slices = a
			.partitions
			.iter()
			.filter(|p| matches!(p.geometry, Partition::SliceArc(_)))
			.count();
		assert_eq!(slices, 1);
		assert_eq!(arcs, 2);
	}
}
