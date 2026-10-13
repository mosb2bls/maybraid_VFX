//! Gallery-scale smoke fits for livable quarters.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use procedural_common::NoiseParams;

use crate::fit::Confines;
use crate::openings::{Opening, OpeningId, Openings};
use crate::usage_areas::livable_quarters::{
	DiningRoom, DiningRoomParameterized, DiningRoomPlan, Kitchen, KitchenCounterLayout,
	KitchenParameterized, LivingRoom, LivingRoomParameterized, SittingRoom,
	SittingRoomParameterized,
};
use building_components::FurnitureGeometry;

fn south_door(extent: Vec3) -> Confines {
	let mut openings = Openings::new();
	let w = (extent.x * 0.25).clamp(0.8, 1.2);
	let x0 = (extent.x - w) * 0.5;
	openings.insert(
		OpeningId::new("door"),
		Opening::passage(Aabb3d::from_min_max(
			Vec3::new(x0, 0.0, -0.15),
			Vec3::new(x0 + w, 2.1, 0.15),
		)),
	);
	Confines::new(Aabb3d::from_min_max(Vec3::ZERO, extent), 0.0, openings)
}

#[test]
fn living_and_sitting_gallery_cells_fit() {
	let living = [
		(Vec3::new(5.0, 2.8, 4.0), 7, 1.1, 0.35),
		(Vec3::new(6.0, 2.8, 4.5), 11, 1.2, 0.4),
		(Vec3::new(9.0, 3.0, 7.0), 99, 1.4, 0.4),
	];
	for (extent, seed, sp, occ) in living {
		let (room, _) = LivingRoom::fit_with_fill(
			&south_door(extent),
			NoiseParams { seed, ..NoiseParams::default() },
			LivingRoomParameterized::with_fill(sp, occ),
		)
		.unwrap_or_else(|e| panic!("living {extent:?} seed={seed}: {e}"));
		assert!(!room.primary_seating.is_empty());
		assert!(room
			.primary_seating
			.iter()
			.all(|f| f.furniture.geometry == FurnitureGeometry::Chair));
	}
	let sitting = [
		(Vec3::new(4.0, 2.8, 3.5), 7, 1.1, 0.4),
		(Vec3::new(5.0, 2.8, 4.0), 11, 1.2, 0.38),
		(Vec3::new(7.0, 3.0, 5.5), 99, 1.4, 0.38),
	];
	for (extent, seed, sp, occ) in sitting {
		let (room, _) = SittingRoom::fit_with_fill(
			&south_door(extent),
			NoiseParams { seed, ..NoiseParams::default() },
			SittingRoomParameterized::with_fill(sp, occ),
		)
		.unwrap_or_else(|e| panic!("sitting {extent:?} seed={seed}: {e}"));
		assert!(!room.primary_seating.is_empty());
		assert!(room
			.primary_seating
			.iter()
			.all(|f| f.furniture.geometry == FurnitureGeometry::Chair));
	}
}

#[test]
fn kitchen_layouts_and_thin_dining_fit() {
	for (layout, seed) in [
		(KitchenCounterLayout::Galley, 7),
		(KitchenCounterLayout::LShape, 11),
		(KitchenCounterLayout::Peninsula, 21),
	] {
		let (room, _) = Kitchen::fit_with_fill(
			&south_door(Vec3::new(6.0, 3.0, 4.5)),
			NoiseParams { seed, ..NoiseParams::default() },
			KitchenParameterized::with_fill(1.2, 0.4).with_layout(layout),
		)
		.unwrap_or_else(|e| panic!("kitchen {layout:?}: {e}"));
		assert!(!room.counter_runs.is_empty());
		assert!(room
			.counter_runs
			.iter()
			.all(|f| f.furniture.geometry == FurnitureGeometry::Counter));
		assert!(room
			.peninsulas
			.iter()
			.all(|f| f.furniture.geometry == FurnitureGeometry::Counter));
		assert!(room.islands.iter().all(|f| f.furniture.geometry == FurnitureGeometry::Counter));
		assert!(room.counter_runs.iter().any(|f| f.furniture.abutment.is_some()));
		if layout == KitchenCounterLayout::LShape {
			assert!(
				room.counter_runs.len() >= 2 || room.counter_layout == KitchenCounterLayout::Galley,
				"L should place two runs or soft-fall to galley"
			);
		}
	}
	for (extent, seed) in [
		(Vec3::new(8.0, 3.0, 3.2), 42),
		(Vec3::new(9.5, 3.0, 3.0), 55),
		(Vec3::new(11.0, 3.0, 3.4), 99),
	] {
		let (room, _) = DiningRoom::fit_with_fill(
			&south_door(extent),
			NoiseParams { seed, ..NoiseParams::default() },
			DiningRoomParameterized::with_fill(1.25, 0.4),
		)
		.unwrap_or_else(|e| panic!("dining {extent:?}: {e}"));
		assert!(!room.tables.is_empty());
	}
	// Roomy cell: table should grow well past the old ~2.6 m hard cap.
	let confines = south_door(Vec3::new(10.0, 3.0, 8.0));
	let plan = DiningRoomPlan::from_parameterized(
		DiningRoomParameterized::with_fill(1.3, 0.45),
		&confines,
		NoiseParams { seed: 3, ..NoiseParams::default() },
	)
	.expect("roomy dining");
	let table = plan.packed.tables.first().expect("table");
	let e = table.max - table.min;
	let long = e.x.max(e.z);
	assert!(long + 1e-3 >= 3.2, "expected host-scaled table long axis >= 3.2, got {long}");
}
