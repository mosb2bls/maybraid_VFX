//! Map a stamped Richmond slot to a painted assembly, when a kit exists.

use building_components::{FurnitureGeometry, FurnitureNode};

use crate::basin::BasinParams;
use crate::bed::BedParams;
use crate::bread::BreadParams;
use crate::chair::ChairParams;
use crate::chest::ChestParams;
use crate::cookware::CookwareParams;
use crate::counter::CounterParams;
use crate::faucet::FaucetParams;
use crate::food_display::FoodDisplayParams;
use crate::fridge::FridgeParams;
use crate::fruit::FruitParams;
use crate::partition::PartitionParams;
use crate::range::RangeParams;
use crate::shelf::ShelfParams;
use crate::table::TableParams;
use crate::Assembly;
use furniture_components::{pose_parts, PlacedPart};

/// Paint kits that have assemblies. Dresser / wardrobe / toilet stay wireframe.
pub fn try_assembly(node: &FurnitureNode) -> Option<Assembly> {
	let seed = node.finish_seed;
	let flush = node.abutment.is_some();
	Some(match node.geometry {
		FurnitureGeometry::Bed => BedParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Chair => ChairParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Table => TableParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Chest => ChestParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Counter => {
			CounterParams { finish_seed: seed, flush_back: flush }.build().assembly()
		}
		FurnitureGeometry::FoodDisplay => {
			FoodDisplayParams { finish_seed: seed }.build().assembly()
		}
		FurnitureGeometry::Fruit => FruitParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Bread => BreadParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Cookware => CookwareParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Basin => BasinParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Faucet => FaucetParams { finish_seed: seed }.build().assembly(),
		FurnitureGeometry::Shelf => {
			ShelfParams { finish_seed: seed, flush_back: flush }.build().assembly()
		}
		FurnitureGeometry::Range => {
			RangeParams { finish_seed: seed, flush_back: flush }.build().assembly()
		}
		FurnitureGeometry::Fridge => {
			FridgeParams { finish_seed: seed, flush_back: flush }.build().assembly()
		}
		FurnitureGeometry::Partition => PartitionParams { finish_seed: seed }.build().assembly(),
		_ => return None,
	})
}

/// Unit-slot parts composed under the slot placement (yaw / scale / abutment).
pub fn posed_assembly(node: &FurnitureNode) -> Option<Vec<PlacedPart>> {
	let assembly = try_assembly(node)?;
	Some(pose_parts(node.placement, &assembly.parts))
}

/// Thin wall just past the kit \(+Z\) face (already aimed at [`FurnitureNode::abutment`]).
pub fn abutment_wall(node: &FurnitureNode) -> Option<building_components::Placement> {
	node.abutment?;
	Some(node.placement.compose_child(building_components::Placement {
		translation: bevy::math::Vec3::new(0.0, 0.0, 0.52),
		yaw: 0.0,
		pitch: 0.0,
		roll: 0.0,
		scale: bevy::math::Vec3::new(1.08, 1.08, 0.04),
	}))
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::{FurnitureAbutment, Placement};

	#[test]
	fn painted_kinds_fill_and_others_do_not() -> anyhow::Result<()> {
		let bed = FurnitureNode::bed(Placement::IDENTITY).with_finish_seed(7);
		if try_assembly(&bed).is_none() {
			return Err(anyhow::anyhow!("bed should paint"));
		}
		let dresser = FurnitureNode::dresser(Placement::IDENTITY).with_finish_seed(7);
		if try_assembly(&dresser).is_some() {
			return Err(anyhow::anyhow!("dresser should stay wireframe-only"));
		}
		Ok(())
	}

	#[test]
	fn wall_counter_flushes_the_toekick() -> anyhow::Result<()> {
		let slot =
			FurnitureNode::counter(Placement::IDENTITY).with_abutment(FurnitureAbutment::PosZ);
		let assembly = try_assembly(&slot).ok_or_else(|| anyhow::anyhow!("counter paint"))?;
		let footer = assembly
			.parts
			.iter()
			.find(|p| p.kind == furniture_components::PartKind::CounterFooter)
			.ok_or_else(|| anyhow::anyhow!("missing footer"))?;
		let face = footer.placement.translation.z + footer.placement.scale.z * 0.5;
		if (face - 0.5).abs() > 1e-4 {
			return Err(anyhow::anyhow!("abutted counter toekick should flush +Z, got {face}"));
		}
		if (footer.placement.scale.x - 1.0).abs() > 1e-5 {
			return Err(anyhow::anyhow!("abutted counter should fill run length"));
		}
		Ok(())
	}

	#[test]
	fn wall_follows_abutment_yaw() -> anyhow::Result<()> {
		let slot = FurnitureNode::bed(Placement::IDENTITY).with_abutment(FurnitureAbutment::NegZ);
		let wall = abutment_wall(&slot).ok_or_else(|| anyhow::anyhow!("expected a wall"))?;
		if wall.translation.z >= 0.0 {
			return Err(anyhow::anyhow!(
				"NegZ abutment should put the wall on −Z, got {}",
				wall.translation.z
			));
		}
		Ok(())
	}
}
