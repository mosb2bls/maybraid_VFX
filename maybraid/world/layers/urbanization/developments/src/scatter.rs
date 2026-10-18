//! Deterministic, recipe-driven jittered-grid scattering.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use procedural_common::{Bounds2, SeededHash};

use crate::math::lerp;
use crate::plan::{sample_confines_yaw, yawed_plan_aabb_extent};

#[derive(Debug, Clone)]
pub struct ScatterChoice<K> {
	pub kind: K,
	pub weight: f32,
	pub min_footprint: f32,
	pub max_footprint: f32,
}

#[derive(Debug, Clone)]
pub struct ScatterRecipe<K> {
	pub grid_side: usize,
	pub min_count: usize,
	pub max_count: usize,
	pub cell_inset: f32,
	pub jitter: f32,
	pub clearance: f32,
	pub choices: Vec<ScatterChoice<K>>,
}

#[derive(Debug, Clone)]
pub struct ScatterCandidate<K> {
	pub slot: usize,
	pub center: Vec2,
	pub yaw: f32,
	pub footprint: Vec2,
	pub kind: K,
}

#[derive(Debug, Clone)]
pub struct ScatterPlan<K> {
	pub target_count: usize,
	pub candidates: Vec<ScatterCandidate<K>>,
}

impl<K: Clone> ScatterRecipe<K> {
	pub fn plan(&self, cell: Aabb3d, root: SeededHash) -> ScatterPlan<K> {
		self.plan_in_bounds(
			Aabb2d {
				min: Vec2::new(cell.min.x, cell.min.z),
				max: Vec2::new(cell.max.x, cell.max.z),
			},
			root,
		)
	}

	/// Plan the same deterministic jittered grid inside an arbitrary 2D region.
	///
	/// This is the non-cell counterpart to [`Self::plan`]. Candidate centers stay
	/// within the recipe inset; callers can apply stricter footprint containment.
	pub fn plan_in_bounds(&self, bounds: Aabb2d, root: SeededHash) -> ScatterPlan<K> {
		assert!(self.grid_side > 1, "scatter grid requires at least two rows");
		assert!(!self.choices.is_empty(), "scatter recipe requires a choice");
		assert!(
			(bounds.max - bounds.min).min_element() >= 2.0 * self.cell_inset.max(0.0),
			"scatter bounds must contain the requested inset"
		);

		let count_range = self.max_count.saturating_sub(self.min_count) + 1;
		let target_count = self.min_count + (root.unit(101) * count_range as f32).floor() as usize;
		let mut slots: Vec<(usize, f32)> = (0..self.grid_side * self.grid_side)
			.map(|slot| (slot, root.unit(200 + slot as u32)))
			.collect();
		slots.sort_by(|a, b| a.1.total_cmp(&b.1));

		let candidates = slots
			.into_iter()
			.map(|(slot, _)| {
				let hash = SeededHash::new(
					root.seed.wrapping_add((slot as u32 + 1).wrapping_mul(0x9E37_79B9)),
				);
				let choice = self.pick_choice(hash.unit(1));
				ScatterCandidate {
					slot,
					center: self.candidate_center(bounds, slot, hash),
					yaw: sample_confines_yaw(hash.unit(4)),
					footprint: Vec2::new(
						lerp(choice.min_footprint, choice.max_footprint, hash.unit(2)),
						lerp(choice.min_footprint, choice.max_footprint, hash.unit(3)),
					),
					kind: choice.kind.clone(),
				}
			})
			.collect();
		ScatterPlan { target_count, candidates }
	}

	pub fn collision_bounds(&self, candidate: &ScatterCandidate<K>) -> Bounds2 {
		let half =
			yawed_plan_aabb_extent(candidate.footprint.x, candidate.footprint.y, candidate.yaw)
				* 0.5 + Vec2::splat(self.clearance);
		Bounds2 { min: candidate.center - half, max: candidate.center + half }
	}

	fn candidate_center(&self, bounds: Aabb2d, slot: usize, hash: SeededHash) -> Vec2 {
		let ix = slot % self.grid_side;
		let iz = slot / self.grid_side;
		let min = bounds.min + Vec2::splat(self.cell_inset);
		let max = bounds.max - Vec2::splat(self.cell_inset);
		let denominator = (self.grid_side - 1) as f32;
		let base = Vec2::new(
			lerp(min.x, max.x, ix as f32 / denominator),
			lerp(min.y, max.y, iz as f32 / denominator),
		);
		let jitter = Vec2::new(
			lerp(-self.jitter, self.jitter, hash.unit(5)),
			lerp(-self.jitter, self.jitter, hash.unit(6)),
		);
		(base + jitter).clamp(min, max)
	}

	fn pick_choice(&self, unit: f32) -> &ScatterChoice<K> {
		let total: f32 = self.choices.iter().map(|choice| choice.weight.max(0.0)).sum();
		assert!(total > f32::EPSILON, "scatter recipe requires a positive choice weight");
		let mut selection = unit * total;
		for choice in &self.choices {
			let weight = choice.weight.max(0.0);
			if weight > 0.0 && selection < weight {
				return choice;
			}
			selection -= weight;
		}
		self.choices
			.iter()
			.rev()
			.find(|choice| choice.weight > 0.0)
			.expect("a choice has positive weight")
	}
}

pub fn bounds_intersect(a: Bounds2, b: Bounds2) -> bool {
	a.min.x <= b.max.x && b.min.x <= a.max.x && a.min.y <= b.max.y && b.min.y <= a.max.y
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;

	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	enum Kind {
		Disabled,
		Enabled,
	}

	#[test]
	fn plan_respects_zero_weight_choices_and_count_range() {
		let recipe = ScatterRecipe {
			grid_side: 4,
			min_count: 3,
			max_count: 5,
			cell_inset: 20.0,
			jitter: 4.0,
			clearance: 2.0,
			choices: vec![
				ScatterChoice {
					kind: Kind::Disabled,
					weight: 0.0,
					min_footprint: 1.0,
					max_footprint: 1.0,
				},
				ScatterChoice {
					kind: Kind::Enabled,
					weight: 1.0,
					min_footprint: 4.0,
					max_footprint: 8.0,
				},
			],
		};
		let cell = Aabb3d::new(Vec3::splat(100.0), Vec3::new(100.0, 1.0, 100.0));
		let plan = recipe.plan(cell, SeededHash::new(42));

		assert!((3..=5).contains(&plan.target_count));
		assert_eq!(plan.candidates.len(), 16);
		assert!(plan.candidates.iter().all(|candidate| candidate.kind == Kind::Enabled));
	}

	#[test]
	fn plan_in_bounds_is_deterministic_and_region_local() -> anyhow::Result<()> {
		let recipe = ScatterRecipe {
			grid_side: 3,
			min_count: 4,
			max_count: 6,
			cell_inset: 3.0,
			jitter: 1.5,
			clearance: 0.5,
			choices: vec![ScatterChoice {
				kind: Kind::Enabled,
				weight: 1.0,
				min_footprint: 4.0,
				max_footprint: 6.0,
			}],
		};
		let bounds = Aabb2d { min: Vec2::new(70.0, -40.0), max: Vec2::new(104.0, -8.0) };
		let first = recipe.plan_in_bounds(bounds, SeededHash::new(73));
		let second = recipe.plan_in_bounds(bounds, SeededHash::new(73));
		anyhow::ensure!(first.target_count == second.target_count);
		anyhow::ensure!(first.candidates.len() == second.candidates.len());
		for (a, b) in first.candidates.iter().zip(&second.candidates) {
			anyhow::ensure!(a.slot == b.slot);
			anyhow::ensure!(a.center == b.center);
			anyhow::ensure!(a.yaw == b.yaw);
			anyhow::ensure!(a.footprint == b.footprint);
			anyhow::ensure!(
				a.center.x >= bounds.min.x + recipe.cell_inset
					&& a.center.x <= bounds.max.x - recipe.cell_inset
			);
			anyhow::ensure!(
				a.center.y >= bounds.min.y + recipe.cell_inset
					&& a.center.y <= bounds.max.y - recipe.cell_inset
			);
		}
		Ok(())
	}

	#[test]
	fn plan_delegates_to_matching_xz_bounds() {
		let recipe = ScatterRecipe {
			grid_side: 2,
			min_count: 1,
			max_count: 2,
			cell_inset: 2.0,
			jitter: 1.0,
			clearance: 0.0,
			choices: vec![ScatterChoice {
				kind: Kind::Enabled,
				weight: 1.0,
				min_footprint: 4.0,
				max_footprint: 5.0,
			}],
		};
		let cell =
			Aabb3d::from_min_max(Vec3::new(10.0, -100.0, 30.0), Vec3::new(50.0, 100.0, 80.0));
		let bounds = Aabb2d {
			min: Vec2::new(cell.min.x, cell.min.z),
			max: Vec2::new(cell.max.x, cell.max.z),
		};
		let from_cell = recipe.plan(cell, SeededHash::new(9));
		let from_bounds = recipe.plan_in_bounds(bounds, SeededHash::new(9));
		assert_eq!(from_cell.target_count, from_bounds.target_count);
		assert!(from_cell.candidates.iter().zip(from_bounds.candidates).all(|(a, b)| a.slot
			== b.slot
			&& a.center == b.center
			&& a.yaw == b.yaw
			&& a.footprint == b.footprint));
	}
}
