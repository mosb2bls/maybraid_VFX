use bevy_math::{Vec2, Vec3};
use building_components::BuildingComponents;
use lod::scene::LodSceneLevel;

use crate::openings::{MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::clipped_ruled_strip::ClippedStripPiece;

use super::openings::ground_door_clip;
use super::{Trazaloid, TrazaloidParams, TrazaloidSide, TrazaloidSlab};

fn demo_params() -> TrazaloidParams {
	TrazaloidParams::default()
}

#[test]
fn resolves_waist_inset_and_gap() -> anyhow::Result<()> {
	let t = Trazaloid::new(demo_params());
	let levels = t.plan_levels();
	assert!((levels[0].0 - 0.0).abs() < 1e-5);
	assert!((levels[1].0 - 3.0).abs() < 1e-5);
	assert!((levels[2].0 - 3.35).abs() < 1e-5);
	assert!((levels[3].0 - 5.85).abs() < 1e-5);
	assert!(levels[1].1.x < levels[0].1.x);
	assert!(levels[1].1.x > levels[3].1.x - 1.0);
	assert_eq!(levels[1].1, levels[2].1);
	Ok(())
}

#[test]
fn default_has_ceiling_no_floor_no_openings() -> anyhow::Result<()> {
	let t = Trazaloid::new(demo_params());
	for w in t.lower_walls() {
		assert!(!w.pieces().is_empty());
		assert!(matches!(w.pieces()[0], ClippedStripPiece::Solid(_)));
	}
	for w in t.upper_walls() {
		assert!(!w.pieces().is_empty());
	}
	assert!(t.floor().is_none());
	assert!(t.openings().is_empty());
	let ceiling = t.ceiling().ok_or_else(|| anyhow::anyhow!("default solid ceiling missing"))?;
	assert!(!ceiling.pieces().is_empty());
	Ok(())
}

#[test]
fn can_omit_ceiling_and_cut_floor_with_shaft() -> anyhow::Result<()> {
	let mut openings = Openings::new();
	openings.insert(
		"shaft",
		Opening::new(
			bevy_math::bounding::Aabb3d::from_min_max(
				Vec3::new(-1.0, -0.5, -1.0),
				Vec3::new(1.0, 0.5, 1.0),
			),
			OpeningLabel::Shaft,
		),
	);
	let t = TrazaloidParams::default()
		.ceiling(TrazaloidSlab::None)
		.floor(TrazaloidSlab::Solid)
		.openings(openings)
		.build();
	assert!(t.ceiling().is_none());
	let floor = t.floor().ok_or_else(|| anyhow::anyhow!("floor present expected"))?;
	assert!(matches!(floor.pieces()[0], ClippedStripPiece::Clipped(_)));
	Ok(())
}

#[test]
fn passage_does_not_cut_floor_slab() -> anyhow::Result<()> {
	let t = TrazaloidParams::default()
		.floor(TrazaloidSlab::Solid)
		.openings(Openings::new().with(
			"south",
			Trazaloid::side_passage_opening(TrazaloidSide::South, Vec2::new(8.0, 6.0), 1.2, 2.1),
		))
		.build();
	let floor = t.floor().ok_or_else(|| anyhow::anyhow!("solid floor expected"))?;
	assert!(matches!(floor.pieces()[0], ClippedStripPiece::Solid(_)));
	Ok(())
}

#[test]
fn south_door_makes_clipped_lower_piece() -> anyhow::Result<()> {
	let params = TrazaloidParams::default().openings(Openings::new().with(
		"south",
		Trazaloid::side_passage_opening(TrazaloidSide::South, Vec2::new(8.0, 6.0), 1.2, 2.1),
	));
	let t = Trazaloid::new(params);
	assert!(matches!(t.lower_walls()[2].pieces()[0], ClippedStripPiece::Clipped(_)));
	assert!(matches!(t.lower_walls()[0].pieces()[0], ClippedStripPiece::Solid(_)));
	Ok(())
}

#[test]
fn largest_passage_wins_per_side() -> anyhow::Result<()> {
	let footprint = Vec2::new(8.0, 6.0);
	let mut openings = Openings::new();
	openings.insert(
		"small",
		Trazaloid::side_passage_opening(TrazaloidSide::South, footprint, 0.6, 1.5),
	);
	openings.insert(
		"large",
		Trazaloid::side_passage_opening(TrazaloidSide::South, footprint, 2.0, 2.4),
	);
	let t = TrazaloidParams::default().openings(openings).build();
	assert!(t.mapped_opening(&OpeningId::new("large")).is_some());
	assert!(t.mapped_opening(&OpeningId::new("small")).is_none());
	let large = t.mapped_opening(&OpeningId::new("large")).unwrap();
	let (bl, br, ..) = large.endpoint_corners();
	assert!(bl.distance(br) > 1.5, "expected wider door from large passage");
	Ok(())
}

#[test]
fn aperture_does_not_map_or_clip() -> anyhow::Result<()> {
	let footprint = Vec2::new(8.0, 6.0);
	let mut openings = Openings::new();
	let mut aperture = Trazaloid::side_passage_opening(TrazaloidSide::North, footprint, 1.5, 1.2);
	aperture.label = OpeningLabel::Aperture;
	openings.insert("win", aperture);
	let t = TrazaloidParams::default().openings(openings).build();
	assert!(t.mapped_opening(&OpeningId::new("win")).is_none());
	assert!(matches!(t.lower_walls()[0].pieces()[0], ClippedStripPiece::Solid(_)));
	Ok(())
}

#[test]
fn door_clip_reaches_ground_and_honors_width() -> anyhow::Result<()> {
	let a0 = Vec3::new(1.0, 0.0, -3.0);
	let b0 = Vec3::new(-1.0, 0.0, -3.0);
	let a1 = Vec3::new(0.8, 3.0, -2.5);
	let b1 = Vec3::new(-0.8, 3.0, -2.5);
	let clip = ground_door_clip(a0, b0, a1, b1, 1.0, 1.8);
	assert_eq!(clip.len(), 4);
	assert!((clip[0].y - 0.0).abs() < 1e-4);
	assert!((clip[1].y - 0.0).abs() < 1e-4);
	assert!((clip[0].distance(clip[1]) - 1.0).abs() < 1e-3);
	Ok(())
}

#[test]
fn high_emits_more_joints_than_medium() -> anyhow::Result<()> {
	let t = Trazaloid::new(demo_params());
	let high = t.joint_nodes_for_level(LodSceneLevel::High).len();
	let mid = t.joint_nodes_for_level(LodSceneLevel::Medium).len();
	assert!(high > mid);
	Ok(())
}

#[test]
fn mapped_opening_matches_passage_plan() -> anyhow::Result<()> {
	let connect = OpeningId::new("connect");
	let t = TrazaloidParams::default()
		.openings(Openings::new().with(
			connect.clone(),
			Trazaloid::side_passage_opening(TrazaloidSide::West, Vec2::new(8.0, 6.0), 1.2, 2.1),
		))
		.build();
	assert!(t.mapped_opening(&OpeningId::new("missing")).is_none());
	let west = t
		.mapped_opening(&connect)
		.ok_or_else(|| anyhow::anyhow!("missing west mapped opening"))?;
	let o = west.orientation.normalize();
	assert!(o.x < -0.9, "orientation={o:?}");
	let (bl, br, tl, tr) = west.endpoint_corners();
	assert!(bl.y.abs() < 1e-3);
	let bottom_x = 0.5 * (bl.x + br.x);
	let top_x = 0.5 * (tl.x + tr.x);
	assert!(
		top_x > bottom_x + 1e-3,
		"pitched top should inset toward center: bottom_x={bottom_x} top_x={top_x}"
	);
	let half = 0.5 * bl.distance(br);
	let wide = west.widened(1.0);
	let (wbl, wbr, ..) = wide.endpoint_corners();
	let half_wide = 0.5 * wbl.distance(wbr);
	assert!(half_wide > half + 0.9, "half={half} half_wide={half_wide}");
	assert!(wbl.z > wbr.z);
	Ok(())
}

#[test]
fn origin_translates_plan_and_west_passage() -> anyhow::Result<()> {
	let origin = Vec3::new(20.0, 5.0, -8.0);
	let footprint = Vec2::new(8.0, 6.0);
	let connect = OpeningId::new("west");
	let t = TrazaloidParams::default()
		.origin(origin)
		.openings(Openings::new().with(
			connect.clone(),
			Trazaloid::side_passage_opening(TrazaloidSide::West, footprint, 1.2, 2.1),
		))
		.build();
	let levels = t.plan_levels();
	anyhow::ensure!((levels[0].0 - origin.y).abs() < 1e-5, "foot y {}", levels[0].0);
	let west = t
		.mapped_opening(&connect)
		.ok_or_else(|| anyhow::anyhow!("west passage should still map after origin shift"))?;
	let o = west.orientation.normalize();
	anyhow::ensure!(o.x < -0.9, "orientation={o:?}");
	let (bl, br, ..) = west.endpoint_corners();
	let mid_x = 0.5 * (bl.x + br.x);
	anyhow::ensure!(
		(mid_x - (origin.x - footprint.x * 0.5)).abs() < 0.6,
		"west door mid_x={mid_x}"
	);
	Ok(())
}

#[test]
fn shaft_cuts_floor_when_origin_is_translated() -> anyhow::Result<()> {
	let origin = Vec3::new(40.0, 12.0, -25.0);
	let mut openings = Openings::new();
	openings.insert(
		"shaft",
		Opening::new(
			bevy_math::bounding::Aabb3d::from_min_max(
				Vec3::new(-1.0, -0.5, -1.0),
				Vec3::new(1.0, 0.5, 1.0),
			),
			OpeningLabel::Shaft,
		),
	);
	let t = TrazaloidParams::default()
		.origin(origin)
		.ceiling(TrazaloidSlab::None)
		.floor(TrazaloidSlab::Solid)
		.openings(openings)
		.build();
	let floor = t.floor().ok_or_else(|| anyhow::anyhow!("translated floor missing"))?;
	anyhow::ensure!(
		matches!(floor.pieces()[0], ClippedStripPiece::Clipped(_)),
		"shaft should cut the world-space floor slab"
	);
	Ok(())
}
