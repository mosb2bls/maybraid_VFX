//! Passage-face clearance packing for usage areas (stalls, rooms, …).
//!
//! Collect long faces of [`OpeningLabel::Passage`] openings on a plan host and
//! build inward keep-out bands so furniture / regions stay clear of the entry.
//!
//! Also: pack regions that **abut** a clearance band without overlapping it
//! (walkway stays open; furniture / sinks sit against its edge).

use bevy_math::bounding::Aabb2d;
use bevy_math::Vec2;
use procedural_common::{
	aabb2_area, aabb3_to_plan, clamp_min_size2, inflate_aabb2, intersects_aabb2,
	max_empty_rect2_by, touches_aabb2, Aabb2dPack, PlanAxes, PlanOpeningFace,
};

use crate::fit::Confines;
use crate::openings::OpeningLabel;

/// Default inward clearance kept free in front of passages (m).
pub const PASSAGE_CLEARANCE: f32 = 1.0;

/// Lateral / approach pad around an authored door keep-out band (m).
///
/// Used when rejecting blocked sales-face doors and when committing door clears
/// so later furniture stays out of the approach.
pub const PASSAGE_APPROACH_PAD: f32 = 0.5;

/// Inflated approach zone around an authored door keep-out.
pub fn approach_zone(door_clear: Aabb2d) -> Aabb2d {
	inflate_aabb2(door_clear, PASSAGE_APPROACH_PAD)
}

/// True when the padded door approach intersects any existing clearance.
pub fn approach_blocked(door_clear: Aabb2d, clearances: &[Aabb2d]) -> bool {
	let zone = approach_zone(door_clear);
	clearances.iter().any(|c| intersects_aabb2(zone, *c))
}

/// Push a door keep-out into `clearances`, optionally inflated by `pad`.
///
/// Residential packs use [`PASSAGE_APPROACH_PAD`]; commercial stalls typically
/// pass `0.0` to preserve prior density.
pub fn commit_door_clear(clearances: &mut Vec<Aabb2d>, door_clear: Aabb2d, pad: f32) {
	if pad > 1e-6 {
		clearances.push(inflate_aabb2(door_clear, pad));
	} else {
		clearances.push(door_clear);
	}
}

/// Host / passage face helpers on the XZ plan.
pub struct PlanHost;

impl PlanHost {
	/// Four cardinal host faces (XZ plan).
	pub fn faces(host: Aabb2d) -> [PlanOpeningFace; 4] {
		[
			PlanOpeningFace {
				thru_is_x: true,
				thru: host.min.x,
				along0: host.min.y,
				along1: host.max.y,
				inward_positive: true,
			},
			PlanOpeningFace {
				thru_is_x: true,
				thru: host.max.x,
				along0: host.min.y,
				along1: host.max.y,
				inward_positive: false,
			},
			PlanOpeningFace {
				thru_is_x: false,
				thru: host.min.y,
				along0: host.min.x,
				along1: host.max.x,
				inward_positive: true,
			},
			PlanOpeningFace {
				thru_is_x: false,
				thru: host.max.y,
				along0: host.min.x,
				along1: host.max.x,
				inward_positive: false,
			},
		]
	}

	pub fn same_wall(a: PlanOpeningFace, b: PlanOpeningFace) -> bool {
		a.thru_is_x == b.thru_is_x && (a.thru - b.thru).abs() < 0.2
	}

	/// Host walls that do not carry a customer passage face.
	pub fn free_faces(host: Aabb2d, passage_faces: &[PlanOpeningFace]) -> Vec<PlanOpeningFace> {
		Self::faces(host)
			.into_iter()
			.filter(|wall| !passage_faces.iter().any(|p| Self::same_wall(*wall, *p)))
			.collect()
	}
}

/// Collect passage opening faces and build inward clearance bands.
pub struct PassageClearance;

/// Authored doors straddle the panel; [`PlanOpeningFace::from_passage`] puts
/// `thru` on the inner volume face (~half panel inside the room). Wall-flush
/// furniture then sits outside a keep-out that starts inset from the wall.
const WALL_SNAP: f32 = 0.28;

/// Snap a passage face's thru coordinate onto the host wall it rides.
pub fn snap_face_to_host_wall(face: PlanOpeningFace, host: Aabb2d) -> PlanOpeningFace {
	if face.thru_is_x {
		if (face.thru - host.min.x).abs() <= WALL_SNAP {
			return PlanOpeningFace { thru: host.min.x, inward_positive: true, ..face };
		}
		if (face.thru - host.max.x).abs() <= WALL_SNAP {
			return PlanOpeningFace { thru: host.max.x, inward_positive: false, ..face };
		}
	} else {
		if (face.thru - host.min.y).abs() <= WALL_SNAP {
			return PlanOpeningFace { thru: host.min.y, inward_positive: true, ..face };
		}
		if (face.thru - host.max.y).abs() <= WALL_SNAP {
			return PlanOpeningFace { thru: host.max.y, inward_positive: false, ..face };
		}
	}
	face
}

impl PassageClearance {
	pub fn collect_faces(confines: &Confines, host: Aabb2d) -> Vec<PlanOpeningFace> {
		let mut out = Vec::new();
		for (_id, opening) in confines.openings.iter() {
			if !matches!(opening.label, OpeningLabel::Passage) {
				continue;
			}
			let passage_plan = aabb3_to_plan(&opening.bounds, PlanAxes::XZ);
			if let Some(face) = PlanOpeningFace::from_passage(host, passage_plan) {
				out.push(face);
			}
		}
		out
	}

	/// [`Self::collect_faces`] with thru snapped to the host wall (residential).
	pub fn collect_faces_wall_snapped(confines: &Confines, host: Aabb2d) -> Vec<PlanOpeningFace> {
		Self::collect_faces(confines, host)
			.into_iter()
			.map(|f| snap_face_to_host_wall(f, host))
			.collect()
	}

	/// Inward bands of `depth` along each passage face.
	pub fn bands(host: Aabb2d, faces: &[PlanOpeningFace], depth: f32) -> Vec<Aabb2d> {
		let mut out = Vec::new();
		for &face in faces {
			let along = face.along_len();
			if let Some(band) = face.band(host, along, depth, 0.5) {
				out.push(band);
			}
		}
		out
	}

	/// [`Self::bands`] at [`PASSAGE_CLEARANCE`].
	pub fn bands_std(host: Aabb2d, faces: &[PlanOpeningFace]) -> Vec<Aabb2d> {
		Self::bands(host, faces, PASSAGE_CLEARANCE)
	}

	/// Shallow inward bands spanning the **full host wall** of each passage.
	///
	/// Door-width bands alone leave the rest of a hall wall free for flush
	/// counters/sofas. A short lip (see [`PASSAGE_WALL_LIP`]) blocks wall-flush
	/// casework without the deep full-wall keep-outs that crush open packs.
	pub fn bands_wall_lip(host: Aabb2d, faces: &[PlanOpeningFace], depth: f32) -> Vec<Aabb2d> {
		let mut out = Vec::new();
		for &face in faces {
			let full = if face.thru_is_x {
				PlanOpeningFace { along0: host.min.y, along1: host.max.y, ..face }
			} else {
				PlanOpeningFace { along0: host.min.x, along1: host.max.x, ..face }
			};
			let along = full.along_len();
			if let Some(band) = full.band(host, along, depth, 0.5) {
				out.push(band);
			}
		}
		out
	}
}

/// Shallow keep-out depth (m) along a passage wall for open residential packs.
pub const PASSAGE_WALL_LIP: f32 = 0.55;

/// True when `region` shares an edge with `clearance` but does not open-overlap it.
///
/// Typical use: furniture / sink bands sit against a passage keep-out without
/// eating the walkway (`clearance` should also be in the packer's hard excludes).
pub fn abuts_clearance(region: Aabb2d, clearance: Aabb2d) -> bool {
	touches_aabb2(region, clearance) && !intersects_aabb2(region, clearance)
}

/// Largest empty rect in `host` avoiding `hard`, strongly preferring candidates
/// that [`abuts_clearance`] `clearance` and meet `min_size` on both axes.
///
/// `hard` should already include `clearance`.
pub fn max_empty_abutting_clearance(
	host: Aabb2d,
	hard: &[Aabb2d],
	clearance: Aabb2d,
) -> Option<Aabb2d> {
	max_empty_abutting_clearance_sized(host, hard, clearance, Vec2::ZERO)
}

/// Like [`max_empty_abutting_clearance`], but ignores candidates smaller than `min_size`.
pub fn max_empty_abutting_clearance_sized(
	host: Aabb2d,
	hard: &[Aabb2d],
	clearance: Aabb2d,
	min_size: Vec2,
) -> Option<Aabb2d> {
	const EPS: f32 = 1e-3;
	max_empty_rect2_by(host, hard, |r| {
		let w = r.max.x - r.min.x;
		let d = r.max.y - r.min.y;
		if w + EPS < min_size.x || d + EPS < min_size.y {
			return f32::NEG_INFINITY;
		}
		let area = aabb2_area(r);
		if abuts_clearance(r, clearance) {
			return area + 1.0e6;
		}
		// Soft pull toward the clearance edge when a perfect abut is unavailable.
		let near = inflate_aabb2(clearance, 0.35);
		if touches_aabb2(r, near) {
			area + 1.0e3
		} else {
			area
		}
	})
	.filter(|r| {
		let w = r.max.x - r.min.x;
		let d = r.max.y - r.min.y;
		w + EPS >= min_size.x && d + EPS >= min_size.y
	})
}

/// Seed + grow a region that stays clear of `hard` (including `clearance`) and
/// abuts `clearance` — kitchen/sink style fill against a keep-out band.
pub fn pack_abutting_clearance(
	host: Aabb2d,
	hard: &[Aabb2d],
	clearance: Aabb2d,
	min_size: Vec2,
	area_target: f32,
) -> Option<Aabb2d> {
	let seed = max_empty_abutting_clearance_sized(host, hard, clearance, min_size)?;
	let seed = clamp_min_size2(seed, min_size)?;
	let target = area_target.max(min_size.x * min_size.y);
	let grown = seed.grow_toward_area(host, hard, target).grow_into(host, hard);
	let grown = clamp_min_size2(grown, min_size)?;
	if !grown.is_clear_of(hard) {
		return None;
	}
	if abuts_clearance(grown, clearance) {
		return Some(grown);
	}
	// Growth can pull off the clearance edge; keep the abutting seed if valid.
	if abuts_clearance(seed, clearance) && seed.is_clear_of(hard) {
		return Some(seed);
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec2;

	#[test]
	fn abuts_clearance_accepts_edge_contact_only() {
		let clear = Aabb2d { min: Vec2::new(0.0, 0.0), max: Vec2::new(1.0, 2.0) };
		let beside = Aabb2d { min: Vec2::new(1.0, 0.0), max: Vec2::new(2.0, 1.0) };
		let overlap = Aabb2d { min: Vec2::new(0.5, 0.0), max: Vec2::new(1.5, 1.0) };
		let far = Aabb2d { min: Vec2::new(1.2, 0.0), max: Vec2::new(2.0, 1.0) };
		assert!(abuts_clearance(beside, clear));
		assert!(!abuts_clearance(overlap, clear));
		assert!(!abuts_clearance(far, clear));
	}

	#[test]
	fn snap_face_moves_thru_to_host_wall() {
		let host = Aabb2d { min: Vec2::new(0.0, 0.0), max: Vec2::new(4.0, 5.0) };
		let inset = PlanOpeningFace {
			thru_is_x: true,
			thru: 0.12,
			along0: 1.0,
			along1: 2.0,
			inward_positive: true,
		};
		let snapped = snap_face_to_host_wall(inset, host);
		assert!((snapped.thru - host.min.x).abs() < 1e-4);
		assert!(snapped.inward_positive);
	}

	#[test]
	fn pack_abutting_clearance_grows_beside_keepout() {
		let host = Aabb2d { min: Vec2::ZERO, max: Vec2::new(6.0, 4.0) };
		let clearance = Aabb2d { min: Vec2::new(0.0, 0.0), max: Vec2::new(1.0, 4.0) };
		let hard = [clearance];
		let packed =
			pack_abutting_clearance(host, &hard, clearance, Vec2::splat(0.5), 8.0).unwrap();
		assert!(abuts_clearance(packed, clearance));
		assert!(packed.is_clear_of(&hard));
		assert!(aabb2_area(packed) + 1e-3 >= 0.5 * 0.5);
	}

	#[test]
	fn wall_flush_blocked_by_snapped_passage() {
		use crate::fit::Confines;
		use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
		use crate::placer::init_host;
		use bevy_math::bounding::Aabb3d;
		use bevy_math::Vec3;
		use procedural_common::intersects_aabb2;

		// Room north of a door on its south wall (thru on y=0).
		let host3 = Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 3.0, 3.5));
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::scoped("t", "pass", "0"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(1.5, 0.0, -0.12), Vec3::new(2.5, 2.1, 0.12)),
				OpeningLabel::Passage,
			),
		);
		let confines = Confines::new(host3, 0.0, openings);
		let host = init_host(&confines).unwrap();
		let flush = Aabb2d { min: Vec2::new(0.2, 0.0), max: Vec2::new(3.8, 0.7) };
		assert!(
			host.clearances.iter().any(|c| intersects_aabb2(flush, *c)),
			"wall-flush counter must hit snapped passage clearance, bands={:?}",
			host.clearances
		);
	}
}
