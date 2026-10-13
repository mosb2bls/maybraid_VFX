//! Plan adjacency and junction detection for orthogonal massing.
//!
//! Perpendicular pairs → L / T / full-cross concave corners.
//! Same-axis coaxial pairs → end-meets (lower run into higher end gable).

use super::geometry::{LongAxis, VolumeCandidate, EPS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlanRect {
	pub min_x: f32,
	pub min_z: f32,
	pub max_x: f32,
	pub max_z: f32,
}

impl PlanRect {
	pub fn from_candidate(c: &VolumeCandidate) -> Self {
		let (min_x, min_z) = c.plan_min();
		let (max_x, max_z) = c.plan_max();
		Self { min_x, min_z, max_x, max_z }
	}

	pub fn overlap(self, other: Self) -> Option<Self> {
		let min_x = self.min_x.max(other.min_x);
		let min_z = self.min_z.max(other.min_z);
		let max_x = self.max_x.min(other.max_x);
		let max_z = self.max_z.min(other.max_z);
		if max_x + EPS < min_x || max_z + EPS < min_z {
			return None;
		}
		Some(Self { min_x, min_z, max_x: max_x.max(min_x), max_z: max_z.max(min_z) })
	}
}

/// A concave plan corner where two perpendicular volumes meet (valley site).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ConcaveCorner {
	/// Long-X volume index.
	pub vol_a: usize,
	/// Long-Z volume index.
	pub vol_b: usize,
	/// Eave/wall side on A: `0` = −Z, `1` = +Z.
	pub side_a: usize,
	/// Eave/wall side on B: `0` = −X, `1` = +X.
	pub side_b: usize,
	/// Junction long-end on A (`0` = min X, `1` = max X), if any.
	pub end_a: Option<usize>,
	/// Junction long-end on B (`0` = min Z, `1` = max Z), if any.
	pub end_b: Option<usize>,
	/// Inner corner in XZ (massing wall plan).
	pub corner_xz: (f32, f32),
}

/// Same-axis end meet: a lower run butts into a higher volume's end gable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CoaxialMeet {
	/// Volume whose long end is stripped back to the gable plane.
	pub vol_run: usize,
	/// Higher / wider host; eaves stay full (end cap / gable drawn).
	pub vol_cap: usize,
	/// Long end on `vol_run` at the interface (`0` = min, `1` = max).
	pub run_end: usize,
	pub long_axis: LongAxis,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct JunctionSet {
	pub perp: Vec<ConcaveCorner>,
	pub coaxial: Vec<CoaxialMeet>,
}

/// Find orthogonal junctions and mark strip-back ends on candidates.
pub(super) fn resolve_junctions(volumes: &mut [VolumeCandidate]) -> JunctionSet {
	let n = volumes.len();
	let rects: Vec<PlanRect> = volumes.iter().map(PlanRect::from_candidate).collect();
	let mut out = JunctionSet::default();

	for i in 0..n {
		for j in (i + 1)..n {
			if volumes[i].long_axis == volumes[j].long_axis {
				if let Some(meets) = classify_coaxial(i, j, volumes, &rects) {
					for m in &meets {
						volumes[m.vol_run].end_free[m.run_end] = false;
					}
					out.coaxial.extend(meets);
				}
				continue;
			}
			let (ia, ib, ra, rb) = if volumes[i].long_axis == LongAxis::X {
				(i, j, rects[i], rects[j])
			} else {
				(j, i, rects[j], rects[i])
			};
			let Some(overlap) = ra.overlap(rb) else {
				continue;
			};
			for c in classify_xz_pair(ia, ib, ra, rb, overlap) {
				if let Some(end) = c.end_a {
					volumes[c.vol_a].end_free[end] = false;
				}
				if let Some(end) = c.end_b {
					volumes[c.vol_b].end_free[end] = false;
				}
				out.perp.push(c);
			}
		}
	}
	out
}

fn classify_xz_pair(
	vol_a: usize,
	vol_b: usize,
	ra: PlanRect,
	rb: PlanRect,
	overlap: PlanRect,
) -> Vec<ConcaveCorner> {
	let a_pos = ra.max_x > overlap.max_x + EPS;
	let a_neg = ra.min_x < overlap.min_x - EPS;
	let b_pos = rb.max_z > overlap.max_z + EPS;
	let b_neg = rb.min_z < overlap.min_z - EPS;

	let mut out = Vec::new();

	// L: each arm extends one way past the overlap.
	if (a_pos ^ a_neg) && (b_pos ^ b_neg) {
		let corner_x = if a_pos { overlap.max_x } else { overlap.min_x };
		let corner_z = if b_pos { overlap.max_z } else { overlap.min_z };
		let side_a = if b_pos { 1 } else { 0 };
		let side_b = if a_pos { 1 } else { 0 };
		let end_a = if a_pos {
			if (ra.min_x - overlap.min_x).abs() <= EPS {
				Some(0)
			} else {
				None
			}
		} else if (ra.max_x - overlap.max_x).abs() <= EPS {
			Some(1)
		} else {
			None
		};
		let end_b = if b_pos {
			if (rb.min_z - overlap.min_z).abs() <= EPS {
				Some(0)
			} else {
				None
			}
		} else if (rb.max_z - overlap.max_z).abs() <= EPS {
			Some(1)
		} else {
			None
		};
		out.push(ConcaveCorner {
			vol_a,
			vol_b,
			side_a,
			side_b,
			end_a,
			end_b,
			corner_xz: (corner_x, corner_z),
		});
		return out;
	}

	// T: bar extends both ways along X, stem extends one way along Z.
	if a_pos && a_neg && (b_pos ^ b_neg) {
		let corner_z = if b_pos { overlap.max_z } else { overlap.min_z };
		let side_a = if b_pos { 1 } else { 0 };
		let end_b = if b_pos {
			if (rb.min_z - overlap.min_z).abs() <= EPS {
				Some(0)
			} else {
				None
			}
		} else if (rb.max_z - overlap.max_z).abs() <= EPS {
			Some(1)
		} else {
			None
		};
		for &(corner_x, side_b) in &[(overlap.min_x, 0usize), (overlap.max_x, 1usize)] {
			out.push(ConcaveCorner {
				vol_a,
				vol_b,
				side_a,
				side_b,
				end_a: None,
				end_b,
				corner_xz: (corner_x, corner_z),
			});
		}
		return out;
	}

	// T: bar along Z, stem along X.
	if b_pos && b_neg && (a_pos ^ a_neg) {
		let corner_x = if a_pos { overlap.max_x } else { overlap.min_x };
		let side_b = if a_pos { 1 } else { 0 };
		let end_a = if a_pos {
			if (ra.min_x - overlap.min_x).abs() <= EPS {
				Some(0)
			} else {
				None
			}
		} else if (ra.max_x - overlap.max_x).abs() <= EPS {
			Some(1)
		} else {
			None
		};
		for &(corner_z, side_a) in &[(overlap.min_z, 0usize), (overlap.max_z, 1usize)] {
			out.push(ConcaveCorner {
				vol_a,
				vol_b,
				side_a,
				side_b,
				end_a,
				end_b: None,
				corner_xz: (corner_x, corner_z),
			});
		}
		return out;
	}

	// Full cross (+): both arms extend past the overlap both ways → four concave
	// corners. Neither volume strips (both are T-bars).
	if a_pos && a_neg && b_pos && b_neg {
		for &(corner_x, side_b) in &[(overlap.min_x, 0usize), (overlap.max_x, 1usize)] {
			for &(corner_z, side_a) in &[(overlap.min_z, 0usize), (overlap.max_z, 1usize)] {
				out.push(ConcaveCorner {
					vol_a,
					vol_b,
					side_a,
					side_b,
					end_a: None,
					end_b: None,
					corner_xz: (corner_x, corner_z),
				});
			}
		}
	}

	out
}

fn classify_coaxial(
	i: usize,
	j: usize,
	volumes: &[VolumeCandidate],
	rects: &[PlanRect],
) -> Option<Vec<CoaxialMeet>> {
	let a = &volumes[i];
	let b = &volumes[j];
	debug_assert_eq!(a.long_axis, b.long_axis);

	const MIDLINE_EPS: f32 = 0.05;
	let (mid_a, mid_b) = match a.long_axis {
		LongAxis::X => (a.ridge.a.z, b.ridge.a.z),
		LongAxis::Z => (a.ridge.a.x, b.ridge.a.x),
	};
	if (mid_a - mid_b).abs() > MIDLINE_EPS {
		return None;
	}

	// Cap = wider short span (hosts the end gable); run = the other.
	let (cap_i, run_i) = if a.short_span > b.short_span + EPS {
		(i, j)
	} else if b.short_span > a.short_span + EPS {
		(j, i)
	} else {
		return None;
	};
	let rc = rects[cap_i];
	let rr = rects[run_i];

	// Short-axis ranges must overlap so the run actually hits the gable face.
	let short_overlap = match a.long_axis {
		LongAxis::X => {
			let lo = rc.min_z.max(rr.min_z);
			let hi = rc.max_z.min(rr.max_z);
			hi - lo > EPS
		}
		LongAxis::Z => {
			let lo = rc.min_x.max(rr.min_x);
			let hi = rc.max_x.min(rr.max_x);
			hi - lo > EPS
		}
	};
	if !short_overlap {
		return None;
	}

	let mut meets = Vec::new();
	match a.long_axis {
		LongAxis::X => {
			if (rr.max_x - rc.min_x).abs() <= EPS {
				meets.push(CoaxialMeet {
					vol_run: run_i,
					vol_cap: cap_i,
					run_end: 1,
					long_axis: LongAxis::X,
				});
			}
			if (rr.min_x - rc.max_x).abs() <= EPS {
				meets.push(CoaxialMeet {
					vol_run: run_i,
					vol_cap: cap_i,
					run_end: 0,
					long_axis: LongAxis::X,
				});
			}
		}
		LongAxis::Z => {
			if (rr.max_z - rc.min_z).abs() <= EPS {
				meets.push(CoaxialMeet {
					vol_run: run_i,
					vol_cap: cap_i,
					run_end: 1,
					long_axis: LongAxis::Z,
				});
			}
			if (rr.min_z - rc.max_z).abs() <= EPS {
				meets.push(CoaxialMeet {
					vol_run: run_i,
					vol_cap: cap_i,
					run_end: 0,
					long_axis: LongAxis::Z,
				});
			}
		}
	}

	if meets.is_empty() {
		None
	} else {
		Some(meets)
	}
}
