//! Richmond-shaped slots plus authored extremes for `/show gallery`.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use building_components::{
	BuildingComponents, FurnitureAbutment, FurnitureGeometry, FurnitureNode, Placement,
};
use buildings::{
	CommonBedroom, CommonBedroomParameterized, Confines, Kitchen, KitchenCounterLayout,
	KitchenParameterized, LivingRoom, LivingRoomParameterized, Opening, OpeningId, Openings,
};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

/// One gallery cell: world-space slot (already offset).
#[derive(Clone, Debug)]
pub struct GallerySlot {
	/// Exhibit group (`richmond-bedroom`, `seed-a`, …). Read by gallery tests.
	#[allow(dead_code)]
	pub label: &'static str,
	pub node: FurnitureNode,
}

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

fn offset_node(mut node: FurnitureNode, origin: Vec3) -> FurnitureNode {
	node.placement.translation += origin;
	node
}

fn slot_filling(
	make: fn(Placement) -> FurnitureNode,
	min: Vec3,
	max: Vec3,
	host: &Aabb3d,
) -> FurnitureNode {
	let aabb = Aabb3d::from_min_max(min, max);
	let center = Vec3::from((aabb.min + aabb.max) * 0.5);
	let extent = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	let mut node = make(Placement::new(center, 0.0).with_scale(extent));
	node.stamp_host_slot(&aabb, host);
	node
}

fn typical_slot(
	make: fn(Placement) -> FurnitureNode,
	size: Vec3,
	origin: Vec3,
	seed: u64,
	abutment: Option<FurnitureAbutment>,
) -> FurnitureNode {
	let mut node =
		make(Placement::new(origin + Vec3::new(0.0, size.y * 0.5, 0.0), 0.0).with_scale(size));
	node.finish_seed = seed;
	if let Some(side) = abutment {
		node = node.with_abutment(side);
	}
	node
}

/// Packed Richmond rooms: wall bedroom, kitchen adjacencies, living seating.
///
/// Kitchen row is galley / L / peninsula / island so side-constraint fitting
/// can be checked on corner joins, stubs, and free runs.
pub fn recorded_richmond_slots() -> anyhow::Result<Vec<GallerySlot>> {
	let mut out = Vec::new();

	let bedroom_origin = Vec3::ZERO;
	let mut bedroom_params = CommonBedroomParameterized::with_fill(1.0, 0.4);
	bedroom_params.bed_against_wall = true;
	let (bedroom, _) = CommonBedroom::fit_with_fill(
		&south_door(Vec3::new(6.0, 3.0, 6.0)),
		NoiseParams { seed: 5, ..NoiseParams::default() },
		bedroom_params,
	)
	.map_err(|err| anyhow::anyhow!("bedroom fit: {err}"))?;
	for node in bedroom.furniture_nodes_for_level(LodSceneLevel::High).flatten() {
		out.push(GallerySlot {
			label: "richmond-bedroom",
			node: offset_node(node, bedroom_origin),
		});
	}

	out.extend(pack_kitchen_slots(
		"richmond-kitchen",
		Vec3::new(10.0, 0.0, 0.0),
		Vec3::new(6.0, 3.0, 4.5),
		7,
		KitchenParameterized::with_fill(1.2, 0.4).with_layout(KitchenCounterLayout::Galley),
	)?);
	out.extend(require_kitchen_slots(
		"richmond-kitchen-l",
		Vec3::new(18.0, 0.0, 0.0),
		Vec3::new(7.0, 3.0, 5.5),
		&[11, 3, 5, 13, 17, 19, 23],
		KitchenParameterized::with_fill(1.15, 0.4).with_layout(KitchenCounterLayout::LShape),
		|k| k.counter_runs.len() >= 2,
		"L kitchen should keep two corner runs",
	)?);
	out.extend(require_kitchen_slots(
		"richmond-kitchen-peninsula",
		Vec3::new(28.0, 0.0, 0.0),
		Vec3::new(7.0, 3.0, 5.5),
		&[21, 3, 7, 11, 15, 19, 25],
		KitchenParameterized::with_fill(1.15, 0.4).with_layout(KitchenCounterLayout::Peninsula),
		|k| !k.peninsulas.is_empty(),
		"peninsula kitchen should emit a stub",
	)?);
	out.extend(require_kitchen_slots(
		"richmond-kitchen-island",
		Vec3::new(38.0, 0.0, 0.0),
		Vec3::new(8.0, 3.0, 6.0),
		&[3, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41],
		KitchenParameterized::with_fill(1.1, 0.55).with_layout(KitchenCounterLayout::Galley),
		|k| !k.islands.is_empty(),
		"island kitchen should emit a free run",
	)?);

	let living_origin = Vec3::new(50.0, 0.0, 0.0);
	let (living, _) = LivingRoom::fit_with_fill(
		&south_door(Vec3::new(6.0, 2.8, 4.5)),
		NoiseParams { seed: 11, ..NoiseParams::default() },
		LivingRoomParameterized::with_fill(1.2, 0.4),
	)
	.map_err(|err| anyhow::anyhow!("living fit: {err}"))?;
	for node in living.furniture_nodes_for_level(LodSceneLevel::High).flatten() {
		out.push(GallerySlot { label: "richmond-living", node: offset_node(node, living_origin) });
	}

	Ok(out)
}

fn pack_kitchen_slots(
	label: &'static str,
	origin: Vec3,
	extent: Vec3,
	seed: i32,
	params: KitchenParameterized,
) -> anyhow::Result<Vec<GallerySlot>> {
	let (kitchen, _) = Kitchen::fit_with_fill(
		&south_door(extent),
		NoiseParams { seed, ..NoiseParams::default() },
		params,
	)
	.map_err(|err| anyhow::anyhow!("{label} fit: {err}"))?;
	Ok(kitchen_to_slots(label, origin, &kitchen))
}

fn require_kitchen_slots(
	label: &'static str,
	origin: Vec3,
	extent: Vec3,
	seeds: &[i32],
	params: KitchenParameterized,
	ok: fn(&Kitchen) -> bool,
	why: &str,
) -> anyhow::Result<Vec<GallerySlot>> {
	let mut last = String::new();
	for &seed in seeds {
		let (kitchen, _) = Kitchen::fit_with_fill(
			&south_door(extent),
			NoiseParams { seed, ..NoiseParams::default() },
			params.clone(),
		)
		.map_err(|err| anyhow::anyhow!("{label} seed={seed} fit: {err}"))?;
		if ok(&kitchen) {
			return Ok(kitchen_to_slots(label, origin, &kitchen));
		}
		last = format!(
			"{label} seed={seed} runs={} pen={} islands={} layout={:?}",
			kitchen.counter_runs.len(),
			kitchen.peninsulas.len(),
			kitchen.islands.len(),
			kitchen.counter_layout
		);
	}
	Err(anyhow::anyhow!("{why} ({last})"))
}

fn kitchen_to_slots(label: &'static str, origin: Vec3, kitchen: &Kitchen) -> Vec<GallerySlot> {
	kitchen
		.furniture_nodes_for_level(LodSceneLevel::High)
		.flatten()
		.into_iter()
		.map(|node| GallerySlot { label, node: offset_node(node, origin) })
		.collect()
}

/// Authored extremes: flat bed, tall chair, long counter, wall vs free, two seeds.
pub fn authored_extreme_slots() -> Vec<GallerySlot> {
	let host = Aabb3d::from_min_max(Vec3::new(-20.0, 0.0, 10.0), Vec3::new(20.0, 3.0, 22.0));
	vec![
		GallerySlot {
			label: "flat-bed",
			node: typical_slot(
				FurnitureNode::bed,
				Vec3::new(2.0, 0.18, 1.6),
				Vec3::new(0.0, 0.0, 14.0),
				11,
				Some(FurnitureAbutment::NegZ),
			),
		},
		GallerySlot {
			label: "tall-chair",
			node: typical_slot(
				FurnitureNode::chair,
				Vec3::new(0.42, 1.35, 0.42),
				Vec3::new(4.0, 0.0, 14.0),
				13,
				None,
			),
		},
		GallerySlot {
			label: "long-counter",
			node: slot_filling(
				FurnitureNode::counter,
				Vec3::new(7.0, 0.0, 13.7),
				Vec3::new(11.2, 0.9, 14.3),
				&host,
			),
		},
		GallerySlot {
			label: "wall-bed",
			node: typical_slot(
				FurnitureNode::bed,
				Vec3::new(2.0, 0.55, 1.6),
				Vec3::new(14.0, 0.0, 14.0),
				17,
				Some(FurnitureAbutment::PosZ),
			),
		},
		GallerySlot {
			label: "free-bed",
			node: typical_slot(
				FurnitureNode::bed,
				Vec3::new(2.0, 0.55, 1.6),
				Vec3::new(18.0, 0.0, 14.0),
				17,
				None,
			),
		},
		GallerySlot {
			label: "seed-a",
			node: typical_slot(
				FurnitureNode::chest,
				Vec3::new(0.9, 0.7, 0.5),
				Vec3::new(0.0, 0.0, 18.0),
				1,
				Some(FurnitureAbutment::NegZ),
			),
		},
		GallerySlot {
			label: "seed-b",
			node: typical_slot(
				FurnitureNode::chest,
				Vec3::new(0.9, 0.7, 0.5),
				Vec3::new(2.4, 0.0, 18.0),
				99,
				Some(FurnitureAbutment::NegZ),
			),
		},
		chest_skin_slot(
			"chest-rockadder",
			furniture_assemblies::palette::ChestKind::Rockadder,
			Vec3::new(5.0, 0.0, 18.0),
		),
		chest_skin_slot(
			"chest-cosimo",
			furniture_assemblies::palette::ChestKind::Cosimo,
			Vec3::new(7.4, 0.0, 18.0),
		),
		chest_skin_slot(
			"chest-foliage",
			furniture_assemblies::palette::ChestKind::Foliage,
			Vec3::new(9.8, 0.0, 18.0),
		),
		chest_skin_slot(
			"chest-terrain",
			furniture_assemblies::palette::ChestKind::Terrain,
			Vec3::new(12.2, 0.0, 18.0),
		),
	]
}

fn chest_skin_slot(
	label: &'static str,
	skin: furniture_assemblies::palette::ChestKind,
	origin: Vec3,
) -> GallerySlot {
	let seed = furniture_assemblies::palette::first_seed_for_chest(skin, 128).unwrap_or(0);
	GallerySlot {
		label,
		node: typical_slot(
			FurnitureNode::chest,
			Vec3::new(0.9, 0.7, 0.5),
			origin,
			seed,
			Some(FurnitureAbutment::NegZ),
		),
	}
}

/// Richmond rooms plus authored extremes.
pub fn gallery_slots() -> anyhow::Result<Vec<GallerySlot>> {
	let mut slots = recorded_richmond_slots()?;
	slots.extend(authored_extreme_slots());
	Ok(slots)
}

/// Typical unit-assembly slot used by `/show bed|chair|chest|counter`.
pub fn unit_slot(geometry: FurnitureGeometry, seed: u64) -> FurnitureNode {
	let (make, size): (fn(Placement) -> FurnitureNode, Vec3) = match geometry {
		FurnitureGeometry::Bed => (FurnitureNode::bed, Vec3::new(2.0, 0.55, 1.6)),
		FurnitureGeometry::Chair => (FurnitureNode::chair, Vec3::new(0.52, 1.10, 0.52)),
		FurnitureGeometry::Table => (FurnitureNode::table, Vec3::new(2.20, 1.00, 1.40)),
		FurnitureGeometry::Chest => (FurnitureNode::chest, Vec3::new(0.9, 0.7, 0.5)),
		FurnitureGeometry::Counter => (FurnitureNode::counter, Vec3::new(1.8, 0.9, 0.6)),
		FurnitureGeometry::FoodDisplay => {
			(FurnitureNode::food_display, Vec3::new(0.70, 0.42, 0.50))
		}
		FurnitureGeometry::Fruit => (FurnitureNode::fruit, Vec3::new(0.28, 0.22, 0.28)),
		FurnitureGeometry::Bread => (FurnitureNode::bread, Vec3::new(0.36, 0.18, 0.22)),
		FurnitureGeometry::Cookware => (FurnitureNode::cookware, Vec3::new(0.54, 0.30, 0.54)),
		FurnitureGeometry::Basin => (FurnitureNode::basin, Vec3::new(0.82, 0.32, 0.64)),
		FurnitureGeometry::Faucet => (FurnitureNode::faucet, Vec3::new(0.24, 0.44, 0.22)),
		FurnitureGeometry::Shelf => (FurnitureNode::shelf, Vec3::new(1.4, 0.40, 0.40)),
		FurnitureGeometry::Range => (FurnitureNode::range, Vec3::new(1.10, 1.00, 0.75)),
		FurnitureGeometry::Fridge => (FurnitureNode::fridge, Vec3::new(1.90, 2.50, 1.10)),
		FurnitureGeometry::Partition => (FurnitureNode::partition, Vec3::new(2.60, 3.20, 0.22)),
		_ => (FurnitureNode::bed, Vec3::ONE),
	};
	typical_slot(make, size, Vec3::ZERO, seed, Some(FurnitureAbutment::NegZ))
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::FurnitureAbutment;
	use furniture_assemblies::{try_assembly, PartKind};

	#[test]
	fn richmond_rooms_emit_expected_kinds() -> anyhow::Result<()> {
		let slots = recorded_richmond_slots()?;
		let bedroom_beds: Vec<_> = slots
			.iter()
			.filter(|s| s.label == "richmond-bedroom" && s.node.geometry == FurnitureGeometry::Bed)
			.collect();
		if bedroom_beds.is_empty() {
			return Err(anyhow::anyhow!("bedroom packer should emit a bed"));
		}
		if bedroom_beds.iter().any(|s| s.node.abutment.is_none()) {
			return Err(anyhow::anyhow!("wall bedroom should stamp abutment"));
		}
		if !slots
			.iter()
			.any(|s| s.label == "richmond-kitchen" && s.node.geometry == FurnitureGeometry::Counter)
		{
			return Err(anyhow::anyhow!("galley kitchen should emit a counter"));
		}
		let l_runs: Vec<_> = slots
			.iter()
			.filter(|s| {
				s.label == "richmond-kitchen-l" && s.node.geometry == FurnitureGeometry::Counter
			})
			.collect();
		if l_runs.iter().filter(|s| s.node.abutment.is_some()).count() < 2 {
			return Err(anyhow::anyhow!("L kitchen should emit two wall-flush counter runs"));
		}
		if !slots.iter().any(|s| {
			s.label == "richmond-kitchen-peninsula" && s.node.geometry == FurnitureGeometry::Counter
		}) {
			return Err(anyhow::anyhow!("peninsula kitchen should emit counters"));
		}
		if !slots.iter().any(|s| {
			s.label == "richmond-kitchen-island"
				&& s.node.geometry == FurnitureGeometry::Counter
				&& s.node.abutment.is_none()
		}) {
			return Err(anyhow::anyhow!("island kitchen should emit a free counter"));
		}
		if !slots
			.iter()
			.any(|s| s.label == "richmond-living" && s.node.geometry == FurnitureGeometry::Chair)
		{
			return Err(anyhow::anyhow!("living room should emit a chair"));
		}
		Ok(())
	}

	#[test]
	fn two_seeds_share_topology() -> anyhow::Result<()> {
		let slots = authored_extreme_slots();
		let a = slots
			.iter()
			.find(|s| s.label == "seed-a")
			.ok_or_else(|| anyhow::anyhow!("missing seed-a"))?;
		let b = slots
			.iter()
			.find(|s| s.label == "seed-b")
			.ok_or_else(|| anyhow::anyhow!("missing seed-b"))?;
		let pa = try_assembly(&a.node).ok_or_else(|| anyhow::anyhow!("seed-a paint"))?;
		let pb = try_assembly(&b.node).ok_or_else(|| anyhow::anyhow!("seed-b paint"))?;
		if pa.parts.len() != pb.parts.len() {
			return Err(anyhow::anyhow!("seed topology length diverged"));
		}
		let mut paint_changed = false;
		for (left, right) in pa.parts.iter().zip(&pb.parts) {
			if left.kind != right.kind || left.placement != right.placement {
				return Err(anyhow::anyhow!("seed changed topology"));
			}
			paint_changed |= left.material != right.material;
		}
		if !paint_changed {
			return Err(anyhow::anyhow!("two seeds should pick different paint"));
		}
		Ok(())
	}

	#[test]
	fn authored_chests_include_rockadder_cosimo_foliage_terrain() -> anyhow::Result<()> {
		let slots = authored_extreme_slots();
		for (label, want) in [
			("chest-rockadder", furniture_shaders::RECIPE_FURNITURE_ROCKADDER),
			("chest-cosimo", furniture_shaders::RECIPE_FURNITURE_COSMOS),
			("chest-foliage", furniture_assemblies::palette::RECIPE_FOLIAGE),
			("chest-terrain", furniture_shaders::RECIPE_FURNITURE_MARBLE),
		] {
			let slot = slots
				.iter()
				.find(|s| s.label == label)
				.ok_or_else(|| anyhow::anyhow!("missing {label}"))?;
			let assembly =
				try_assembly(&slot.node).ok_or_else(|| anyhow::anyhow!("{label} paint"))?;
			let trunk = assembly
				.parts
				.iter()
				.find(|p| p.kind == PartKind::ChestTrunk)
				.ok_or_else(|| anyhow::anyhow!("{label} trunk"))?;
			match &trunk.material.name {
				material_ref::MaterialId::Name(name) if name == want => {}
				other => return Err(anyhow::anyhow!("{label} should be {want}, got {other:?}")),
			}
		}
		Ok(())
	}

	#[test]
	fn unit_chair_back_faces_the_wall() -> anyhow::Result<()> {
		let slot = unit_slot(FurnitureGeometry::Chair, 4);
		if slot.abutment != Some(FurnitureAbutment::NegZ) {
			return Err(anyhow::anyhow!("unit chair should abut −Z"));
		}
		let assembly = try_assembly(&slot).ok_or_else(|| anyhow::anyhow!("chair paint"))?;
		if assembly.parts.iter().all(|p| p.kind != PartKind::ChairBack) {
			return Err(anyhow::anyhow!("missing back"));
		}
		if (slot.placement.yaw - FurnitureAbutment::NegZ.facing_yaw()).abs() > 1e-5 {
			return Err(anyhow::anyhow!(
				"unit chair yaw should be NegZ facing, got {}",
				slot.placement.yaw
			));
		}
		Ok(())
	}
}
