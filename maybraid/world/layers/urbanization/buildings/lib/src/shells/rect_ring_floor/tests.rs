use bevy_math::{Vec2, Vec3};
use building_components::panels::PanelGeometry;
use building_components::BuildingComponents;
use lod::scene::LodSceneLevel;

use crate::openings::{MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::ClippedRectangularStripPiece;
use crate::shells::ortho::OrthoSide;

use super::{RectRingFloor, RectRingFloorParams, RectRingFloorSlab};

#[test]
fn default_constructs_outer_and_inner_walls() {
	let r = RectRingFloorParams::default().build();
	// 4 outer + 4 inner sides.
	assert_eq!(r.wall_count(), 8);
	assert!(!r.has_floor());
}

#[test]
fn omitting_inner_walls_keeps_outer_loop_and_slabs() {
	let r = RectRingFloorParams::default()
		.inner_walls(false)
		.floor(RectRingFloorSlab::Solid)
		.build();
	assert_eq!(r.wall_count(), 4);
	assert!(r.has_floor());
}

#[test]
fn inner_courtyard_smaller_than_outer() {
	let r = RectRingFloorParams::default().build();
	assert!(r.params().inner.x < r.params().outer.x);
	assert!(r.params().inner.y < r.params().outer.y);
}

#[test]
fn walls_are_rectangle_kits() {
	let r = RectRingFloorParams::default().build();
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
fn passage_on_outer_south_maps() {
	let r = RectRingFloorParams::default()
		.openings(Openings::new().with(
			"door",
			RectRingFloor::side_passage_opening(
				OrthoSide::South,
				Vec3::ZERO,
				Vec2::new(8.0, 6.0),
				1.2,
				2.1,
			),
		))
		.build();
	assert!(r.mapped_opening(&OpeningId::new("door")).is_some());
	assert!(r.walls().iter().any(|w| {
		w.pieces().iter().any(|p| matches!(p, ClippedRectangularStripPiece::Clipped(_)))
	}));
}

#[test]
fn cornerish_passage_maps_to_intersecting_side_not_nearest_mid() {
	// Midpoint is closer to East face, but the AABB only intersects the South
	// wall volume — must still cut South (Les Halles awkward SE door case).
	let opening = Opening::passage(bevy_math::bounding::Aabb3d::from_min_max(
		Vec3::new(2.8, 0.2, -3.15),
		Vec3::new(3.6, 2.8, -2.85),
	));
	let r = RectRingFloorParams::default()
		.openings(Openings::new().with("awkward", opening))
		.build();
	assert!(
		r.mapped_opening(&OpeningId::new("awkward")).is_some(),
		"passage that intersects South must map even if East mid is closer"
	);
}

#[test]
fn corner_depth_nibble_loses_to_true_face_span() {
	// End-of-run South door also clips an adjacent face via authorship depth.
	// Prefer the large true-face span over a ~0.4 m corner nibble.
	let mut door = RectRingFloor::side_passage_opening(
		OrthoSide::South,
		Vec3::ZERO,
		Vec2::new(8.0, 6.0),
		1.4,
		2.4,
	);
	// Push leaf toward the SE corner of the outer ring.
	door.bounds = {
		let min = Vec3::from(door.bounds.min) + Vec3::new(2.6, 0.0, 0.0);
		let max = Vec3::from(door.bounds.max) + Vec3::new(2.6, 0.0, 0.0);
		bevy_math::bounding::Aabb3d::from_min_max(min, max)
	};
	let r = RectRingFloorParams::default()
		.openings(Openings::new().with("se_door", door))
		.build();
	let mapped = r.mapped_opening(&OpeningId::new("se_door")).expect("door must map");
	let cut_w = mapped.face.lower_left.distance(mapped.face.lower_right);
	assert!(cut_w > 1.0, "expected full leaf span, not corner nibble; cut_w={cut_w}");
}

#[test]
fn passage_wins_overlap_against_aperture() {
	let mut openings = Openings::new();
	openings.insert(
		"door",
		RectRingFloor::side_passage_opening(
			OrthoSide::South,
			Vec3::ZERO,
			Vec2::new(8.0, 6.0),
			1.5,
			2.1,
		),
	);
	openings.insert(
		"win",
		RectRingFloor::side_aperture_opening(
			OrthoSide::South,
			Vec3::ZERO,
			Vec2::new(8.0, 6.0),
			1.5,
			1.2,
			1.0,
		),
	);
	let r = RectRingFloorParams::default().openings(openings).build();
	assert!(r.mapped_opening(&OpeningId::new("door")).is_some());
	assert!(r.mapped_opening(&OpeningId::new("win")).is_none());
}

#[test]
fn multiple_openings_on_same_outer_side_all_map() {
	let mut openings = Openings::new();
	let mut door = RectRingFloor::side_passage_opening(
		OrthoSide::South,
		Vec3::ZERO,
		Vec2::new(8.0, 6.0),
		1.2,
		2.1,
	);
	door.bounds = {
		let min = Vec3::from(door.bounds.min) + Vec3::new(-2.0, 0.0, 0.0);
		let max = Vec3::from(door.bounds.max) + Vec3::new(-2.0, 0.0, 0.0);
		bevy_math::bounding::Aabb3d::from_min_max(min, max)
	};
	let mut win = RectRingFloor::side_aperture_opening(
		OrthoSide::South,
		Vec3::ZERO,
		Vec2::new(8.0, 6.0),
		1.2,
		1.2,
		1.0,
	);
	win.bounds = {
		let min = Vec3::from(win.bounds.min) + Vec3::new(2.0, 0.0, 0.0);
		let max = Vec3::from(win.bounds.max) + Vec3::new(2.0, 0.0, 0.0);
		bevy_math::bounding::Aabb3d::from_min_max(min, max)
	};
	openings.insert("door", door);
	openings.insert("win", win);
	let r = RectRingFloorParams::default().openings(openings).build();
	assert!(r.mapped_opening(&OpeningId::new("door")).is_some());
	assert!(r.mapped_opening(&OpeningId::new("win")).is_some());
}

#[test]
fn wide_passage_authors_broad_side_omission() {
	// Nearly full-width south passage is the supported way to open a gallery run.
	let r = RectRingFloorParams::default()
		.openings(Openings::new().with(
			"gap",
			RectRingFloor::side_passage_opening(
				OrthoSide::South,
				Vec3::ZERO,
				Vec2::new(8.0, 6.0),
				7.5,
				2.8,
			),
		))
		.build();
	assert!(r.mapped_opening(&OpeningId::new("gap")).is_some());
	assert_eq!(r.wall_count(), 8);
}

#[test]
fn solid_floor_has_frame_pieces() {
	let r = RectRingFloorParams::default().floor(RectRingFloorSlab::Solid).build();
	assert!(r.has_floor());
	let panels = r.panel_nodes_for_level(LodSceneLevel::High);
	assert!(panels.len() > r.wall_count());
}

#[test]
fn cuts_slab_can_remove_a_frame_band() {
	let mut openings = Openings::new();
	// Cover the entire south frame band (outer −Z strip).
	openings.insert(
		"shaft",
		Opening::new(
			bevy_math::bounding::Aabb3d::from_min_max(
				Vec3::new(-5.0, -0.5, -3.5),
				Vec3::new(5.0, 0.5, -1.0),
			),
			OpeningLabel::Shaft,
		),
	);
	let solid = RectRingFloorParams::default().floor(RectRingFloorSlab::Solid).build();
	let cut = RectRingFloorParams::default()
		.floor(RectRingFloorSlab::Solid)
		.openings(openings)
		.build();
	// Shaft AABBs also map onto walls (extra wall panels); assert the floor
	// frame lost or subdivided the covered south band.
	assert!(
		cut.floor_band_count() != solid.floor_band_count()
			|| cut.panel_nodes_for_level(LodSceneLevel::High).len()
				!= solid.panel_nodes_for_level(LodSceneLevel::High).len(),
		"shaft should change floor bands or panel topology (solid_bands={} cut_bands={})",
		solid.floor_band_count(),
		cut.floor_band_count()
	);
	assert!(
		cut.floor_band_count() <= solid.floor_band_count(),
		"cutting a full south band should not add floor bands (solid={} cut={})",
		solid.floor_band_count(),
		cut.floor_band_count()
	);
}

#[test]
fn two_corner_shafts_both_cut_south_floor_band() {
	// Default outer 8×6, inner 4×3 → south band z∈[-3,-1.5], x∈[-4,4].
	// Two equal corner shafts on that band must each leave a floor void.
	let mut openings = Openings::new();
	openings.insert(
		"sw",
		Opening::new(
			bevy_math::bounding::Aabb3d::from_min_max(
				Vec3::new(-4.0, -0.5, -3.0),
				Vec3::new(-2.0, 0.5, -1.5),
			),
			OpeningLabel::Shaft,
		),
	);
	openings.insert(
		"se",
		Opening::new(
			bevy_math::bounding::Aabb3d::from_min_max(
				Vec3::new(2.0, -0.5, -3.0),
				Vec3::new(4.0, 0.5, -1.5),
			),
			OpeningLabel::Shaft,
		),
	);
	let cut = RectRingFloorParams::default()
		.floor(RectRingFloorSlab::Solid)
		.openings(openings)
		.build();
	assert!(!cut.floor_covers_xz(-3.0, -2.25), "SW shaft footprint must not retain gallery floor");
	assert!(!cut.floor_covers_xz(3.0, -2.25), "SE shaft footprint must not retain gallery floor");
	assert!(cut.floor_covers_xz(0.0, -2.25), "mid-south gallery between shafts should keep floor");
}

#[test]
fn horizontal_floor_kits_use_neg_z_panel_space() {
	use building_components::scene_children::pose;
	let r = RectRingFloorParams::default().floor(RectRingFloorSlab::Solid).build();
	let mut floors = 0usize;
	for node in r.panel_nodes_for_level(LodSceneLevel::High).flatten() {
		if !matches!(node.geometry, PanelGeometry::Rectangle(_)) {
			continue;
		}
		let up = node.placement.rotation() * Vec3::Y;
		if up.y.abs() < 0.8 {
			continue;
		}
		floors += 1;
		let t = pose(node.placement);
		let a0 = t * Vec3::ZERO;
		let a1 = t * Vec3::X;
		let b0 = t * Vec3::new(0.0, 0.0, -1.0);
		let b1 = t * Vec3::new(1.0, 0.0, -1.0);
		let ys = [a0.y, a1.y, b0.y, b1.y];
		let y_span = ys.into_iter().fold(f32::NEG_INFINITY, f32::max)
			- ys.into_iter().fold(f32::INFINITY, f32::min);
		assert!(
			y_span < 0.05,
			"floor kit corners should be coplanar in Y, span={y_span} pitch={}",
			node.placement.pitch
		);
		let dx = (a1 - a0).length();
		let dz = (b0 - a0).length();
		assert!(dx > 0.5 && dz > 0.5, "floor kit should span the band, dx={dx} dz={dz}");
		assert!((b1 - a0).length() > dx.max(dz), "far corner should sit opposite a0");
	}
	assert!(floors >= 4, "ring floor should emit at least four horizontal slabs, got {floors}");
}
