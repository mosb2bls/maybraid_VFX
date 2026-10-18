//! Polyline stadium corridors, closest-point queries, and piecewise grade.

use bevy_math::Vec2;
use procedural_common::Smoothstep01;

/// Result of projecting a point onto a polyline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosestOnPolyline {
	/// Euclidean distance from the query to the closest point on the path.
	pub distance: f32,
	/// Arc length from the first vertex to the closest point.
	pub arc_s: f32,
	/// Total polyline length.
	pub total_len: f32,
	/// Closest point on the path (xz as `Vec2`).
	pub point: Vec2,
	/// Index of the segment `[points[i], points[i+1]]` that owns the closest point.
	pub segment_index: usize,
	/// Parameter along that segment in `[0, 1]` (`0` = start vertex).
	pub local_t: f32,
}

impl ClosestOnPolyline {
	/// Normalized progress along the path in `[0, 1]`.
	pub fn t(&self) -> f32 {
		if self.total_len <= 1e-6 {
			0.0
		} else {
			(self.arc_s / self.total_len).clamp(0.0, 1.0)
		}
	}
}

/// Closest point / arc-length query on an open polyline.
///
/// Empty path → zeroed result at the origin. Single vertex → distance to that point.
pub fn closest_on_polyline(path: &[Vec2], p: Vec2) -> ClosestOnPolyline {
	if path.is_empty() {
		return ClosestOnPolyline {
			distance: p.length(),
			arc_s: 0.0,
			total_len: 0.0,
			point: Vec2::ZERO,
			segment_index: 0,
			local_t: 0.0,
		};
	}
	if path.len() == 1 {
		return ClosestOnPolyline {
			distance: p.distance(path[0]),
			arc_s: 0.0,
			total_len: 0.0,
			point: path[0],
			segment_index: 0,
			local_t: 0.0,
		};
	}

	let mut best_d2 = f32::INFINITY;
	let mut best_s = 0.0;
	let mut best_p = path[0];
	let mut best_seg = 0usize;
	let mut best_t = 0.0;
	let mut prefix = 0.0;
	let mut total = 0.0;

	for (seg_i, window) in path.windows(2).enumerate() {
		let a = window[0];
		let b = window[1];
		let ab = b - a;
		let len = ab.length();
		total += len;
		if len <= 1e-8 {
			let d2 = p.distance_squared(a);
			if d2 < best_d2 {
				best_d2 = d2;
				best_s = prefix;
				best_p = a;
				best_seg = seg_i;
				best_t = 0.0;
			}
			continue;
		}
		let t = ((p - a).dot(ab) / (len * len)).clamp(0.0, 1.0);
		let q = a + ab * t;
		let d2 = p.distance_squared(q);
		if d2 < best_d2 {
			best_d2 = d2;
			best_s = prefix + t * len;
			best_p = q;
			best_seg = seg_i;
			best_t = t;
		}
		prefix += len;
	}

	ClosestOnPolyline {
		distance: best_d2.sqrt(),
		arc_s: best_s,
		total_len: total,
		point: best_p,
		segment_index: best_seg,
		local_t: best_t,
	}
}

fn segment_len(path: &[Vec2], seg_i: usize) -> f32 {
	if seg_i + 1 >= path.len() {
		return 0.0;
	}
	path[seg_i].distance(path[seg_i + 1])
}

fn pitch_into_node(path: &[Vec2], levels: &[f32], node: usize) -> f32 {
	if node == 0 || node >= levels.len() || node >= path.len() {
		return pitch_out_of_node(path, levels, node);
	}
	let len = segment_len(path, node - 1).max(1e-6);
	(levels[node] - levels[node - 1]) / len
}

fn pitch_out_of_node(path: &[Vec2], levels: &[f32], node: usize) -> f32 {
	if node + 1 >= levels.len() || node + 1 >= path.len() {
		return if node > 0 { pitch_into_node(path, levels, node) } else { 0.0 };
	}
	let len = segment_len(path, node).max(1e-6);
	(levels[node + 1] - levels[node]) / len
}

/// Elevation at signed arc offset `s` from node `j`, blending inbound/outbound pitch.
///
/// `s > 0` is downstream (toward higher indices), `s < 0` upstream.
fn elevation_from_node_pitches(
	path: &[Vec2],
	levels: &[f32],
	node: usize,
	s: f32,
	blend: f32,
) -> f32 {
	let w0 = levels.get(node).copied().unwrap_or(0.0);
	let pitch_in = pitch_into_node(path, levels, node);
	let pitch_out = pitch_out_of_node(path, levels, node);
	let blend = blend.max(1e-6);
	// Map s ∈ [-blend, +blend] → u ∈ [0, 1] (inbound → outbound).
	let u = Smoothstep01::eval((s / blend + 1.0) * 0.5);
	let pitch = pitch_in + (pitch_out - pitch_in) * u;
	w0 + pitch * s
}

/// Piecewise-linear grade along a polyline with pitch blending near vertices.
///
/// On each segment, elevations lerp between node levels. Within `node_blend`
/// world units of a vertex (along the path), inbound and outbound pitches are
/// blended so kinks do not produce a hard slope discontinuity for nearby samples.
///
/// `levels.len()` should match `path.len()`; mismatched tails are clamped.
pub fn grade_along_polyline(path: &[Vec2], levels: &[f32], p: Vec2, node_blend: f32) -> f32 {
	if path.is_empty() || levels.is_empty() {
		return 0.0;
	}
	if path.len() == 1 || levels.len() == 1 {
		return levels[0];
	}
	let n = path.len().min(levels.len());
	let path = &path[..n];
	let levels = &levels[..n];
	if n == 1 {
		return levels[0];
	}

	let c = closest_on_polyline(path, p);
	let seg = c.segment_index.min(n - 2);
	let len = segment_len(path, seg).max(1e-6);
	let w_a = levels[seg];
	let w_b = levels[seg + 1];
	let w_linear = w_a + (w_b - w_a) * c.local_t;

	let blend = node_blend.max(0.0);
	if blend <= 1e-6 {
		return w_linear;
	}

	let s_from_a = c.local_t * len;
	let s_from_b = (1.0 - c.local_t) * len;
	let mut w = w_linear;
	let mut blend_w = 0.0;

	if s_from_a < blend {
		let alpha = Smoothstep01::eval(1.0 - s_from_a / blend);
		let w_node = elevation_from_node_pitches(path, levels, seg, s_from_a, blend);
		w += alpha * (w_node - w);
		blend_w += alpha;
	}
	if s_from_b < blend {
		let alpha = Smoothstep01::eval(1.0 - s_from_b / blend);
		// Upstream of node seg+1 ⇒ negative s.
		let w_node = elevation_from_node_pitches(path, levels, seg + 1, -s_from_b, blend);
		// Renormalize if both ends contribute on a short segment.
		if blend_w > 1e-6 {
			let t = alpha / (blend_w + alpha);
			w = w + t * (w_node - w);
		} else {
			w += alpha * (w_node - w);
		}
	}
	w
}

/// Stadium-chain corridor: points along a polyline with constant half-width.
#[derive(Debug, Clone)]
pub struct PolylineRegion {
	pub points: Vec<Vec2>,
	pub half_width: f32,
}

impl PolylineRegion {
	pub fn new(points: Vec<Vec2>, half_width: f32) -> Self {
		Self { points, half_width: half_width.max(1e-3) }
	}

	/// Representative sample point (mid-path) for wet-volume probes.
	pub fn sample_point(&self) -> Vec2 {
		if self.points.is_empty() {
			return Vec2::ZERO;
		}
		if self.points.len() == 1 {
			return self.points[0];
		}
		let mid = self.points.len() / 2;
		self.points[mid]
	}

	pub fn sdf(&self, p: Vec2) -> f32 {
		closest_on_polyline(&self.points, p).distance - self.half_width
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn straight_path_midpoint_and_offset() -> anyhow::Result<()> {
		let path = [Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)];
		let on = closest_on_polyline(&path, Vec2::new(50.0, 0.0));
		assert!((on.arc_s - 50.0).abs() < 1e-3);
		assert!(on.distance < 1e-3);
		assert!((on.total_len - 100.0).abs() < 1e-3);
		assert_eq!(on.segment_index, 0);
		assert!((on.local_t - 0.5).abs() < 1e-3);

		let off = closest_on_polyline(&path, Vec2::new(50.0, 8.0));
		assert!((off.arc_s - 50.0).abs() < 1e-3);
		assert!((off.distance - 8.0).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn polyline_region_inside_outside() -> anyhow::Result<()> {
		let region = PolylineRegion::new(vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0)], 5.0);
		assert!(region.sdf(Vec2::new(20.0, 0.0)) < 0.0);
		assert!(region.sdf(Vec2::new(20.0, 10.0)) > 0.0);
		Ok(())
	}

	#[test]
	fn piecewise_grade_uses_local_segment_pitch() -> anyhow::Result<()> {
		// Steep first half, flat second half — global head→toe would be wrong at mid.
		let path = vec![Vec2::new(0.0, 0.0), Vec2::new(50.0, 0.0), Vec2::new(100.0, 0.0)];
		let levels = vec![50.0, 40.0, 40.0];
		let mid0 = grade_along_polyline(&path, &levels, Vec2::new(25.0, 0.0), 0.0);
		assert!((mid0 - 45.0).abs() < 1e-3, "first segment mid={mid0}");
		let mid1 = grade_along_polyline(&path, &levels, Vec2::new(75.0, 0.0), 0.0);
		assert!((mid1 - 40.0).abs() < 1e-3, "second segment mid={mid1}");
		Ok(())
	}

	#[test]
	fn node_pitch_blend_is_continuous_at_vertex() -> anyhow::Result<()> {
		let path = vec![Vec2::new(0.0, 0.0), Vec2::new(50.0, 0.0), Vec2::new(100.0, 0.0)];
		let levels = vec![50.0, 30.0, 20.0];
		let at_node = grade_along_polyline(&path, &levels, Vec2::new(50.0, 0.0), 10.0);
		assert!((at_node - 30.0).abs() < 1e-2, "W at node={at_node}");
		let just_before = grade_along_polyline(&path, &levels, Vec2::new(49.0, 0.0), 10.0);
		let just_after = grade_along_polyline(&path, &levels, Vec2::new(51.0, 0.0), 10.0);
		assert!((just_before - at_node).abs() < 2.0);
		assert!((just_after - at_node).abs() < 2.0);
		Ok(())
	}
}
