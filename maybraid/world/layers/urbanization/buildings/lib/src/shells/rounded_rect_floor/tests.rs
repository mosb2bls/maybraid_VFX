use bevy_math::{Vec2, Vec3};
use building_components::panels::PanelGeometry;
use building_components::partitions::Partition;
use building_components::BuildingComponents;
use lod::scene::LodSceneLevel;

use crate::openings::{MapsOpenings, OpeningId, Openings};
use crate::paneling::ClippedRectangularStripPiece;

use super::{RoundedRectFloor, RoundedRectFloorParams, RoundedRectFloorSide, RoundedRectFloorSlab};

#[test]
fn default_has_straights_and_arc_corners() {
	let r = RoundedRectFloor::new(RoundedRectFloorParams::default());
	assert!((r.corner_radius() - 1.0).abs() < 1e-4);
	for s in r.straights() {
		assert!(!s.pieces().is_empty());
		assert!(s
			.pieces()
			.iter()
			.flat_map(|p| p.panels())
			.all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
	}
	for c in r.corners() {
		let corner = c.as_ref().expect("corner arc present");
		assert!(!corner.partitions.is_empty());
		assert!(corner.partitions.iter().any(|p| matches!(p.geometry, Partition::Arc(_))));
	}
	let south = &r.straights()[RoundedRectFloorSide::South.face_index()];
	let len = match &south.pieces()[0] {
		ClippedRectangularStripPiece::Solid(p) => p.edge.length(),
		ClippedRectangularStripPiece::Clipped(p) => p.edge.length(),
	};
	assert!((len - 6.0).abs() < 1e-3, "straight len={len}");
}

#[test]
fn zero_radius_full_side_straights() {
	let mut params = RoundedRectFloorParams::default();
	params.corner_radius = 0.0;
	let r = params.build();
	assert!(r.corner_radius() < 1e-4);
	let south = &r.straights()[RoundedRectFloorSide::South.face_index()];
	let len = match &south.pieces()[0] {
		ClippedRectangularStripPiece::Solid(p) => p.edge.length(),
		ClippedRectangularStripPiece::Clipped(p) => p.edge.length(),
	};
	assert!((len - 8.0).abs() < 1e-3);
	assert!(r.corners().iter().all(|c| c.is_none()));
}

#[test]
fn south_passage_clips_straight() {
	let r = RoundedRectFloorParams::default()
		.openings(Openings::new().with(
			"south",
			RoundedRectFloor::side_passage_opening(
				RoundedRectFloorSide::South,
				Vec3::ZERO,
				Vec2::new(8.0, 6.0),
				1.2,
				2.1,
			),
		))
		.build();
	assert!(matches!(
		r.straights()[RoundedRectFloorSide::South.face_index()].pieces()[0],
		ClippedRectangularStripPiece::Clipped(_)
	));
	assert!(r.mapped_opening(&OpeningId::new("south")).is_some());
}

#[test]
fn solid_floor_emits_core_edges_and_quarters() {
	let r = RoundedRectFloorParams::default().floor(RoundedRectFloorSlab::Solid).build();
	assert!(r.has_floor());
	// 4 wall straights + core + 4 edge strips + quarter fans → well above walls alone.
	let panels = r.panel_nodes_for_level(LodSceneLevel::High);
	assert!(panels.len() > 8, "expected core + edge strips + quarters, got {}", panels.len());
	let parts = r.partition_nodes_for_level(LodSceneLevel::High);
	assert!(!parts.is_empty(), "arc corners should emit partitions");
}
