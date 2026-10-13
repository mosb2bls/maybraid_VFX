use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::partitions::{Partition, SLICE_KIT_HEIGHT};

use crate::openings::{MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};

use super::ring::SECTORS;
use super::{ArcFloor, ArcFloorParams, ArcFloorSlab};

fn openings_at(ts_labels: &[(&str, f32, OpeningLabel)]) -> Openings {
	let mut openings = Openings::new();
	for (id, t, label) in ts_labels {
		let (id, opening) =
			ArcFloor::plan_opening_at_t(*id, label.clone(), Vec3::ZERO, 4.0, 3.0, *t);
		openings.insert(id, opening);
	}
	openings
}

#[test]
fn openings_cut_wall_partitions() -> anyhow::Result<()> {
	let floor = ArcFloorParams::new(Vec3::ZERO, 4.0, 3.0)
		.openings(openings_at(&[
			("door", 0.0, OpeningLabel::Passage),
			("window", 0.5, OpeningLabel::Aperture), // −X
		]))
		.build();
	assert!(!floor.wall_partitions().is_empty());
	assert!(floor.openings().len() >= 1);
	Ok(())
}

#[test]
fn slab_none_omits_nodes() -> anyhow::Result<()> {
	let floor = ArcFloorParams::new(Vec3::ZERO, 4.0, 3.0)
		.floor(ArcFloorSlab::None)
		.ceiling(ArcFloorSlab::None)
		.build();
	assert!(floor.floor_nodes().is_empty());
	assert!(floor.ceiling_nodes().is_empty());
	Ok(())
}

#[test]
fn solid_slab_without_openings() -> anyhow::Result<()> {
	let solid = ArcFloorParams::new(Vec3::ZERO, 4.0, 3.0).floor(ArcFloorSlab::Solid).build();
	// 4 caps + 1 inscribed fill
	assert_eq!(solid.floor_nodes().len(), 5);
	Ok(())
}

#[test]
fn large_floor_opening_removes_slab() -> anyhow::Result<()> {
	let r = 4.0;
	// Square AABB with half-length ≈ radius → removes entire Solid floor.
	let mut openings = Openings::new();
	openings.insert(
		"clear",
		Opening::new(
			Aabb3d::from_min_max(Vec3::new(-r, -0.5, -r), Vec3::new(r, 0.5, r)),
			OpeningLabel::Shaft,
		),
	);
	let floor = ArcFloorParams::new(Vec3::ZERO, r, 3.0)
		.floor(ArcFloorSlab::Solid)
		.openings(openings)
		.build();
	assert!(floor.floor_nodes().is_empty());
	Ok(())
}

#[test]
fn mapped_opening_from_wall_hit() -> anyhow::Result<()> {
	let connect = OpeningId::new("connect");
	// t = 0 → +X (arc assets sit on local +X at yaw 0).
	let floor = ArcFloorParams::new(Vec3::ZERO, 4.0, 3.0)
		.openings(openings_at(&[("connect", 0.0, OpeningLabel::Passage)]))
		.build();
	let east = floor
		.mapped_opening(&connect)
		.ok_or_else(|| anyhow::anyhow!("missing mapped opening {connect:?}"))?;
	let orient = east.orientation.normalize();
	assert!(orient.x > 0.7, "east door should face +X, orient={orient:?}");
	let (bl, br, ..) = east.endpoint_corners();
	let mid = (bl + br) * 0.5;
	assert!(mid.x > 3.0, "mapped mid should sit on +X ring, mid={mid:?}");
	assert!(bl.distance(br) > 0.1);
	Ok(())
}

#[test]
fn passage_does_not_cut_floor_slab() -> anyhow::Result<()> {
	let floor = ArcFloorParams::new(Vec3::ZERO, 4.0, 3.0)
		.floor(ArcFloorSlab::Solid)
		.openings(openings_at(&[("door", 0.0, OpeningLabel::Passage)]))
		.build();
	// Solid fill with no slab-cutting openings: 4 caps + 1 inscribed rect.
	assert_eq!(floor.floor_nodes().len(), 5);
	Ok(())
}

#[test]
fn east_door_does_not_drop_quarter_ring() -> anyhow::Result<()> {
	let floor = ArcFloorParams::new(Vec3::ZERO, 4.0, 3.0)
		.openings(openings_at(&[("door", 0.0, OpeningLabel::Passage)]))
		.build();
	let solid_deg: f32 = floor
		.wall_partitions()
		.iter()
		.filter_map(|p| match &p.geometry {
			Partition::Arc(a) => Some(a.sweep_degrees),
			Partition::SliceArc(a) => Some(a.sweep_degrees),
			_ => None,
		})
		.sum();
	// Full ring is 360°; one door should remove well under a quarter of solid.
	assert!(solid_deg > 300.0, "unexpected missing wall mass: solid_deg={solid_deg}");
	Ok(())
}

#[test]
fn opening_aabb_and_wall_cut_share_the_same_side() -> anyhow::Result<()> {
	let (_id, opening) =
		ArcFloor::plan_opening_at_t("door", OpeningLabel::Passage, Vec3::ZERO, 4.0, 3.0, 0.0);
	let opening_x = 0.5 * (opening.bounds.min.x + opening.bounds.max.x);
	assert!(opening_x > 3.0, "plan opening should be on +X, x={opening_x}");

	let floor = ArcFloorParams::new(Vec3::ZERO, 4.0, 3.0)
		.openings(Openings::new().with("door", opening))
		.build();
	// Non-solid sectors for this door must be low indices (yaw near 0 → +X), not ~12 (−X).
	let (sectors, _) = floor.params().resolve_wall_sweeps();
	let hit: Vec<u32> = (0..SECTORS).filter(|&i| !sectors[i as usize].is_solid()).collect();
	assert!(!hit.is_empty(), "door should cut at least one sector");
	for i in &hit {
		assert!(
			*i <= 2 || *i >= SECTORS - 2,
			"cut sector {i} is on the wrong side of the ring (hits={hit:?})"
		);
	}
	Ok(())
}

#[test]
fn raised_aperture_keeps_footer_strip() -> anyhow::Result<()> {
	// Window at −Z, clear of the floor: y ∈ [1, 2] on a 3m storey.
	let mut openings = Openings::new();
	openings.insert(
		"window",
		Opening::new(
			Aabb3d::from_min_max(Vec3::new(-0.8, 1.0, -4.5), Vec3::new(0.8, 2.0, -3.2)),
			OpeningLabel::Aperture,
		),
	);
	let floor = ArcFloorParams::new(Vec3::ZERO, 4.0, 3.0).openings(openings).build();
	let (sectors, parts) = floor.params().resolve_wall_sweeps();
	let cut: Vec<_> = (0..SECTORS).filter(|&i| !sectors[i as usize].is_solid()).collect();
	assert!(!cut.is_empty(), "window should cut sectors");
	// Footer band [0, 1): slice kit with Y-scale = h / SLICE_KIT_HEIGHT (= 5 for h=1).
	let expect_y_scale = 1.0 / SLICE_KIT_HEIGHT;
	let footers = parts
		.iter()
		.filter(|p| {
			matches!(p.geometry, Partition::SliceArc(_))
				&& (p.placement.translation.y - 0.0).abs() < 1e-3
				&& (p.placement.scale.y - expect_y_scale).abs() < 1e-2
		})
		.count();
	assert!(footers >= 1, "expected scaled slice footer under window, cut={cut:?}");
	Ok(())
}

#[test]
fn solid_runs_prefer_large_sweeps() -> anyhow::Result<()> {
	// No openings → one 180 + leftover merge into large arcs (12+12 sectors).
	let floor = ArcFloorParams::new(Vec3::ZERO, 4.0, 3.0).build();
	let arcs = floor
		.wall_partitions()
		.iter()
		.filter(|p| matches!(p.geometry, Partition::Arc(_)))
		.count();
	assert!(arcs <= 4, "expected few merged solids, got {arcs}");
	assert!(!floor.wall_partitions().is_empty());
	Ok(())
}
