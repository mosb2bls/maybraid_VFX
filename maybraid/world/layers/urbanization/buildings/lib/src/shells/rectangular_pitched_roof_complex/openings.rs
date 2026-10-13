//! Post-solve openings: assign world-space voids onto resolved pitched roofs.
//!
//! Geometry (valleys, strip-back, end-caps) is solved first. Each
//! [`OpeningLabel::Passage`] / [`OpeningLabel::Aperture`] is then given to the
//! nearest roof by pitch/gable centroid distance; that roof's existing openings
//! pipeline does the clipping and contact map.

use bevy_math::Vec3;

use crate::openings::{MappedOpenings, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
use crate::shells::pitched_rectangular_roof::{PitchedRoof, PitchedRoofParams};

use super::RectangularPitchedRoofComplex;

/// Partition authored openings onto roofs and rebuild those that receive any.
pub(super) fn apply_openings(
	roofs: Vec<PitchedRoof>,
	authored: &Openings,
) -> (Vec<PitchedRoof>, Openings, MappedOpenings) {
	if authored.is_empty() || roofs.is_empty() {
		return (roofs, Openings::new(), MappedOpenings::new());
	}

	let mut buckets: Vec<Openings> = (0..roofs.len()).map(|_| Openings::new()).collect();
	for (id, opening) in authored.iter() {
		if !matches!(opening.label, OpeningLabel::Passage | OpeningLabel::Aperture) {
			continue;
		}
		let Some(ri) = nearest_roof_index(opening, &roofs) else {
			continue;
		};
		buckets[ri].insert(id.clone(), opening.clone());
	}

	let mut out = Vec::with_capacity(roofs.len());
	let mut all_openings = Openings::new();
	let mut all_mapped = MappedOpenings::new();
	for (i, roof) in roofs.into_iter().enumerate() {
		if buckets[i].is_empty() {
			out.push(roof);
			continue;
		}
		let rebuilt = PitchedRoofParams::new(roof.params().halves.clone())
			.style(roof.params().style)
			.joint_thickness(roof.params().joint_thickness)
			.openings(std::mem::take(&mut buckets[i]))
			.build();
		for (id, opening) in rebuilt.openings().iter() {
			all_openings.insert(id.clone(), opening.clone());
			if let Some(mapped) = rebuilt.mapped_opening(id) {
				all_mapped.insert(id.clone(), *mapped);
			}
		}
		out.push(rebuilt);
	}
	(out, all_openings, all_mapped)
}

fn nearest_roof_index(opening: &Opening, roofs: &[PitchedRoof]) -> Option<usize> {
	let mid = Vec3::from((opening.bounds.min + opening.bounds.max) * 0.5);
	let mut best: Option<(usize, f32)> = None;
	for (i, roof) in roofs.iter().enumerate() {
		for half in &roof.params().halves {
			let c = pitch_centroid(half.eave_line, half.ridge_line);
			let d = mid.distance_squared(c);
			let replace = match best {
				None => true,
				Some((_, prev)) => d < prev,
			};
			if replace {
				best = Some((i, d));
			}
		}
		for end in 0..2 {
			let drawn = roof.params().halves.iter().any(|h| {
				if end == 0 {
					h.draw_in_half_gable_end.0
				} else {
					h.draw_in_half_gable_end.1
				}
			});
			if !drawn {
				continue;
			}
			let c = gable_centroid(&roof.params().halves, end);
			let d = mid.distance_squared(c);
			let replace = match best {
				None => true,
				Some((_, prev)) => d < prev,
			};
			if replace {
				best = Some((i, d));
			}
		}
	}
	best.map(|(i, _)| i)
}

fn pitch_centroid(eave: (Vec3, Vec3), ridge: (Vec3, Vec3)) -> Vec3 {
	(eave.0 + eave.1 + ridge.0 + ridge.1) * 0.25
}

fn gable_centroid(
	halves: &[crate::shells::pitched_rectangular_roof::RoofHalf; 2],
	end: usize,
) -> Vec3 {
	let e_pos = if end == 0 { halves[0].eave_line.0 } else { halves[0].eave_line.1 };
	let e_neg = if end == 0 { halves[1].eave_line.0 } else { halves[1].eave_line.1 };
	let ridge = if end == 0 { halves[0].ridge_line.0 } else { halves[0].ridge_line.1 };
	(e_pos + e_neg + ridge) / 3.0
}

impl RectangularPitchedRoofComplex {
	/// Author a pitch opening on a resolved roof half (world-space AABB).
	///
	/// `roof` / `half` index the post-decompose roofs. Prefer this over guessing
	/// AABBs against authored massing boxes.
	pub fn pitch_opening(
		&self,
		roof: usize,
		half: usize,
		u: f32,
		v: f32,
		width: f32,
		height: f32,
		label: OpeningLabel,
	) -> Option<Opening> {
		let roof = self.roofs.get(roof)?;
		let half = roof.params().halves.get(half)?;
		Some(PitchedRoof::pitch_opening(half, u, v, width, height, label))
	}

	/// Author a gable-end opening on a resolved roof (both halves at `end`).
	pub fn gable_end_opening(
		&self,
		roof: usize,
		end: usize,
		width: f32,
		height: f32,
		label: OpeningLabel,
	) -> Option<Opening> {
		let roof = self.roofs.get(roof)?;
		Some(PitchedRoof::gable_end_opening(&roof.params().halves, end, width, height, label))
	}
}

impl MapsOpenings for RectangularPitchedRoofComplex {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&crate::openings::MappedOpening> {
		self.mapped.get(id)
	}
}
