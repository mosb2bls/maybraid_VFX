//! Unit tests for [`super::LivableApartment`].

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::{BuildingComponents, Layer};
use lod::scene::LodSceneLevel;
use procedural_common::{aabb2_area, NoiseParams};

use crate::fit::{Confines, FillRegion, MultiConfines, SpaceKind};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::usage_areas::plan_cells::shared_edge_span;
use crate::usage_areas::plan_geom::host_xz;

use super::layout::{residential_access, room_xz};
use super::room::ApartmentRoom;
use super::{LivableApartment, EPS, INTERNAL_WALLS_LAYER};

fn apt_with_door(extent: Vec3) -> Confines {
	let mut openings = Openings::new();
	openings.insert(
		OpeningId::new("door"),
		Opening::new(
			Aabb3d::from_min_max(
				Vec3::new(extent.x * 0.35, 0.0, -0.15),
				Vec3::new(extent.x * 0.65, 2.2, 0.15),
			),
			OpeningLabel::Passage,
		),
	);
	Confines::new(Aabb3d::from_min_max(Vec3::ZERO, extent), 0.0, openings)
}

#[test]
fn layout_has_entryway_and_rooms() {
	let confines = apt_with_door(Vec3::new(10.0, 3.0, 8.0));
	let (apt, _) = LivableApartment::from_confines(0, &confines, NoiseParams::default()).unwrap();
	assert!(
		apt.rooms.iter().any(|r| matches!(r, ApartmentRoom::Entryway { .. })),
		"expected entryway"
	);
	assert!(
		apt.rooms.iter().any(|r| matches!(
			r,
			ApartmentRoom::Living(_)
				| ApartmentRoom::Kitchen(_)
				| ApartmentRoom::Bedroom(_)
				| ApartmentRoom::Dining(_)
				| ApartmentRoom::OpenHall { .. }
		)),
		"expected at least one common/private/open quarter"
	);
	assert!(!apt.max_rects.is_empty(), "expected max-rect decomposition");
}

#[test]
fn larger_apt_gets_bedroom() {
	let confines = apt_with_door(Vec3::new(14.0, 3.0, 10.0));
	let (apt, _) = LivableApartment::from_confines(
		0,
		&confines,
		NoiseParams { seed: 3, ..Default::default() },
	)
	.unwrap();
	assert!(
		apt.rooms.iter().any(|r| matches!(r, ApartmentRoom::Bedroom(_))),
		"expected bedroom in larger apt"
	);
}

#[test]
fn internal_walls_only_on_high_structural_band() {
	let confines = apt_with_door(Vec3::new(14.0, 3.0, 10.0));
	let (apt, _) = LivableApartment::from_confines(
		0,
		&confines,
		NoiseParams { seed: 3, ..Default::default() },
	)
	.unwrap();
	assert!(
		!apt.partitions.is_empty()
			|| apt
				.rooms
				.iter()
				.any(|r| !r.panel_nodes_for_level(LodSceneLevel::High).is_empty()),
		"fixture needs internal panels to exercise LOD"
	);
	let high = apt.panel_nodes_for_level(LodSceneLevel::High);
	assert!(
		high.labeled.contains_key(&Layer::new(INTERNAL_WALLS_LAYER)),
		"High should tag internal walls"
	);
	let medium = apt.panel_nodes_for_level(LodSceneLevel::Medium);
	assert!(
		!medium.labeled.contains_key(&Layer::new(INTERNAL_WALLS_LAYER)),
		"Medium should drop internal walls"
	);
	assert!(medium.len() < high.len(), "Medium should emit fewer panels than High");
}

#[test]
fn l_shape_reaches_entry_from_far() {
	let bar = FillRegion::new(
		SpaceKind::InternalSpace,
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 3.0, 6.0)),
			0.0,
			{
				let mut o = Openings::new();
				o.insert(
					OpeningId::new("door"),
					Opening::new(
						Aabb3d::from_min_max(Vec3::new(4.0, 0.0, -0.15), Vec3::new(5.0, 2.2, 0.15)),
						OpeningLabel::Passage,
					),
				);
				o
			},
		),
	);
	let stub = FillRegion::new(
		SpaceKind::InternalSpace,
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 6.0), Vec3::new(4.0, 3.0, 12.0)),
			0.0,
			Openings::new(),
		),
	);
	let cells = MultiConfines::new([bar, stub]);
	let (apt, _) = LivableApartment::from_multi(0, &cells, NoiseParams::default()).unwrap();
	assert!(apt.max_rects.len() >= 2, "L should yield ≥2 max-rects");
	assert!(apt.rooms.iter().any(|r| matches!(r, ApartmentRoom::Entryway { .. })));
	let far = apt.rooms.iter().filter_map(room_xz).any(|r| r.min.y > 5.5);
	assert!(far, "expected packed content in far L leg");
}

#[test]
fn shallow_door_cell_entry_stays_connected() {
	let door = FillRegion::new(
		SpaceKind::InternalSpace,
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(3.0, 3.0, 2.7)),
			0.0,
			{
				let mut o = Openings::new();
				o.insert(
					OpeningId::new("door"),
					Opening::new(
						Aabb3d::from_min_max(Vec3::new(0.9, 0.0, -0.15), Vec3::new(2.1, 2.2, 0.15)),
						OpeningLabel::Passage,
					),
				);
				o
			},
		),
	);
	let body = FillRegion::new(
		SpaceKind::InternalSpace,
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 2.7), Vec3::new(6.0, 3.0, 12.0)),
			0.0,
			Openings::new(),
		),
	);
	let (apt, _) =
		LivableApartment::from_multi(0, &MultiConfines::new([door, body]), NoiseParams::default())
			.unwrap();
	let entry = apt
		.rooms
		.iter()
		.find_map(|r| match r {
			ApartmentRoom::Entryway { confines, .. } => Some(host_xz(&confines.bounds)),
			_ => None,
		})
		.expect("entryway");
	assert!(
		(entry.max.y - entry.min.y - 2.7).abs() < 0.05,
		"expected full-depth entry, got {entry:?}"
	);
	let touches_body = apt.max_rects.iter().any(|r| {
		shared_edge_span(entry, *r)
			.is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= residential_access().open_touch())
	});
	assert!(touches_body, "entry must share an edge with the body max-rect");
	assert!(
		!apt.max_rects.is_empty()
			|| apt.rooms.iter().any(|r| matches!(
				r,
				ApartmentRoom::Living(_)
					| ApartmentRoom::Kitchen(_)
					| ApartmentRoom::Bedroom(_)
					| ApartmentRoom::OpenHall { .. }
			)),
		"body should pack"
	);
}

#[test]
fn thin_entry_corridor_claimed_not_dropped() {
	let stem = FillRegion::new(
		SpaceKind::InternalSpace,
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.8, 3.0, 8.0)),
			0.0,
			{
				let mut o = Openings::new();
				o.insert(
					OpeningId::new("door"),
					Opening::new(
						Aabb3d::from_min_max(Vec3::new(0.4, 0.0, -0.15), Vec3::new(1.4, 2.2, 0.15)),
						OpeningLabel::Passage,
					),
				);
				o
			},
		),
	);
	let body = FillRegion::new(
		SpaceKind::InternalSpace,
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(1.8, 0.0, 0.0), Vec3::new(8.0, 3.0, 8.0)),
			0.0,
			Openings::new(),
		),
	);
	let (apt, residual) =
		LivableApartment::from_multi(0, &MultiConfines::new([stem, body]), NoiseParams::default())
			.unwrap();
	assert!(
		apt.rooms.iter().any(|r| matches!(r, ApartmentRoom::Entryway { .. })),
		"expected entry on stem"
	);
	assert!(
		apt.rooms.iter().any(|r| matches!(
			r,
			ApartmentRoom::Living(_)
				| ApartmentRoom::Kitchen(_)
				| ApartmentRoom::Bedroom(_)
				| ApartmentRoom::Dining(_)
				| ApartmentRoom::OpenHall { .. }
		)),
		"body should pack, rooms={:?}",
		apt.rooms.len()
	);
	let stem_left = residual.within.iter().any(|f| {
		let xz = host_xz(&f.confines.bounds);
		xz.max.x - xz.min.x < 2.0 && aabb2_area(xz) > 5.0
	});
	assert!(!stem_left, "thin stem should be claimed as entry, not residual");
}
