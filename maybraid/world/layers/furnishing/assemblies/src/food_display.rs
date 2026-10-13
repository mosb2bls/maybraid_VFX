//! Sit-on diner display: one case filling the slot.

use crate::palette::{carcass, metal};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{slab, PartKind, PlacedPart};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FoodDisplayParams {
	pub finish_seed: u64,
}

impl FoodDisplayParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> FoodDisplay {
		FoodDisplay::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct FoodDisplay {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl FoodDisplay {
	pub fn from_params(params: FoodDisplayParams) -> Self {
		let seed = params.finish_seed;
		Self {
			finish_seed: seed,
			parts: vec![PlacedPart {
				kind: PartKind::FoodDisplay,
				placement: slab(1.0, 0.0, 1.0),
				material: if crate::palette::mix_seed(seed, 4) % 2 == 0 {
					metal(seed, 1)
				} else {
					carcass(seed, 1)
				},
			}],
		}
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::FoodDisplay,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}
