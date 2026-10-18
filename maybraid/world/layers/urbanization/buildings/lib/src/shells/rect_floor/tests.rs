use bevy_math::{Vec2, Vec3};
use building_components::panels::PanelGeometry;
use building_components::BuildingComponents;
use lod::scene::LodSceneLevel;

use crate::openings::{MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::ClippedRectangularStripPiece;

use super::{RectFloor, RectFloorParams, RectFloorSide, RectFloorSlab};

#[test]
fn default_emits_rectangle_kit_walls() {
	let r = RectFloor::new(RectFloorParams::default());
	assert_eq!(r.walls().n(), 4);
	for face in r.walls().faces() {
		assert_eq!(face.pieces().len(), 1);
		assert!(face
			.pieces()
			.iter()
			.flat_map(|p| p.panels())
			.all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
	}
	assert!(!r.has_floor());
	assert!(!r.has_ceiling());
	assert!(r.openings().is_empty());
}

#[test]
fn south_passage_clips_and_maps_offset() {
	let center = Vec3::ZERO;
	let footprint = Vec2::new(8.0, 6.0);
	// Offset door toward +X on south face.
	let opening =
		RectFloor::side_passage_opening(RectFloorSide::South, center, footprint, 1.2, 2.1);
	// Shift the authored AABB toward +X (helper is centered; rebuild offset).
	let mut openings = Openings::new();
	let mut o = opening;
	let min = Vec3::from(o.bounds.min) + Vec3::new(2.0, 0.0, 0.0);
	let max = Vec3::from(o.bounds.max) + Vec3::new(2.0, 0.0, 0.0);
	o.bounds = bevy_math::bounding::Aabb3d::from_min_max(min, max);
	openings.insert("south", o);

	let r = RectFloorParams::default().openings(openings).build();
	assert!(matches!(
		r.walls().faces()[RectFloorSide::South.face_index()].pieces()[0],
		ClippedRectangularStripPiece::Clipped(_)
	));
	let mapped = r.mapped_opening(&OpeningId::new("south")).expect("mapped south");
	assert!(mapped.orientation.y < -0.9);
	let (bl, br, ..) = mapped.endpoint_corners();
	let mid_x = 0.5 * (bl.x + br.x);
	assert!(mid_x > 0.5, "door should sit on +X side, mid_x={mid_x}");
}

#[test]
fn aperture_maps_as_window() {
	let r = RectFloorParams::default()
		.openings(Openings::new().with(
			"win",
			RectFloor::side_aperture_opening(
				RectFloorSide::North,
				Vec3::ZERO,
				Vec2::new(8.0, 6.0),
				1.5,
				1.2,
				1.0,
			),
		))
		.build();
	assert!(r.mapped_opening(&OpeningId::new("win")).is_some());
	assert!(matches!(
		r.walls().faces()[RectFloorSide::North.face_index()].pieces()[0],
		ClippedRectangularStripPiece::Clipped(_)
	));
}

#[test]
fn shaft_cuts_floor_at_position() {
	let mut openings = Openings::new();
	openings.insert(
		"shaft",
		Opening::new(
			bevy_math::bounding::Aabb3d::from_min_max(
				Vec3::new(1.0, -0.5, -2.0),
				Vec3::new(2.5, 0.5, -0.5),
			),
			OpeningLabel::Shaft,
		),
	);
	let r = RectFloorParams::default()
		.floor(RectFloorSlab::Solid)
		.openings(openings)
		.build();
	assert!(r.has_floor());
	let panels = r.panel_nodes_for_level(LodSceneLevel::High);
	assert!(panels.len() > 4, "walls + framed floor");
}

#[test]
fn passage_does_not_cut_floor() {
	let r = RectFloorParams::default()
		.floor(RectFloorSlab::Solid)
		.openings(Openings::new().with(
			"door",
			RectFloor::side_passage_opening(
				RectFloorSide::West,
				Vec3::ZERO,
				Vec2::new(8.0, 6.0),
				1.2,
				2.1,
			),
		))
		.build();
	assert!(r.has_floor());
}

#[test]
fn largest_passage_wins_per_side() {
	let footprint = Vec2::new(8.0, 6.0);
	let mut openings = Openings::new();
	openings.insert(
		"small",
		RectFloor::side_passage_opening(RectFloorSide::South, Vec3::ZERO, footprint, 0.6, 1.5),
	);
	openings.insert(
		"large",
		RectFloor::side_passage_opening(RectFloorSide::South, Vec3::ZERO, footprint, 2.0, 2.4),
	);
	let r = RectFloorParams::default().openings(openings).build();
	assert!(r.mapped_opening(&OpeningId::new("large")).is_some());
	assert!(r.mapped_opening(&OpeningId::new("small")).is_none());
}
