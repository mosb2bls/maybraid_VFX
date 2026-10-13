//! Fridge: carcass plus a pair of hinge doors on the front.

use crate::palette::{enamel, hardware};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{run_slab, slab, PartKind, PlacedPart};
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FridgeParams {
	pub finish_seed: u64,
	pub flush_back: bool,
}

impl FridgeParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num), flush_back: true }
	}

	pub fn build(&self) -> Fridge {
		Fridge::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fridge {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Fridge {
	pub fn from_params(params: FridgeParams) -> Self {
		let seed = params.finish_seed;
		let flush = params.flush_back;
		let mut right = slab(1.0, 0.04, 0.96);
		right.scale.x = 1.0;
		right.translation.x = 0.0;
		let mut left = right;
		left.yaw = PI;
		Self {
			finish_seed: seed,
			parts: vec![
				PlacedPart {
					kind: PartKind::FridgeBody,
					placement: run_slab(1.0, 1.0, 0.0, 1.0, flush),
					material: enamel(seed, 1),
				},
				PlacedPart {
					kind: PartKind::FridgeDoor,
					placement: right,
					material: hardware(seed, 2),
				},
				PlacedPart {
					kind: PartKind::FridgeDoor,
					placement: left,
					material: hardware(seed, 2),
				},
			],
		}
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Fridge,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn two_doors_share_the_center_hinge() -> anyhow::Result<()> {
		let fridge = FridgeParams::unit_from_num(2).build();
		let doors: Vec<_> =
			fridge.parts.iter().filter(|p| p.kind == PartKind::FridgeDoor).collect();
		if doors.len() != 2 {
			return Err(anyhow::anyhow!("expected two doors"));
		}
		if (doors[0].placement.yaw - doors[1].placement.yaw).abs() < 1.0 {
			return Err(anyhow::anyhow!("doors should face opposite halves"));
		}
		Ok(())
	}
}
