//! Mixed-use Les Halles monotower: commercial gallery storeys below, livable above.
//!
//! One shared [`LesHallesParameterized`] shell is replayed per storey. Shaft slots
//! merge inbound [`OpeningLabel::Shaft`] openings with a sampled 1…4 complement.
//! Stairs and roof are deferred to the tower consumer — shafts remain
//! [`SpaceKind::InternalSpace`] residuals.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::furniture::FurnitureNode;
use building_components::joints::JointNode;
use building_components::labels::LabelNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::openings::{OpeningLabel, Openings};
use crate::storeys::les_halles::parameterized::{
	footprint_extents, MAX_MONOTOWER_STOREY_HEIGHT, MIN_MONOTOWER_STOREY_HEIGHT,
};
use crate::storeys::les_halles::{
	LesHallesArcadeUsage, LesHallesCommercialUsage, LesHallesFloorPlan, LesHallesLivableUsage,
	LesHallesOpeningProgram, LesHallesParameterized, LesHallesUsagePlan,
};

const SALT_FLOOR: f32 = 11.0;

/// One storey in a [`MixedUseLesHallesMonotower`]: shared shell + painted usage.
#[derive(Debug, Clone, PartialEq)]
pub enum MixedUseLesHallesStorey {
	Arcade {
		floor_plan: LesHallesFloorPlan,
		usage: LesHallesArcadeUsage,
		wall_material: Option<MaterialRef>,
	},
	Commercial {
		floor_plan: LesHallesFloorPlan,
		usage: LesHallesCommercialUsage,
		wall_material: Option<MaterialRef>,
	},
	Livable {
		floor_plan: LesHallesFloorPlan,
		usage: LesHallesLivableUsage,
		wall_material: Option<MaterialRef>,
	},
}

impl MixedUseLesHallesStorey {
	pub fn floor_plan(&self) -> &LesHallesFloorPlan {
		match self {
			Self::Arcade { floor_plan, .. }
			| Self::Commercial { floor_plan, .. }
			| Self::Livable { floor_plan, .. } => floor_plan,
		}
	}

	pub fn floor_plan_mut(&mut self) -> &mut LesHallesFloorPlan {
		match self {
			Self::Arcade { floor_plan, .. }
			| Self::Commercial { floor_plan, .. }
			| Self::Livable { floor_plan, .. } => floor_plan,
		}
	}

	pub fn is_arcade(&self) -> bool {
		matches!(self, Self::Arcade { .. })
	}

	pub fn is_commercial(&self) -> bool {
		matches!(self, Self::Commercial { .. })
	}

	pub fn wall_material(&self) -> Option<&MaterialRef> {
		match self {
			Self::Arcade { wall_material, .. }
			| Self::Commercial { wall_material, .. }
			| Self::Livable { wall_material, .. } => wall_material.as_ref(),
		}
	}

	/// Stamp a wall shader look onto every emitted panel (kit style unchanged).
	pub fn with_wall_material(mut self, material: MaterialRef) -> Self {
		match &mut self {
			Self::Arcade { wall_material, .. }
			| Self::Commercial { wall_material, .. }
			| Self::Livable { wall_material, .. } => {
				*wall_material = Some(material);
			}
		}
		self
	}
}

impl BuildingComponents for MixedUseLesHallesStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.floor_plan().panel_nodes_for_level(level);
		match self {
			Self::Arcade { .. } => {}
			Self::Commercial { usage, .. } => {
				out.extend(usage.panel_nodes_for_level(level));
			}
			Self::Livable { usage, .. } => {
				out.extend(usage.panel_nodes_for_level(level));
			}
		}
		if let Some(material) = self.wall_material() {
			out = out.with_material(material.clone());
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.floor_plan().joint_nodes_for_level(level);
		if let Self::Livable { usage, .. } = self {
			out.extend(usage.joint_nodes_for_level(level));
		}
		out
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		match self {
			Self::Livable { usage, .. } => usage.furniture_nodes_for_level(level),
			Self::Commercial { usage, .. } => usage.furniture_nodes_for_level(level),
			Self::Arcade { usage, .. } => usage.furniture_nodes_for_level(level),
		}
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		match self {
			Self::Livable { usage, .. } => usage.furniture_usage_nodes_for_level(level),
			Self::Commercial { usage, .. } => usage.furniture_usage_nodes_for_level(level),
			Self::Arcade { usage, .. } => usage.furniture_usage_nodes_for_level(level),
		}
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		match self {
			Self::Commercial { usage, .. } => usage.label_nodes_for_level(level),
			Self::Livable { usage, .. } => usage.label_nodes_for_level(level),
			Self::Arcade { usage, .. } => usage.label_nodes_for_level(level),
		}
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		Some(self.floor_plan().structural_probe(self.wall_material().cloned()))
	}
}

/// Commercial-below / livable-above stack of Les Halles storeys.
#[derive(Debug, Clone, PartialEq)]
pub struct MixedUseLesHallesMonotower {
	pub parameterized: LesHallesParameterized,
	pub storey_height: f32,
	/// Count of commercial storeys above the ground arcade (`floors[1..1+n_commercial]`).
	pub n_commercial: usize,
	/// Active shaft placement slots (`0…3`), frozen for the tower.
	pub shaft_slots: Vec<usize>,
	pub floors: Vec<MixedUseLesHallesStorey>,
}

impl MixedUseLesHallesMonotower {
	pub fn floor_count(&self) -> usize {
		self.floors.len()
	}

	/// Fit a mixed-use stack whose shell knobs come from `sample_shell`.
	pub fn fit_sampled(
		confines: &Confines,
		noise: NoiseParams,
		sample_shell: fn(&Confines, NoiseParams) -> Result<LesHallesParameterized, FitError>,
	) -> Result<(Self, FillableRegions), FitError> {
		let (_, _, total_h) = footprint_extents(confines)?;
		let n_storeys = LesHallesParameterized::sample_monotower_storey_count(confines, noise);
		let max_h_for_n = (total_h / n_storeys as f32).max(MIN_MONOTOWER_STOREY_HEIGHT);
		let storey_height = LesHallesParameterized::sample_monotower_storey_height(confines, noise)
			.clamp(MIN_MONOTOWER_STOREY_HEIGHT, max_h_for_n.min(MAX_MONOTOWER_STOREY_HEIGHT));
		let used_h = n_storeys as f32 * storey_height;
		if used_h + 1e-3 < MIN_MONOTOWER_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}

		// Shell sampling uses a representative single-storey slice.
		let y0 = confines.bounds.min.y;
		let shell_confines = slice_confines(confines, y0, storey_height, Openings::new());
		let params = sample_shell(&shell_confines, noise)?;

		let shaft_slots = resolve_shaft_slots(&params, confines, noise);
		let upper_n = n_storeys.saturating_sub(1);
		let n_commercial = if upper_n == 0 {
			0
		} else {
			LesHallesParameterized::sample_monotower_commercial_count(confines, noise, upper_n)
		};

		let mut floors = Vec::with_capacity(n_storeys);
		for i in 0..n_storeys {
			let fy0 = y0 + i as f32 * storey_height;
			let storey_openings =
				LesHallesFloorPlan::shaft_requests_for_slots(&params, confines, &shaft_slots);
			// Re-home requests onto this storey's Y band.
			let storey_openings = rebase_openings_y(storey_openings, fy0, storey_height);
			let inbound = rebase_openings_y(inbound_shaft_openings(confines), fy0, storey_height);
			let mut openings = storey_openings;
			for (id, opening) in inbound.iter() {
				openings.insert(id.clone(), opening.clone());
			}
			let storey_confines = slice_confines(confines, fy0, storey_height, openings);
			let program = if i == 0 {
				LesHallesOpeningProgram::GroundArcade
			} else {
				LesHallesOpeningProgram::CommercialGallery
			};
			let (floor_plan, regions) = LesHallesFloorPlan::from_parameterized_with(
				params.clone(),
				&storey_confines,
				crate::shells::rect_ring_floor::RectRingFloorSlab::None,
				program,
			)?;
			let floor_noise = floor_noise(noise, i);
			let storey = if i == 0 {
				let keep_outs = LesHallesArcadeUsage::pillar_keep_outs_of(&floor_plan);
				let (usage, _) =
					LesHallesArcadeUsage::paint_avoiding(regions, floor_noise, &keep_outs)?;
				MixedUseLesHallesStorey::Arcade { floor_plan, usage, wall_material: None }
			} else if i <= n_commercial {
				let (usage, _) = LesHallesCommercialUsage::paint(regions, floor_noise)?;
				MixedUseLesHallesStorey::Commercial { floor_plan, usage, wall_material: None }
			} else {
				let (usage, _) = LesHallesLivableUsage::paint(regions, floor_noise)?;
				MixedUseLesHallesStorey::Livable { floor_plan, usage, wall_material: None }
			};
			floors.push(storey);
		}

		let tower =
			Self { parameterized: params, storey_height, n_commercial, shaft_slots, floors };
		// Residuals: stack footprint atop the used height (consumer authors roof).
		let residual = FillableRegions {
			within: Vec::new(),
			atop: tower
				.floors
				.first()
				.map(|f| f.floor_plan().fillable_regions().atop)
				.unwrap_or_default(),
		};
		let _ = used_h;
		Ok((tower, residual))
	}

	/// Fit with [`LesHallesParameterized::sample_curtain`] gallery depths.
	pub fn fit_curtain(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::fit_sampled(confines, noise, LesHallesParameterized::sample_curtain)
	}
}

impl Fit for MixedUseLesHallesMonotower {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::fit_sampled(confines, noise, LesHallesParameterized::sample_monotower)
	}
}

impl BuildingComponents for MixedUseLesHallesMonotower {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for floor in &self.floors {
			out.extend(floor.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for floor in &self.floors {
			out.extend(floor.joint_nodes_for_level(level));
		}
		out
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		for floor in &self.floors {
			out.extend(floor.furniture_nodes_for_level(level));
		}
		out
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		let mut out = Layers::new();
		for floor in &self.floors {
			out.extend(floor.furniture_usage_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for floor in &self.floors {
			out.extend(floor.label_nodes_for_level(level));
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		self.floors
			.iter()
			.filter_map(BuildingComponents::structural_lod)
			.reduce(|a, b| a.merge(b))
	}
}

fn slice_confines(base: &Confines, y0: f32, height: f32, openings: Openings) -> Confines {
	let min = Vec3::from(base.bounds.min);
	let max = Vec3::from(base.bounds.max);
	let bounds =
		Aabb3d::from_min_max(Vec3::new(min.x, y0, min.z), Vec3::new(max.x, y0 + height, max.z));
	Confines::new(bounds, base.roll, openings)
}

fn inbound_shaft_openings(confines: &Confines) -> Openings {
	let mut out = Openings::new();
	for (id, opening) in confines.openings.iter() {
		if matches!(opening.label, OpeningLabel::Shaft) {
			out.insert(id.clone(), opening.clone());
		}
	}
	out
}

fn rebase_openings_y(openings: Openings, y0: f32, height: f32) -> Openings {
	let mut out = Openings::new();
	for (id, mut opening) in openings.openings.into_iter() {
		let omin = Vec3::from(opening.bounds.min);
		let omax = Vec3::from(opening.bounds.max);
		opening.bounds = Aabb3d::from_min_max(
			Vec3::new(omin.x, y0, omin.z),
			Vec3::new(omax.x, y0 + height.min((omax.y - omin.y).max(1.0)), omax.z),
		);
		out.insert(id, opening);
	}
	out
}

fn resolve_shaft_slots(
	params: &LesHallesParameterized,
	confines: &Confines,
	noise: NoiseParams,
) -> Vec<usize> {
	let y0 = confines.bounds.min.y;
	let h = LesHallesParameterized::sample_monotower_storey_height(confines, noise)
		.max(MIN_MONOTOWER_STOREY_HEIGHT);

	// Map only inbound shafts first so authored slots are preserved.
	let mut slots = Vec::new();
	let inbound = inbound_shaft_openings(confines);
	if !inbound.openings.is_empty() {
		let probe = slice_confines(confines, y0, h, rebase_openings_y(inbound, y0, h));
		if let Ok((plan, _)) = LesHallesFloorPlan::from_parameterized(params.clone(), &probe) {
			slots = plan.shaft_slots;
		}
	}

	let want = LesHallesParameterized::sample_monotower_shaft_count(confines, noise)
		.max(slots.len())
		.clamp(1, 4);
	if slots.len() < want {
		for s in LesHallesParameterized::sample_monotower_shaft_slots(confines, noise, want) {
			if !slots.contains(&s) {
				slots.push(s);
			}
			if slots.len() >= want {
				break;
			}
		}
	}
	if slots.is_empty() {
		slots = LesHallesParameterized::sample_monotower_shaft_slots(confines, noise, want);
	}
	slots.sort_unstable();
	slots.dedup();
	slots
}

fn floor_noise(noise: NoiseParams, floor_i: usize) -> NoiseParams {
	let mut n = noise;
	n.seed = noise.seed.wrapping_add(floor_i as i32 * 97);
	// Touch salt lane so spatial samples diverge even with same seed math.
	let _ = NoiseConfig::new(n).sample_unit_4d(0.0, floor_i as f32, 0.0, SALT_FLOOR);
	n
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	use crate::storeys::les_halles::{MAX_MONOTOWER_STOREYS, MIN_MONOTOWER_STOREYS};

	fn large_tower_bounds() -> Aabb3d {
		// ~72×54 footprint, ~16 m tall → several 3–5 m storeys.
		Aabb3d::from_min_max(Vec3::new(-36.0, 0.0, -27.0), Vec3::new(36.0, 16.0, 27.0))
	}

	#[test]
	fn mixed_use_stacks_arcade_then_commercial_then_livable() {
		let confines = Confines::from_bounds(large_tower_bounds());
		let noise = NoiseParams { seed: 42, ..NoiseParams::default() };
		let (tower, _) = MixedUseLesHallesMonotower::fit_to_confines(&confines, noise).unwrap();
		assert!(tower.floor_count() >= MIN_MONOTOWER_STOREYS);
		assert!(tower.floor_count() <= MAX_MONOTOWER_STOREYS);
		assert!(tower.floors[0].is_arcade());
		assert!(!tower.shaft_slots.is_empty());
		assert!(tower.shaft_slots.len() <= 4);
		for (i, floor) in tower.floors.iter().enumerate() {
			if i == 0 {
				assert!(floor.is_arcade(), "floor 0 should be arcade");
			} else if i <= tower.n_commercial {
				assert!(floor.is_commercial(), "floor {i} should be commercial");
			} else {
				assert!(
					!floor.is_commercial() && !floor.is_arcade(),
					"floor {i} should be livable"
				);
			}
			assert!(
				(floor.floor_plan().parameterized.gallery_width
					- tower.parameterized.gallery_width)
					.abs() < 1e-4,
				"shared shell gallery depth"
			);
		}
		assert!(tower.floors[0]
			.floor_plan()
			.openings
			.iter()
			.any(|(id, _)| { id.as_str().contains("outer_breezeway") }));
	}

	#[test]
	fn inbound_shafts_are_preserved_in_slots() {
		let bounds = large_tower_bounds();
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams { seed: 7, ..NoiseParams::default() };
		let params = LesHallesParameterized::sample_monotower(&empty, noise).unwrap();
		// Seed two requests; remap onto a storey-height probe to learn the slots
		// the floor plan will claim for those AABBs under this placement.
		let mut openings = LesHallesFloorPlan::shaft_requests_for_slots(&params, &empty, &[1, 3]);
		let h = 4.0_f32;
		openings = rebase_openings_y(openings, bounds.min.y, h);
		let probe = Confines::new(
			Aabb3d::from_min_max(
				Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
				Vec3::new(bounds.max.x, bounds.min.y + h, bounds.max.z),
			),
			0.0,
			openings.clone(),
		);
		let (probe_plan, _) =
			LesHallesFloorPlan::from_parameterized(params.clone(), &probe).unwrap();
		assert_eq!(probe_plan.shaft_slots.len(), 2);
		let expected = probe_plan.shaft_slots.clone();

		// Full-height inbound for the tower fit.
		let tower_openings = rebase_openings_y(openings, bounds.min.y, bounds.max.y - bounds.min.y);
		let confines = Confines::new(bounds, 0.0, tower_openings);
		let (tower, _) = MixedUseLesHallesMonotower::fit_to_confines(&confines, noise).unwrap();
		for slot in &expected {
			assert!(
				tower.shaft_slots.contains(slot),
				"expected inbound slot {slot} in {:?}",
				tower.shaft_slots
			);
		}
	}

	#[test]
	fn filled_floors_emit_furniture() {
		let confines = Confines::from_bounds(large_tower_bounds());
		for seed in [7, 42, 1337] {
			let noise = NoiseParams { seed, ..NoiseParams::default() };
			let (tower, _) = MixedUseLesHallesMonotower::fit_to_confines(&confines, noise).unwrap();
			for (i, floor) in tower.floors.iter().enumerate() {
				if floor.is_arcade() {
					continue;
				}
				let slots = floor.furniture_nodes_for_level(LodSceneLevel::High).flatten();
				if floor.is_commercial() {
					continue;
				}
				assert!(!slots.is_empty(), "seed {seed} livable floor {i} should emit furniture");
			}
		}
	}

	#[test]
	fn tall_confines_stay_within_two_to_seven_storeys() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-36.0, 0.0, -27.0),
			Vec3::new(36.0, 35.0, 27.0),
		));
		for seed in [1, 7, 42, 99, 1337] {
			let noise = NoiseParams { seed, ..NoiseParams::default() };
			let (tower, _) = MixedUseLesHallesMonotower::fit_to_confines(&confines, noise).unwrap();
			assert!(
				(MIN_MONOTOWER_STOREYS..=MAX_MONOTOWER_STOREYS).contains(&tower.floor_count()),
				"seed {seed} floors={}",
				tower.floor_count()
			);
			assert!(tower.floors[0].is_arcade());
		}
	}
}
