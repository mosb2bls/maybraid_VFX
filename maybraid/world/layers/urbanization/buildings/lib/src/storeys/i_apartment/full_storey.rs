//! I-Apartment full storey: floor plan → [`LivableApartments`] per primary rect.
//!
//! Primary rects are filled **progressively**: each block may wall shared
//! interfaces with later siblings, then injects [`OpeningLabel::Boundary`] onto
//! those siblings so they do not double-wall.

use building_components::joints::JointNode;
use building_components::labels::LabelNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers, MassingVolume};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::usage_areas::boundary_openings::inject_shared_boundary_from;
use crate::usage_areas::plan_geom::{host_xz, noise_for_cell};
use crate::usage_areas::{LivableApartments, LivableApartmentsOptions};

use super::floor_plan::IApartmentFloorPlan;
use super::SCOPE;

/// Full I-Apartment storey: I-frame shell + livable apartment blocks per primary rect.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentFullStorey {
	pub floor_plan: IApartmentFloorPlan,
	/// One [`LivableApartments`] pack per primary rectangular residual.
	pub blocks: Vec<LivableApartments>,
}

impl IApartmentFullStorey {
	/// Wrap an already-fitted floor plan and allocate livable apartment blocks.
	pub fn from_floor_plan(
		floor_plan: IApartmentFloorPlan,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let regions = floor_plan.fillable_regions();
		Self::fill_from_plan(floor_plan, regions, noise)
	}

	fn fill_from_plan(
		floor_plan: IApartmentFloorPlan,
		regions: FillableRegions,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let mut blocks = Vec::new();
		let mut residual_within = Vec::new();
		let opts =
			LivableApartmentsOptions { hall_width: Some(floor_plan.hall_width), targets: None };

		// Progressive fill: each primary rect may claim shared edges, then marks
		// later siblings with Boundary so they skip those faces.
		let mut pending: Vec<FillRegion> = regions.within;
		for i in 0..pending.len() {
			// Match Les Halles: diversify seed per primary rect so stem/flange
			// packs and room programs do not clone each other.
			let block_noise = noise_for_cell(noise, i as i32);
			match LivableApartments::from_confines_with(
				&pending[i].confines,
				block_noise,
				opts.clone(),
			) {
				Ok((block, nested)) => {
					let owner = host_xz(&pending[i].confines.bounds);
					for j in (i + 1)..pending.len() {
						inject_shared_boundary_from(
							owner,
							&mut pending[j].confines,
							SCOPE,
							format!("prog_{i}_{j}"),
						);
					}
					blocks.push(block);
					residual_within.extend(nested.within.into_iter().map(as_closet_if_internal));
				}
				Err(FitError::TooSmall { .. }) => {
					residual_within
						.push(FillRegion::new(SpaceKind::ClosetSpace, pending[i].confines.clone()));
				}
				Err(err) => return Err(err),
			}
		}

		Ok((
			Self { floor_plan, blocks },
			FillableRegions { within: residual_within, atop: regions.atop },
		))
	}
}

fn as_closet_if_internal(region: FillRegion) -> FillRegion {
	match region.kind {
		SpaceKind::InternalSpace => FillRegion::new(SpaceKind::ClosetSpace, region.confines),
		_ => region,
	}
}

impl Fit for IApartmentFullStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (floor_plan, regions) = IApartmentFloorPlan::fit_to_confines(confines, noise)?;
		Self::fill_from_plan(floor_plan, regions, noise)
	}
}

impl BuildingComponents for IApartmentFullStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.floor_plan.panel_nodes_for_level(level);
		for block in &self.blocks {
			out.extend(block.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.floor_plan.joint_nodes_for_level(level);
		for block in &self.blocks {
			out.extend(block.joint_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = self.floor_plan.label_nodes_for_level(level);
		for block in &self.blocks {
			out.extend(block.label_nodes_for_level(level));
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let y0 = self.floor_plan.center_xz.y;
		let height = self.floor_plan.storey_height;
		let volumes: Vec<_> = self
			.floor_plan
			.primary_rects
			.iter()
			.map(|rect| MassingVolume::cuboid(rect.to_aabb2(), y0, height))
			.collect();
		if volumes.is_empty() {
			None
		} else {
			Some(BuildingStructuralLodProbe::from_volumes(volumes))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::OpeningLabel;
	use crate::storeys::i_apartment::{IApartmentFloorPlan, IApartmentParameterized};
	use crate::usage_areas::plan_cells::{hall_frontage_length, PlanCell, MIN_GROUP_CONNECTIVITY};
	use crate::usage_areas::plan_geom::host_xz;
	use bevy::prelude::Transform;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use building_components::{
		STRUCTURAL_HIGH_OUTSIDE_METERS, STRUCTURAL_LOW_OUTSIDE_METERS,
		STRUCTURAL_MEDIUM_OUTSIDE_METERS,
	};

	fn storey_seed(seed: i32) -> IApartmentFullStorey {
		let bounds = Aabb3d::from_min_max(Vec3::new(-22.0, 0.0, -18.0), Vec3::new(22.0, 3.5, 18.0));
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		let params = IApartmentParameterized::sample(&empty, noise).unwrap();
		let inbound = IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty);
		let confines = Confines::new(bounds, 0.0, inbound);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		IApartmentFullStorey::from_floor_plan(plan, noise).unwrap().0
	}

	#[test]
	fn full_storey_allocates_block_per_rect() {
		let bounds = Aabb3d::from_min_max(Vec3::new(-22.0, 0.0, -18.0), Vec3::new(22.0, 3.5, 18.0));
		let confines = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = IApartmentParameterized::sample(&confines, noise).unwrap();
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		let n = plan.primary_rects.len();
		let (storey, _) = IApartmentFullStorey::from_floor_plan(plan, noise).unwrap();
		assert_eq!(storey.blocks.len(), n);
		assert!(!storey.panel_nodes_for_level(LodSceneLevel::High).is_empty());
	}

	#[test]
	fn structural_probe_high_within_perimeter_cutoff() {
		let storey = storey_seed(0);
		let probe = storey.structural_lod().expect("composed LivableApartments footprints");
		assert!(!probe.footprints().is_empty());
		assert_eq!(probe.high_outside_meters, STRUCTURAL_HIGH_OUTSIDE_METERS);

		let inside = Transform::from_xyz(0.0, 1.5, 0.0);
		assert_eq!(probe.level_for(&inside), LodSceneLevel::High);

		// Just past High → Medium (exterior kits, no interior walls).
		let mid = Transform::from_xyz(STRUCTURAL_HIGH_OUTSIDE_METERS + 40.0, 1.5, 0.0);
		assert!(probe.distance_outside(&mid) > STRUCTURAL_HIGH_OUTSIDE_METERS);
		assert!(probe.distance_outside(&mid) <= STRUCTURAL_MEDIUM_OUTSIDE_METERS);
		assert_eq!(probe.level_for(&mid), LodSceneLevel::Medium);

		// Far beyond Low → UltraLow massing.
		let far = Transform::from_xyz(500.0, 1.5, 500.0);
		assert!(probe.distance_outside(&far) > STRUCTURAL_LOW_OUTSIDE_METERS);
		assert_eq!(probe.level_for(&far), LodSceneLevel::UltraLow);

		let high_n = storey.panel_nodes_for_level(LodSceneLevel::High).len();
		let mid_n = storey.panel_nodes_for_level(LodSceneLevel::Medium).len();
		assert!(mid_n < high_n, "Medium should drop internal apartment walls");
	}

	#[test]
	fn full_storey_blocks_contain_apartments_when_connected() {
		let bounds = Aabb3d::from_min_max(Vec3::new(-22.0, 0.0, -18.0), Vec3::new(22.0, 3.5, 18.0));
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = IApartmentParameterized::sample(&empty, noise).unwrap();
		let inbound = IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty);
		let confines = Confines::new(bounds, 0.0, inbound);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		let (storey, _) = IApartmentFullStorey::from_floor_plan(plan, noise).unwrap();
		assert!(!storey.blocks.is_empty());
		assert!(
			storey.blocks.iter().any(|b| !b.apartments.is_empty())
				|| storey.blocks.iter().any(|b| !b.halls.hall_bands.is_empty()),
			"expected apartments or carved halls in at least one block"
		);
	}

	#[test]
	fn seed_1337_apartments_have_real_hall_frontage() {
		let storey = storey_seed(1337);
		for block in &storey.blocks {
			for apt in &block.apartments {
				let mut best = 0.0_f32;
				for part in apt.cells.iter() {
					let cell = PlanCell::new(0, host_xz(&part.confines.bounds));
					best = best.max(hall_frontage_length(&cell, &block.halls.hall_bands, 1e-3));
				}
				assert!(
					best + 1e-3 >= MIN_GROUP_CONNECTIVITY,
					"apartment hall frontage {best:.3} < {MIN_GROUP_CONNECTIVITY}"
				);
			}
		}
	}

	#[test]
	fn seed_1337_later_block_inherits_progressive_boundary() {
		let storey = storey_seed(1337);
		if storey.blocks.len() < 2 {
			return;
		}
		// Flange (later) should carry Boundary openings from progressive handoff
		// and/or exterior marking.
		let flange = &storey.blocks[1];
		let has_boundary = flange
			.confines
			.openings
			.iter()
			.any(|(_, o)| matches!(o.label, OpeningLabel::Boundary));
		assert!(has_boundary, "expected Boundary openings on progressive/exterior faces");
		// Stem should author some enclosure walls (shared interface + hall).
		assert!(
			!storey.blocks[0].walls.is_empty(),
			"stem block should wall its claimed interfaces"
		);
	}

	#[test]
	fn seed_1337_flange_livable1_no_oversized_closets() {
		use crate::usage_areas::livable_apartment::ApartmentRoom;
		use procedural_common::aabb2_area;

		let storey = storey_seed(1337);
		assert!(storey.blocks.len() >= 2, "expected stem + flange");
		let flange = &storey.blocks[1];
		let livable1 =
			flange.apartments.iter().find(|a| a.region_id == 0).expect("Flange Livable 1");
		for room in &livable1.rooms {
			if let ApartmentRoom::HouseholdCloset { confines, .. } = room {
				let area = aabb2_area(host_xz(&confines.bounds));
				assert!(
					area < 8.0,
					"HouseholdCloset area {area:.2} ≥ 8 — normalize should reopen large demotions"
				);
			}
		}
	}

	#[test]
	fn seed_1337_flange_livable1_keeps_closed_rooms() {
		use crate::usage_areas::livable_apartment::ApartmentRoom;
		use procedural_common::aabb2_area;

		let storey = storey_seed(1337);
		let apt = storey.blocks[1]
			.apartments
			.iter()
			.find(|a| a.region_id == 0)
			.expect("Flange Livable 1");
		let closed = apt.rooms.iter().filter(|r| r.is_closed()).count();
		let large_open_halls = apt
			.rooms
			.iter()
			.filter(|r| matches!(r, ApartmentRoom::OpenHall { .. }))
			.filter_map(|r| match r {
				ApartmentRoom::OpenHall { confines, .. } => {
					Some(aabb2_area(host_xz(&confines.bounds)))
				}
				_ => None,
			})
			.filter(|&a| a > 12.0)
			.count();
		assert!(
			closed > 0,
			"SpineHall closed rooms should survive normalize (not demote to OpenHall)"
		);
		assert!(
			large_open_halls <= 1,
			"expected at most the spine hall as a large OpenHall, got {large_open_halls}"
		);
	}

	/// Kitchen/living furniture must not sit flush on a shared entry/hall wall
	/// (Flange Livable 1 + 5). Checks the shallow passage-wall lip, not a full
	/// hall inflate (open rooms may overlap spine bands by design).
	#[test]
	fn seed_1337_flange_livable_open_furniture_clears_halls() {
		use crate::usage_areas::livable_apartment::ApartmentRoom;
		use crate::usage_areas::plan_cells::shared_edge_span;
		use crate::usage_areas::PASSAGE_WALL_LIP;
		use bevy_math::bounding::{Aabb2d, BoundingVolume};
		use bevy_math::Vec2;
		use procedural_common::{aabb2_area, intersects_aabb2};

		fn fill_xz(t: bevy_math::Vec3, s: bevy_math::Vec3) -> Aabb2d {
			Aabb2d {
				min: Vec2::new(t.x - s.x * 0.5, t.z - s.z * 0.5),
				max: Vec2::new(t.x + s.x * 0.5, t.z + s.z * 0.5),
			}
		}

		/// Inward lip across the shared edge span, into `room`.
		fn shared_wall_lip(
			room: Aabb2d,
			along_x: bool,
			lo: f32,
			hi: f32,
			mid: f32,
			depth: f32,
		) -> Aabb2d {
			if along_x {
				let inward_pos = room.center().y >= mid;
				if inward_pos {
					Aabb2d { min: Vec2::new(lo, mid), max: Vec2::new(hi, mid + depth) }
				} else {
					Aabb2d { min: Vec2::new(lo, mid - depth), max: Vec2::new(hi, mid) }
				}
			} else if room.center().x >= mid {
				Aabb2d { min: Vec2::new(mid, lo), max: Vec2::new(mid + depth, hi) }
			} else {
				Aabb2d { min: Vec2::new(mid - depth, lo), max: Vec2::new(mid, hi) }
			}
		}

		let storey = storey_seed(1337);
		assert!(storey.blocks.len() >= 2, "expected stem + flange");
		let flange = &storey.blocks[1];
		for &rid in &[0u32, 4u32] {
			let Some(apt) = flange.apartments.iter().find(|a| a.region_id == rid) else {
				continue;
			};
			let halls: Vec<Aabb2d> = apt
				.walkways
				.iter()
				.copied()
				.chain(apt.rooms.iter().filter_map(|r| match r {
					ApartmentRoom::Entryway { confines, .. }
					| ApartmentRoom::OpenHall { confines, .. } => Some(host_xz(&confines.bounds)),
					_ => None,
				}))
				.collect();
			for room in &apt.rooms {
				let mut footprints = Vec::new();
				let room_xz = match room {
					ApartmentRoom::Kitchen(k) => {
						for f in k
							.counter_runs
							.iter()
							.chain(k.peninsulas.iter())
							.chain(k.islands.iter())
							.chain(k.fillers.iter())
						{
							footprints.push(fill_xz(
								f.label.placement.translation,
								f.label.placement.scale,
							));
						}
						fill_xz(k.room_type.placement.translation, k.room_type.placement.scale)
					}
					ApartmentRoom::Living(l) => {
						for f in l
							.primary_seating
							.iter()
							.chain(l.secondary_seating.iter())
							.chain(l.fillers.iter())
						{
							footprints.push(fill_xz(
								f.label.placement.translation,
								f.label.placement.scale,
							));
						}
						fill_xz(l.room_type.placement.translation, l.room_type.placement.scale)
					}
					_ => continue,
				};
				for hall in &halls {
					let Some((along_x, lo, hi, mid)) = shared_edge_span(room_xz, *hall) else {
						continue;
					};
					// Skip open-over-spine: room substantially overlaps the hall.
					let overlap = Aabb2d {
						min: Vec2::new(
							room_xz.min.x.max(hall.min.x),
							room_xz.min.y.max(hall.min.y),
						),
						max: Vec2::new(
							room_xz.max.x.min(hall.max.x),
							room_xz.max.y.min(hall.max.y),
						),
					};
					if aabb2_area(overlap) > 0.25 {
						continue;
					}
					let lip =
						shared_wall_lip(room_xz, along_x, lo, hi, mid, PASSAGE_WALL_LIP - 0.05);
					for fp in &footprints {
						assert!(
							!intersects_aabb2(*fp, lip),
							"region {rid}: open furniture flush on hall wall"
						);
					}
				}
			}
		}
	}
}
