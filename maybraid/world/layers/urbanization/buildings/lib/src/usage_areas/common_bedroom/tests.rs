use super::*;
use crate::fit::Fit;
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::usage_areas::clearance::PASSAGE_CLEARANCE;
use crate::usage_areas::livable_quarters::ResidentialBathroom;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::furniture::FurnitureAbutment;
use building_components::FurnitureGeometry;
use procedural_common::{aabb3_to_plan, PlanAxes};

fn roomy_south() -> Confines {
	let mut openings = Openings::new();
	openings.insert(
		OpeningId::new("door_a"),
		Opening::passage(Aabb3d::from_min_max(Vec3::new(1.5, 0.0, -0.2), Vec3::new(2.5, 2.2, 0.2))),
	);
	Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.0, 6.0)), 0.0, openings)
}

#[test]
fn common_bedroom_places_bed_and_tracks_partition_doors() {
	let confines = roomy_south();
	let (room, regions) = CommonBedroom::fit_with_fill(
		&confines,
		NoiseParams { seed: 7, ..NoiseParams::default() },
		CommonBedroomParameterized::with_fill(1.0, 0.7),
	)
	.unwrap();
	assert_eq!(room.room_type.text.as_str(), "CommonBedroom");
	assert!(!room.beds.is_empty());
	assert!(room.beds.iter().all(|b| b.label.text.as_str() == "Bed"));
	assert!(room.nightstands.iter().all(|n| n.label.text.as_str() == "Nightstand"));
	assert!(room
		.small_bedroom_furniture
		.iter()
		.all(|s| s.label.text.as_str() == "SmallBedroomFurniture"));
	assert!(room.wardrobes.iter().all(|w| w.label.text.as_str() == "Wardrobe"));
	assert!(room.dressers.iter().all(|d| d.label.text.as_str() == "Dresser"));
	assert!(room
		.bedroom_furniture
		.iter()
		.all(|b| b.label.text.as_str() == "BedroomFurniture"));
	assert!(room.walk_in_closets.iter().all(|c| c.label.text.as_str() == "WalkInCloset"));
	assert_eq!(
		regions.within.len(),
		room.closets.len() + room.walk_in_closets.len() + room.ensuites.len()
	);
	for c in &room.closets {
		assert!(matches!(c.door.label, OpeningLabel::Passage));
		assert!(c.door_id.0.contains("closet_door"));
	}
	assert!(room.ensuites.len() <= 1);
	for e in &room.ensuites {
		assert!(matches!(e.door.label, OpeningLabel::Passage));
		assert!(e.door_id.0.contains("ensuite_door"));
	}
	if !room.closets.is_empty() {
		assert!(!room.closet_walls.is_empty());
	}
	if !room.ensuites.is_empty() {
		assert!(!room.ensuite_walls.is_empty());
	}
}

#[test]
fn common_bedroom_at_most_one_ensuite() {
	let confines =
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.0, 10.0)), 0.0, {
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::new("door_a"),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(4.0, 0.0, -0.2),
					Vec3::new(6.0, 2.2, 0.2),
				)),
			);
			openings
		});
	for seed in 0..20 {
		let (room, _) = CommonBedroom::fit_with_fill(
			&confines,
			NoiseParams { seed, ..NoiseParams::default() },
			CommonBedroomParameterized::with_fill(1.0, 0.9),
		)
		.unwrap();
		assert!(room.ensuites.len() <= 1, "seed={seed} placed {} ensuites", room.ensuites.len());
	}
}

#[test]
fn common_bedroom_can_place_wardrobe_or_dresser() {
	let confines =
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.0, 10.0)), 0.0, {
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::new("door_a"),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(4.0, 0.0, -0.2),
					Vec3::new(6.0, 2.2, 0.2),
				)),
			);
			openings
		});
	let mut saw_storage = false;
	for seed in 0..48 {
		let (room, _) = CommonBedroom::fit_with_fill(
			&confines,
			NoiseParams { seed, ..NoiseParams::default() },
			CommonBedroomParameterized::with_fill(1.25, 0.85),
		)
		.unwrap();
		if !room.wardrobes.is_empty() || !room.dressers.is_empty() {
			saw_storage = true;
			break;
		}
	}
	assert!(saw_storage, "expected some seed to place a wardrobe or dresser");
}

#[test]
fn common_bedroom_storage_long_face_on_wall_and_sep() {
	use procedural_common::{inflate_aabb2, intersects_aabb2};
	let confines =
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(12.0, 3.0, 12.0)), 0.0, {
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::new("door_a"),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(5.0, 0.0, -0.2),
					Vec3::new(7.0, 2.2, 0.2),
				)),
			);
			openings
		});
	let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
	let mut saw_storage = false;
	for seed in 0..80 {
		let plan = CommonBedroomPlan::from_parameterized(
			CommonBedroomParameterized::with_fill(1.25, 0.85),
			&confines,
			NoiseParams { seed, ..NoiseParams::default() },
		)
		.unwrap();
		let storage: Vec<_> = plan
			.packed
			.wardrobes
			.iter()
			.chain(plan.packed.dressers.iter())
			.cloned()
			.collect();
		if !storage.is_empty() {
			saw_storage = true;
		}
		for s in &storage {
			let p = aabb3_to_plan(s, PlanAxes::XZ);
			let w = p.max.x - p.min.x;
			let d = p.max.y - p.min.y;
			const EPS: f32 = 0.08;
			let long_on_wall = if w + EPS >= d {
				(p.min.y - host.min.y).abs() < EPS || (p.max.y - host.max.y).abs() < EPS
			} else {
				(p.min.x - host.min.x).abs() < EPS || (p.max.x - host.max.x).abs() < EPS
			};
			assert!(long_on_wall, "storage long face not on host wall (seed={seed})");
		}
		if storage.len() >= 2 {
			let a = aabb3_to_plan(&storage[0], PlanAxes::XZ);
			let b = aabb3_to_plan(&storage[1], PlanAxes::XZ);
			assert!(
				!intersects_aabb2(inflate_aabb2(a, 1.0 - 1e-3), b),
				"storage pieces closer than 1m (seed={seed})"
			);
		}
		assert!(plan.packed.wardrobes.len() <= 1);
		assert!(plan.packed.dressers.len() <= 1);
	}
	assert!(saw_storage, "expected some seed with wardrobe or dresser");
}

#[test]
fn common_bedroom_nightstands_abut_beds() {
	use procedural_common::{inflate_aabb2, touches_aabb2};
	let confines = roomy_south();
	let params = CommonBedroomParameterized::with_fill(1.0, 0.75);
	let plan = CommonBedroomPlan::from_parameterized(
		params,
		&confines,
		NoiseParams { seed: 7, ..NoiseParams::default() },
	)
	.unwrap();
	for ns in &plan.packed.nightstands {
		let n = aabb3_to_plan(ns, PlanAxes::XZ);
		let abuts_long_side = plan.packed.beds.iter().any(|bed| {
			let b = aabb3_to_plan(bed, PlanAxes::XZ);
			if !touches_aabb2(n, inflate_aabb2(b, 0.2)) {
				return false;
			}
			let bed_w = b.max.x - b.min.x;
			let bed_d = b.max.y - b.min.y;
			const EPS: f32 = 0.15;
			let nc = (n.min + n.max) * 0.5;
			if bed_w + 1e-3 >= bed_d {
				nc.x > b.min.x - EPS
					&& nc.x < b.max.x + EPS
					&& (nc.y > b.max.y - EPS || nc.y < b.min.y + EPS)
			} else {
				nc.y > b.min.y - EPS
					&& nc.y < b.max.y + EPS
					&& (nc.x > b.max.x - EPS || nc.x < b.min.x + EPS)
			}
		});
		assert!(abuts_long_side, "nightstand not on a long side of any bed");
	}
}

#[test]
fn common_bedroom_bed_against_wall_prefers_host_edge() {
	let confines = roomy_south();
	let mut params = CommonBedroomParameterized::with_fill(1.0, 0.4);
	params.bed_against_wall = true;
	let plan = CommonBedroomPlan::from_parameterized(
		params,
		&confines,
		NoiseParams { seed: 5, ..NoiseParams::default() },
	)
	.unwrap();
	let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
	let bed = aabb3_to_plan(&plan.packed.beds[0], PlanAxes::XZ);
	const EPS: f32 = 0.08;
	let against = (bed.min.x - host.min.x).abs() < EPS
		|| (bed.max.x - host.max.x).abs() < EPS
		|| (bed.min.y - host.min.y).abs() < EPS
		|| (bed.max.y - host.max.y).abs() < EPS;
	assert!(against, "bed_against_wall did not flush bed to a host wall");

	let packed_bed = plan.packed.beds[0];
	let expected = FurnitureAbutment::from_flush(&packed_bed, &confines.bounds)
		.expect("packed bed still flushes the host");
	let room = CommonBedroom::from_plan(plan, &confines);
	let slot = &room.beds[0].furniture;
	assert_eq!(slot.geometry, FurnitureGeometry::Bed);
	let abutment = slot.abutment.expect("wall-flush bed should stamp abutment");
	assert_eq!(abutment, expected);
	assert!((slot.placement.yaw - abutment.facing_yaw()).abs() < 1e-5);
	assert_ne!(slot.finish_seed, 0);
}

#[test]
fn common_bedroom_partitions_keep_sep_gap() {
	use procedural_common::inflate_aabb2;
	let confines =
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(12.0, 3.0, 12.0)), 0.0, {
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::new("door_a"),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(5.0, 0.0, -0.2),
					Vec3::new(7.0, 2.2, 0.2),
				)),
			);
			openings
		});
	let params = CommonBedroomParameterized::with_fill(1.0, 0.9);
	let plan = CommonBedroomPlan::from_parameterized(
		params,
		&confines,
		NoiseParams { seed: 3, ..NoiseParams::default() },
	)
	.unwrap();
	let parts: Vec<_> = plan
		.packed
		.closets
		.iter()
		.chain(plan.packed.walk_in_closets.iter())
		.chain(plan.packed.ensuites.iter())
		.map(|p| aabb3_to_plan(&p.bounds, PlanAxes::XZ))
		.collect();
	for i in 0..parts.len() {
		for j in (i + 1)..parts.len() {
			let halo = inflate_aabb2(parts[i], 1.0 - 1e-3);
			assert!(
				!procedural_common::intersects_aabb2(halo, parts[j]),
				"partitions {i} and {j} closer than 1m"
			);
		}
	}
}

#[test]
fn common_bedroom_ensuite_grows_toward_area_target() {
	use procedural_common::aabb2_area;
	let confines =
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(14.0, 3.2, 12.0)), 0.0, {
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::new("door_a"),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(6.0, 0.0, -0.2),
					Vec3::new(8.0, 2.2, 0.2),
				)),
			);
			openings
		});
	let mut params = CommonBedroomParameterized::with_fill(1.25, 0.7);
	params.ensuite_area_target = 70.0;
	params.bedroom_area_reserve = 40.0;
	params.bed_against_wall = true;
	let host_area = 14.0 * 12.0;
	let mut found = None;
	for seed in 0..40 {
		let plan = CommonBedroomPlan::from_parameterized(
			params.clone(),
			&confines,
			NoiseParams { seed, ..NoiseParams::default() },
		)
		.unwrap();
		if let Some(ensuite) = plan.packed.ensuites.first() {
			let plan2 = aabb3_to_plan(&ensuite.bounds, PlanAxes::XZ);
			found = Some((aabb2_area(plan2), plan2));
			break;
		}
	}
	let (area, bounds) = found.expect("expected an ensuite in a large room");
	assert!(area + 1e-3 >= 2.6 * 1.8, "ensuite area {area} below enlarged mins");
	assert!(
		area + 0.5 >= host_area * 0.12,
		"ensuite area {area} should grow past mins in host {host_area}"
	);
	assert!(
		area <= host_area * 0.25 + 2.0,
		"ensuite area {area} should stay within the half-axis envelope of host {host_area}"
	);
	let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
	let ew = bounds.max.x - bounds.min.x;
	let ed = bounds.max.y - bounds.min.y;
	let hw = host.max.x - host.min.x;
	let hd = host.max.y - host.min.y;
	assert!(ew <= hw * 0.5 + 1e-2, "ensuite X span {ew} exceeds half host width {hw}");
	assert!(ed <= hd * 0.5 + 1e-2, "ensuite Z span {ed} exceeds half host depth {hd}");
}

#[test]
fn common_bedroom_large_room_prefers_enclosure_before_filler() {
	let confines =
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(12.0, 3.2, 12.0)), 0.0, {
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::new("door_a"),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(5.0, 0.0, -0.2),
					Vec3::new(7.0, 2.2, 0.2),
				)),
			);
			openings
		});
	let mut saw_enclosure = false;
	let mut saw_walk_in_or_furniture = false;
	for seed in 0..64 {
		let (room, _) = CommonBedroom::fit_with_fill(
			&confines,
			NoiseParams { seed, ..NoiseParams::default() },
			CommonBedroomParameterized::with_fill(1.3, 0.7),
		)
		.unwrap();
		if !room.ensuites.is_empty() || !room.closets.is_empty() || !room.walk_in_closets.is_empty()
		{
			saw_enclosure = true;
		}
		if !room.walk_in_closets.is_empty() || !room.bedroom_furniture.is_empty() {
			saw_walk_in_or_furniture = true;
		}
		if !room.bedroom_furniture.is_empty() {
			assert!(
				!room.ensuites.is_empty()
					|| !room.closets.is_empty()
					|| !room.walk_in_closets.is_empty(),
				"BedroomFurniture without enclosure (seed={seed})"
			);
			assert!(room.bedroom_furniture.len() <= 1);
		}
	}
	assert!(saw_enclosure, "expected enclosure in a large room");
	assert!(saw_walk_in_or_furniture, "expected WalkInCloset or BedroomFurniture across seeds");
}

#[test]
fn common_bedroom_avoids_passage_clearance() {
	let confines = roomy_south();
	let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
	let faces = crate::usage_areas::PassageClearance::collect_faces(&confines, host);
	let bands = crate::usage_areas::PassageClearance::bands_std(host, &faces);
	assert!(!bands.is_empty());
	let params = CommonBedroomParameterized::with_fill(1.0, 0.55);
	let plan = CommonBedroomPlan::from_parameterized(
		params,
		&confines,
		NoiseParams { seed: 42, ..NoiseParams::default() },
	)
	.unwrap();
	assert!(!plan.packed.beds.is_empty());
	for bed in &plan.packed.beds {
		let p = aabb3_to_plan(bed, PlanAxes::XZ);
		for band in &bands {
			assert!(
				!procedural_common::intersects_aabb2(p, *band),
				"bed intersects passage clearance (depth ~{PASSAGE_CLEARANCE})"
			);
		}
	}
}

#[test]
fn common_bedroom_furniture_avoids_partition_door_clear() {
	use procedural_common::{inflate_aabb2, intersects_aabb2};
	let confines =
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(12.0, 3.2, 12.0)), 0.0, {
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::new("door_a"),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(5.0, 0.0, -0.2),
					Vec3::new(7.0, 2.2, 0.2),
				)),
			);
			openings
		});
	let mut saw_partition = false;
	for seed in 0..48 {
		let plan = CommonBedroomPlan::from_parameterized(
			CommonBedroomParameterized::with_fill(1.3, 0.7),
			&confines,
			NoiseParams { seed, ..NoiseParams::default() },
		)
		.unwrap();
		let parts: Vec<_> = plan
			.packed
			.closets
			.iter()
			.chain(plan.packed.walk_in_closets.iter())
			.chain(plan.packed.ensuites.iter())
			.collect();
		if parts.is_empty() {
			continue;
		}
		saw_partition = true;
		let furniture: Vec<_> = plan
			.packed
			.beds
			.iter()
			.chain(plan.packed.nightstands.iter())
			.chain(plan.packed.small_bedroom_furniture.iter())
			.chain(plan.packed.wardrobes.iter())
			.chain(plan.packed.dressers.iter())
			.chain(plan.packed.bedroom_furniture.iter())
			.map(|a| aabb3_to_plan(a, PlanAxes::XZ))
			.collect();
		for part in parts {
			let approach = inflate_aabb2(part.door_clear, 0.5);
			for furn in &furniture {
				assert!(
					!intersects_aabb2(approach, *furn),
					"furniture intersects partition door approach (seed={seed})"
				);
			}
		}
	}
	assert!(saw_partition, "expected some seed to place a partition");
}

#[test]
fn common_bedroom_soft_fails_tiny_cell() {
	let confines =
		Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(1.5, 2.5, 1.5)));
	assert!(matches!(
		CommonBedroom::fit_to_confines(&confines, NoiseParams::default()),
		Err(FitError::TooSmall { .. })
	));
}

#[test]
fn common_bedroom_gallery_like_rooms_place_enclosures() {
	let specs: [(Vec3, i32, f32, f32); 5] = [
		(Vec3::new(6.5, 3.0, 6.5), 7, 1.2, 0.55),
		(Vec3::new(8.0, 3.0, 8.0), 42, 1.25, 0.55),
		(Vec3::new(11.0, 3.2, 10.0), 3, 1.35, 0.5),
		(Vec3::new(12.0, 3.2, 12.0), 17, 1.4, 0.45),
		(Vec3::new(14.0, 3.2, 11.0), 33, 1.45, 0.55),
	];
	for (extent, seed, space, occ) in specs {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door_a"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(extent.x * 0.35, 0.0, -0.2),
				Vec3::new(extent.x * 0.65, 2.2, 0.2),
			)),
		);
		let confines = Confines::new(Aabb3d::from_min_max(Vec3::ZERO, extent), 0.0, openings);
		let mut params = CommonBedroomParameterized::with_fill(space, occ);
		params.bed_against_wall = true;
		let (room, _) = CommonBedroom::fit_with_fill(
			&confines,
			NoiseParams { seed, ..NoiseParams::default() },
			params,
		)
		.unwrap();
		let enclosure = room.closets.len() + room.walk_in_closets.len() + room.ensuites.len();
		assert!(
			enclosure > 0,
			"expected closet/walk-in/ensuite in {extent:?} seed={seed} (space={space} occ={occ})"
		);
	}
}

#[test]
fn common_bedroom_ensuite_within_fits_residential_bathroom() {
	let confines =
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(14.0, 3.2, 12.0)), 0.0, {
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::new("door_a"),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(6.0, 0.0, -0.2),
					Vec3::new(8.0, 2.2, 0.2),
				)),
			);
			openings
		});
	let mut params = CommonBedroomParameterized::with_fill(1.25, 0.7);
	params.ensuite_area_target = 70.0;
	params.bedroom_area_reserve = 40.0;
	params.bed_against_wall = true;

	let mut found = false;
	for seed in 0..48 {
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		let (room, regions) =
			CommonBedroom::fit_with_fill(&confines, noise, params.clone()).unwrap();
		if room.ensuites.is_empty() {
			continue;
		}
		found = true;
		assert_eq!(
			room.ensuite_bathrooms.len(),
			room.ensuites.len(),
			"seed={seed}: expected bathroom for each ensuite"
		);
		for ensuite in &room.ensuites {
			let region = regions
				.within
				.iter()
				.find(|r| r.confines.openings.get(&ensuite.door_id).is_some());
			let region = region.expect("missing within region for ensuite door");
			assert!(
				region.confines.openings.get(&ensuite.door_id).is_some(),
				"ensuite door id not preserved on within confines"
			);
			ResidentialBathroom::fit_to_confines(&region.confines, noise).unwrap();
		}
		break;
	}
	assert!(found, "expected at least one seed with an ensuite");
}
