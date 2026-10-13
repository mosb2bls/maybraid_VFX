//! Chest: trunk, lid, and a metal latch on the trunk front.

use crate::palette::{chest, hardware};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{latch_slab, slab, PartKind, PlacedPart};

/// Finish-only knobs. Topology does not change with [`Self::finish_seed`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChestParams {
	pub finish_seed: u64,
}

impl ChestParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Chest {
		Chest::from_params(*self)
	}
}

/// Trunk fills most of the slot; lid occupies the top band; latch on the front.
#[derive(Clone, Debug, PartialEq)]
pub struct Chest {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Chest {
	pub fn from_params(params: ChestParams) -> Self {
		let seed = params.finish_seed;
		Self {
			finish_seed: seed,
			parts: vec![
				PlacedPart {
					kind: PartKind::ChestTrunk,
					placement: slab(1.0, 0.0, 0.78),
					material: chest(seed, 1),
				},
				PlacedPart {
					kind: PartKind::ChestLid,
					placement: slab(1.02, 0.78, 1.0),
					material: chest(seed, 2),
				},
				PlacedPart {
					kind: PartKind::ChestLatch,
					placement: latch_slab(0.22, 0.56, 0.74, 0.12),
					material: hardware(seed, 3),
				},
			],
		}
	}

	pub fn unit_from_num(num: u32) -> Self {
		ChestParams::unit_from_num(num).build()
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Chest,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn y_span(part: &PlacedPart) -> (f32, f32) {
		let y0 = part.placement.translation.y;
		(y0, y0 + part.placement.scale.y)
	}

	#[test]
	fn lid_sits_on_the_trunk() -> anyhow::Result<()> {
		let chest = ChestParams::unit_from_num(3).build();
		let trunk = chest
			.parts
			.iter()
			.find(|p| p.kind == PartKind::ChestTrunk)
			.ok_or_else(|| anyhow::anyhow!("missing trunk"))?;
		let lid = chest
			.parts
			.iter()
			.find(|p| p.kind == PartKind::ChestLid)
			.ok_or_else(|| anyhow::anyhow!("missing lid"))?;
		let (_, trunk_top) = y_span(trunk);
		let (lid_bottom, _) = y_span(lid);
		if (lid_bottom - trunk_top).abs() > 1e-4 {
			return Err(anyhow::anyhow!(
				"lid bottom {lid_bottom} should meet trunk top {trunk_top}"
			));
		}
		Ok(())
	}

	#[test]
	fn trunk_and_lid_share_the_chest_skin() -> anyhow::Result<()> {
		let built = ChestParams::unit_from_num(3).build();
		let want = match crate::palette::chest_kind(3) {
			crate::palette::ChestKind::Rockadder => furniture_shaders::RECIPE_FURNITURE_ROCKADDER,
			crate::palette::ChestKind::Cosimo => furniture_shaders::RECIPE_FURNITURE_COSMOS,
			crate::palette::ChestKind::Foliage => crate::palette::RECIPE_FOLIAGE,
			crate::palette::ChestKind::Terrain => furniture_shaders::RECIPE_FURNITURE_MARBLE,
		};
		for part in built.parts.iter().filter(|p| p.kind != PartKind::ChestLatch) {
			match &part.material.name {
				material_ref::MaterialId::Name(name) if name == want => {}
				other => {
					return Err(anyhow::anyhow!("{:?} should be {want}, got {other:?}", part.kind));
				}
			}
		}
		Ok(())
	}

	#[test]
	fn latch_sits_on_the_trunk_front() -> anyhow::Result<()> {
		let chest = ChestParams::unit_from_num(3).build();
		let latch = chest
			.parts
			.iter()
			.find(|p| p.kind == PartKind::ChestLatch)
			.ok_or_else(|| anyhow::anyhow!("missing latch"))?;
		match &latch.material.name {
			material_ref::MaterialId::Name(name)
				if name == furniture_shaders::RECIPE_FURNITURE_METAL => {}
			other => return Err(anyhow::anyhow!("latch should be metal, got {other:?}")),
		}
		let want = latch_slab(0.22, 0.56, 0.74, 0.12);
		if (latch.placement.translation - want.translation).length() > 1e-4
			|| (latch.placement.scale - want.scale).length() > 1e-4
		{
			return Err(anyhow::anyhow!(
				"latch should sit on the upper trunk front, got {:?}",
				latch.placement
			));
		}
		if latch.placement.translation.z.abs() > 1e-4 {
			return Err(anyhow::anyhow!(
				"latch plan Z offset should stay 0 (front from kit map), got {}",
				latch.placement.translation.z
			));
		}
		Ok(())
	}
}
