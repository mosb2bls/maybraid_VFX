use bevy_math::Vec3;
use building_components::partitions::Partition;
use building_components::BuildingComponents;
use lod::scene::LodSceneLevel;

use crate::openings::{MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};

use super::{CircRingFloor, CircRingFloorParams, CircRingFloorSlab};

#[test]
fn default_constructs_two_rings() {
	let r = CircRingFloorParams::default().build();
	assert!(!r.outer_wall().partitions.is_empty());
	assert!(!r.inner_wall().partitions.is_empty());
	assert!(r.params().inner_radius < r.params().outer_radius);
	assert!(!r.has_floor());
}

#[test]
fn annulus_clip_set_on_solid_floor() {
	let r = CircRingFloorParams::default().floor(CircRingFloorSlab::Solid).build();
	assert!(r.has_floor());
	let floor = r.floor_circle().expect("floor");
	assert_eq!(floor.clip, Some(r.params().inner_radius));
	assert!(floor.radius >= r.params().outer_radius - 1e-3);
}

#[test]
fn passage_maps_on_outer() {
	let (id, opening) =
		CircRingFloor::plan_opening_at_t("door", OpeningLabel::Passage, Vec3::ZERO, 5.0, 3.0, 0.0);
	let r = CircRingFloorParams::default()
		.openings(Openings::new().with(id.clone(), opening))
		.build();
	assert!(r.mapped_opening(&OpeningId::new("door")).is_some());
	let slices = r
		.outer_wall()
		.partitions
		.iter()
		.filter(|p| matches!(p.geometry, Partition::SliceArc(_)))
		.count();
	assert!(slices >= 1, "outer ring should get a slice opening");
}

#[test]
fn cuts_slab_can_omit_annulus() {
	let mut openings = Openings::new();
	openings.insert(
		"shaft",
		Opening::new(
			bevy_math::bounding::Aabb3d::from_min_max(
				Vec3::new(-6.0, -0.5, -6.0),
				Vec3::new(6.0, 0.5, 6.0),
			),
			OpeningLabel::Shaft,
		),
	);
	let solid = CircRingFloorParams::default().floor(CircRingFloorSlab::Solid).build();
	let cut = CircRingFloorParams::default()
		.floor(CircRingFloorSlab::Solid)
		.openings(openings)
		.build();
	assert!(solid.has_floor());
	assert!(!cut.has_floor());
	let _ = solid.panel_nodes_for_level(LodSceneLevel::High);
}
