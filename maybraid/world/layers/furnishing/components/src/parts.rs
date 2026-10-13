//! Exploded kit pieces in unit-slot space.

use bevy::prelude::Component;
use building_components::{AssetPath, Placement};
use material_ref::MaterialRef;

use crate::assets;
use crate::kit_space::{
	place_kit, BOX_KIT_TO_UNIT, HINGE_KIT_TO_UNIT, LATCH_KIT_TO_UNIT, LEG_KIT_TO_UNIT,
	RANGE_DOOR_KIT_TO_UNIT,
};

/// One kit piece: unit-slot slab + deferred paint.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedPart {
	pub kind: PartKind,
	pub placement: Placement,
	pub material: MaterialRef,
}

/// Authored furniture kit piece (matches `maybraid/art/furniture/`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartKind {
	BedFrame,
	Mattress,
	Covers,
	ChestTrunk,
	ChestLid,
	ChestLatch,
	ChairLeg,
	ChairSeat,
	ChairBack,
	CounterFooter,
	CounterVolume,
	CounterTop,
	FoodDisplay,
	Apple,
	Orange,
	Pear,
	Banana,
	Loaf,
	Baguette,
	Bun,
	Boule,
	ShelfRow,
	RangeBody,
	RangeDoor,
	RangeBurner,
	RangeKnob,
	Basin,
	Faucet,
	FridgeBody,
	FridgeDoor,
	Pot,
	PotLid,
	Skillet,
	Saucepan,
	TableTop,
	TableTopRound,
	TableLeg,
	TableApron,
	TablePedestal,
}

impl PartKind {
	/// Destination GLB under `maybraid/assets`.
	pub const fn asset_path(self) -> AssetPath {
		match self {
			Self::BedFrame => assets::BEDFRAME_001,
			Self::Mattress => assets::MATTRESS_001,
			Self::Covers => assets::COVERS_001,
			Self::ChestTrunk => assets::CHEST_TRUNK_001,
			Self::ChestLid => assets::CHEST_LID_001,
			Self::ChestLatch => assets::CHEST_LATCH_001,
			Self::ChairLeg => assets::CHAIR_LEG_001,
			Self::ChairSeat => assets::CHAIR_SEAT_001,
			Self::ChairBack => assets::CHAIR_BACK_001,
			Self::CounterFooter => assets::COUNTER_FOOTER_001,
			Self::CounterVolume => assets::COUNTER_VOLUME_001,
			Self::CounterTop => assets::COUNTER_TOP_001,
			Self::FoodDisplay => assets::FOOD_DISPLAY_001,
			Self::Apple => assets::APPLE_001,
			Self::Orange => assets::ORANGE_001,
			Self::Pear => assets::PEAR_001,
			Self::Banana => assets::BANANA_001,
			Self::Loaf => assets::LOAF_001,
			Self::Baguette => assets::BAGUETTE_001,
			Self::Bun => assets::BUN_001,
			Self::Boule => assets::BOULE_001,
			Self::ShelfRow => assets::SHELF_ROW_001,
			Self::RangeBody => assets::RANGE_BODY_001,
			Self::RangeDoor => assets::RANGE_DOOR_001,
			Self::RangeBurner => assets::RANGE_BURNER_001,
			Self::RangeKnob => assets::RANGE_KNOB_001,
			Self::Basin => assets::BASIN_001,
			Self::Faucet => assets::FAUCET_001,
			Self::FridgeBody => assets::FRIDGE_BODY_001,
			Self::FridgeDoor => assets::FRIDGE_DOOR_001,
			Self::Pot => assets::POT_001,
			Self::PotLid => assets::POT_LID_001,
			Self::Skillet => assets::SKILLET_001,
			Self::Saucepan => assets::SAUCEPAN_001,
			Self::TableTop => assets::TABLE_TOP_001,
			Self::TableTopRound => assets::TABLE_TOP_ROUND_001,
			Self::TableLeg => assets::TABLE_LEG_001,
			Self::TableApron => assets::TABLE_APRON_001,
			Self::TablePedestal => assets::TABLE_PEDESTAL_001,
		}
	}

	/// Authored-kit → unit-slot map for this piece.
	pub const fn kit_to_unit(self) -> Placement {
		match self {
			Self::ChairLeg | Self::TableLeg => LEG_KIT_TO_UNIT,
			Self::ChestLatch => LATCH_KIT_TO_UNIT,
			Self::FridgeDoor => HINGE_KIT_TO_UNIT,
			Self::RangeDoor => RANGE_DOOR_KIT_TO_UNIT,
			_ => BOX_KIT_TO_UNIT,
		}
	}

	/// Slot-local pose for this GLB: kit remap, then the unit-slot slab.
	pub fn place_in_slot(self, slot: Placement, unit: Placement) -> Placement {
		place_kit(slot, self.kit_to_unit(), unit)
	}
}

/// Stamped on a fulfilled kit piece so world code can find a chest lid or trunk
/// after the furniture cell streams in.
///
/// [`Default`] exists so Bevy's scene template can patch a real part over it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FurnitureKitPart {
	pub kind: PartKind,
	pub finish_seed: u64,
	pub slot: u32,
}

impl Default for FurnitureKitPart {
	fn default() -> Self {
		Self { kind: PartKind::ChestLid, finish_seed: 0, slot: 0 }
	}
}

/// Compose unit-slot parts under a stamped slot placement (yaw / scale / abutment).
pub fn pose_parts(slot: Placement, parts: &[PlacedPart]) -> Vec<PlacedPart> {
	parts
		.iter()
		.map(|part| PlacedPart {
			kind: part.kind,
			placement: part.kind.place_in_slot(slot, part.placement),
			material: part.material.clone(),
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn asset_paths_match_the_art_layout() -> anyhow::Result<()> {
		if PartKind::Covers.asset_path().as_str() != "furniture/bed/covers/covers_001.glb" {
			return Err(anyhow::anyhow!("covers path drifted from art/furniture"));
		}
		if PartKind::ChairLeg.asset_path().as_str() != "furniture/chair/legs/chair_leg_001.glb" {
			return Err(anyhow::anyhow!("chair-leg path drifted from art/furniture"));
		}
		if PartKind::ChestLatch.asset_path().as_str() != "furniture/chest/latch/latch_001.glb" {
			return Err(anyhow::anyhow!("latch path drifted from art/furniture"));
		}
		if PartKind::FoodDisplay.asset_path().as_str()
			!= "furniture/food_display/case/food_display_001.glb"
		{
			return Err(anyhow::anyhow!("food-display path drifted from art/furniture"));
		}
		Ok(())
	}
}
