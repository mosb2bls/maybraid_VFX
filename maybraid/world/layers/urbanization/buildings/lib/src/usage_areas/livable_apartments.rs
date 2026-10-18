//! Residential fill over hall-connected enclosed suites.
//!
//! Pipeline: [`HallEnclosedSuites`] (halls → groups → doors/walls) →
//! [`LivableApartment`] per suite (no per-cell shells — avoids double-walling).

use bevy_math::Vec3;
use building_components::joints::JointNode;
use building_components::labels::LabelNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{aabb_xz_extent, Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::usage_areas::hall_connected_suites::{
	HallEnclosedSuites, HallSuiteEncloseParams, HallSuitePackParams,
};
use crate::usage_areas::halls_to_shafts::HallsToShafts;
use crate::usage_areas::livable_apartment::{LivableApartment, INTERNAL_WALLS_LAYER};
use crate::usage_areas::plan_cells::MIN_GROUP_CONNECTIVITY;
use crate::usage_areas::plan_geom::{host_xz, noise_for_cell};

const EPS: f32 = 1e-3;
const MIN_ROOM: f32 = 2.5;
const SCOPE: &str = "livable_apartments";

/// Noise knobs for [`LivableApartments`] (target-area catalog + hall width).
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartmentsParameterized {
	/// Corridor clear width for halls (`None` ⇒ sample inside HTS).
	pub hall_width: Option<f32>,
	/// Target apartment areas in m² (catalog order; large → small preferred).
	pub targets: Vec<f32>,
}

impl LivableApartmentsParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let fp = aabb_xz_extent(&confines.bounds);
		if fp.x + EPS < MIN_ROOM || fp.y + EPS < MIN_ROOM {
			return Err(FitError::TooSmall { reason: "livable_apartments_host" });
		}
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		Ok(Self { hall_width: None, targets: generate_apartment_targets(&cfg, c) })
	}

	pub fn with_hall_width(mut self, hall_width: Option<f32>) -> Self {
		self.hall_width = hall_width;
		self
	}
}

/// Options for [`LivableApartments::from_confines_with`].
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartmentsOptions {
	pub hall_width: Option<f32>,
	pub targets: Option<Vec<f32>>,
}

impl Default for LivableApartmentsOptions {
	fn default() -> Self {
		Self { hall_width: None, targets: None }
	}
}

/// Hall carve + enclosed suites filled with [`LivableApartment`].
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartments {
	pub confines: Confines,
	pub parameterized: LivableApartmentsParameterized,
	pub halls: HallsToShafts,
	pub apartments: Vec<LivableApartment>,
	pub walls: Vec<ClippedRectangularStrip>,
	pub hall_width: f32,
}

impl LivableApartments {
	pub fn from_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines_with(confines, noise, LivableApartmentsOptions::default())
	}

	pub fn from_confines_with(
		confines: &Confines,
		noise: NoiseParams,
		options: LivableApartmentsOptions,
	) -> Result<(Self, FillableRegions), FitError> {
		let mut params = LivableApartmentsParameterized::sample(confines, noise)?;
		params.hall_width = options.hall_width.or(params.hall_width);
		if let Some(targets) = options.targets {
			if !targets.is_empty() {
				params.targets = targets;
			}
		}
		Self::from_parameterized(params, confines, noise)
	}

	pub fn from_parameterized(
		params: LivableApartmentsParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		if params.targets.is_empty() {
			return Err(FitError::InvalidConfines { reason: "livable_apartments_empty_targets" });
		}

		let enclosed = HallEnclosedSuites::from_confines(
			confines,
			noise,
			HallSuitePackParams {
				hall_width: params.hall_width,
				targets: params.targets.clone(),
				min_room: MIN_ROOM,
				min_connectivity: MIN_GROUP_CONNECTIVITY,
			},
			HallSuiteEncloseParams { scope: SCOPE, ..Default::default() },
		)?;

		// No residual rooms → try the whole host as one apartment.
		if enclosed.suites.is_empty() {
			return singleton_host(confines, params, enclosed, noise);
		}

		let HallEnclosedSuites {
			confines: host,
			halls,
			hall_width,
			suites,
			walls,
			mut residual_within,
		} = enclosed;

		let mut apartments = Vec::new();
		let mut apt_id = 0u32;
		for multi in suites {
			// Per-suite seed — same parent noise + nearby centers otherwise
			// correlate program_from_area / RLA choices across the flange.
			let apt_noise = noise_for_cell(noise, apt_id as i32);
			match LivableApartment::from_multi(apt_id, &multi, apt_noise) {
				Ok((mut apt, nested)) => {
					apt.shell = None;
					apt_id = apt_id.saturating_add(1);
					apartments.push(apt);
					residual_within.extend(nested.within);
				}
				Err(FitError::TooSmall { .. }) => {
					residual_within.extend(multi.parts);
				}
				Err(err) => return Err(err),
			}
		}

		Ok((
			Self { confines: host, parameterized: params, halls, apartments, walls, hall_width },
			FillableRegions { within: residual_within, atop: Vec::new() },
		))
	}
}

impl Fit for LivableApartments {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines(confines, noise)
	}
}

impl BuildingComponents for LivableApartments {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.walls {
			out.extend_under(INTERNAL_WALLS_LAYER, wall.panel_nodes_for_level(level));
		}
		for apt in &self.apartments {
			out.extend(apt.panel_nodes_for_level(level));
		}
		structural_layers(level, out)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for wall in &self.walls {
			out.extend_under(INTERNAL_WALLS_LAYER, wall.joint_nodes_for_level(level));
		}
		for apt in &self.apartments {
			out.extend(apt.joint_nodes_for_level(level));
		}
		structural_layers(level, out)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for apt in &self.apartments {
			out.extend(apt.label_nodes_for_level(level));
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		// Host footprint, then merge nested apartment cells so parents can compose
		// multi-block perimeters (e.g. IApartmentFullStorey).
		let mut probe = BuildingStructuralLodProbe::new([host_xz(&self.confines.bounds)]);
		for apt in &self.apartments {
			if let Some(nested) = apt.structural_lod() {
				probe = probe.merge(nested);
			}
		}
		Some(probe)
	}
}

/// High keeps suite-divider walls; coarser bands drop [`INTERNAL_WALLS_LAYER`].
/// Nested apartments already apply the same filter on their own internals.
fn structural_layers<T>(level: LodSceneLevel, layers: Layers<T>) -> Layers<T> {
	if matches!(level, LodSceneLevel::High) {
		layers
	} else {
		layers.except([INTERNAL_WALLS_LAYER])
	}
}

/// Noise-perturbed apartment target-area catalog (m²), large → small.
fn generate_apartment_targets(cfg: &NoiseConfig, center: Vec3) -> Vec<f32> {
	const BASES: &[f32] = &[55.0, 48.0, 40.0, 35.0, 30.0, 26.0, 22.0, 18.0];
	BASES
		.iter()
		.enumerate()
		.map(|(i, &base)| {
			cfg.sample_range_f32_4d(
				(base - 4.0).max(14.0),
				base + 5.0,
				center.x,
				center.y,
				center.z,
				80.0 + i as f32,
			)
		})
		.collect()
}

fn singleton_host(
	confines: &Confines,
	params: LivableApartmentsParameterized,
	enclosed: HallEnclosedSuites,
	noise: NoiseParams,
) -> Result<(LivableApartments, FillableRegions), FitError> {
	let HallEnclosedSuites { halls, hall_width, mut residual_within, .. } = enclosed;
	match LivableApartment::from_confines(0, confines, noise) {
		Ok((mut apt, nested)) => {
			apt.shell = None;
			residual_within.extend(nested.within);
			Ok((
				LivableApartments {
					confines: confines.clone(),
					parameterized: params,
					halls,
					apartments: vec![apt],
					walls: Vec::new(),
					hall_width,
				},
				FillableRegions { within: residual_within, atop: Vec::new() },
			))
		}
		Err(FitError::TooSmall { .. }) => {
			residual_within.push(FillRegion::new(SpaceKind::InternalSpace, confines.clone()));
			Ok((
				LivableApartments {
					confines: confines.clone(),
					parameterized: params,
					halls,
					apartments: Vec::new(),
					walls: Vec::new(),
					hall_width,
				},
				FillableRegions { within: residual_within, atop: Vec::new() },
			))
		}
		Err(err) => Err(err),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
	use bevy_math::bounding::Aabb3d;
	use building_components::Layer;

	fn host_with_shafts_and_passage() -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("s0"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(-6.0, 0.0, -1.0), Vec3::new(-4.0, 3.0, 1.0)),
				OpeningLabel::Shaft,
			),
		);
		openings.insert(
			OpeningId::new("s1"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(4.0, 0.0, -1.0), Vec3::new(6.0, 3.0, 1.0)),
				OpeningLabel::Shaft,
			),
		);
		openings.insert(
			OpeningId::new("p0"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(-0.6, 0.0, 7.7), Vec3::new(0.6, 2.2, 8.1)),
				OpeningLabel::Passage,
			),
		);
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(-12.0, 0.0, -8.0), Vec3::new(12.0, 3.5, 8.0)),
			0.0,
			openings,
		)
	}

	#[test]
	fn packs_halls_and_apartments() {
		let confines = host_with_shafts_and_passage();
		let (block, regions) = LivableApartments::from_confines_with(
			&confines,
			NoiseParams::default(),
			LivableApartmentsOptions {
				hall_width: Some(2.5),
				targets: Some(vec![40.0, 30.0, 22.0, 18.0]),
			},
		)
		.unwrap();
		assert!(!block.halls.hall_bands.is_empty());
		assert!(!block.apartments.is_empty());
		assert!(block.apartments.iter().all(|a| a.shell.is_none()));
		let door_count = block
			.apartments
			.iter()
			.flat_map(|a| a.cells.iter())
			.flat_map(|p| p.confines.openings.iter())
			.filter(|(id, _)| id.as_str().contains("hall_door"))
			.count();
		assert!(door_count >= 1, "expected hall doors on groups");
		assert!(
			door_count <= block.apartments.len(),
			"expected ≤1 door per apartment, doors={door_count} apts={}",
			block.apartments.len()
		);
		let _ = regions;
	}

	#[test]
	fn packs_some_multi_cell_apartments() {
		let confines = host_with_shafts_and_passage();
		let (block, _) = LivableApartments::from_confines_with(
			&confines,
			NoiseParams { seed: 3, ..NoiseParams::default() },
			LivableApartmentsOptions {
				hall_width: Some(2.5),
				targets: Some(vec![55.0, 48.0, 40.0, 30.0]),
			},
		)
		.unwrap();
		assert!(
			block.apartments.iter().any(|a| a.cells.len() >= 2),
			"expected at least one non-rectangular / multi-cell group"
		);
	}

	#[test]
	fn suite_divider_walls_only_on_high_structural_band() {
		let confines = host_with_shafts_and_passage();
		let (block, _) = LivableApartments::from_confines_with(
			&confines,
			NoiseParams::default(),
			LivableApartmentsOptions {
				hall_width: Some(2.5),
				targets: Some(vec![40.0, 30.0, 22.0, 18.0]),
			},
		)
		.unwrap();
		assert!(!block.walls.is_empty(), "fixture needs suite-divider walls to exercise LOD");
		let high = block.panel_nodes_for_level(LodSceneLevel::High);
		assert!(
			high.labeled.contains_key(&Layer::new(INTERNAL_WALLS_LAYER)),
			"High should tag suite / apartment internal walls"
		);
		let medium = block.panel_nodes_for_level(LodSceneLevel::Medium);
		assert!(
			!medium.labeled.contains_key(&Layer::new(INTERNAL_WALLS_LAYER)),
			"Medium should drop suite-divider and nested internal walls"
		);
		assert!(medium.len() < high.len(), "Medium should emit fewer panels than High");
	}
}
