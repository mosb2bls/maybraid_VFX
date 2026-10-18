use bevy_math::{Vec2, Vec3};
use building_components::panels::PanelGeometry;
use building_components::BuildingComponents;
use lod::scene::LodSceneLevel;

use crate::openings::{MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::ClippedRectangularStripPiece;

use super::{IFloor, IFloorParams, IFloorSlab};

#[test]
fn full_i_has_more_edges_than_l() {
	let i = IFloorParams::default().build();
	let l = IFloorParams::new(Vec3::ZERO, Vec2::new(2.0, 6.0), 3.0)
		.top_left_length(None)
		.top_right_length(None)
		.bottom_left_length(Some(2.0))
		.bottom_right_length(None)
		.build();
	assert!(i.wall_count() > l.wall_count(), "I={} L={}", i.wall_count(), l.wall_count());
	assert!(l.wall_count() > 4, "L should exceed a plain rectangle");
}

#[test]
fn stem_only_is_four_walls() {
	let r = IFloorParams::new(Vec3::ZERO, Vec2::new(2.0, 6.0), 3.0)
		.top_left_length(None)
		.top_right_length(None)
		.bottom_left_length(None)
		.bottom_right_length(None)
		.build();
	assert_eq!(r.wall_count(), 4);
}

#[test]
fn walls_are_rectangle_kits() {
	let r = IFloorParams::default().build();
	for w in r.walls() {
		assert!(!w.pieces().is_empty());
		assert!(w
			.pieces()
			.iter()
			.flat_map(|p| p.panels())
			.all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
	}
}

#[test]
fn passage_on_first_edge_clips() {
	let edges = IFloorParams::default().build().edges().to_vec();
	let edge = edges[0];
	let r = IFloorParams::default()
		.openings(Openings::new().with("door", IFloor::edge_passage_opening(edge, 1.0, 2.1)))
		.build();
	assert!(r.mapped_opening(&OpeningId::new("door")).is_some());
	assert!(r.walls().iter().any(|w| {
		w.pieces().iter().any(|p| matches!(p, ClippedRectangularStripPiece::Clipped(_)))
			|| w.pieces().len() > 1
	}));
}

#[test]
fn solid_floor_has_multiple_pieces() {
	let r = IFloorParams::default().floor(IFloorSlab::Solid).build();
	assert!(r.has_floor());
	let panels = r.panel_nodes_for_level(LodSceneLevel::High);
	assert!(panels.len() > r.wall_count());
}

#[test]
fn shaft_can_remove_a_slab_piece() {
	// Cover the entire top flange AABB.
	let mut openings = Openings::new();
	openings.insert(
		"shaft",
		Opening::new(
			bevy_math::bounding::Aabb3d::from_min_max(
				Vec3::new(-10.0, -0.5, 2.0),
				Vec3::new(10.0, 0.5, 10.0),
			),
			OpeningLabel::Shaft,
		),
	);
	let with = IFloorParams::default().floor(IFloorSlab::Solid).openings(openings).build();
	let without = IFloorParams::default().floor(IFloorSlab::Solid).build();
	assert!(with.has_floor());
	assert!(
		with.panel_nodes_for_level(LodSceneLevel::High).len()
			< without.panel_nodes_for_level(LodSceneLevel::High).len()
	);
}
