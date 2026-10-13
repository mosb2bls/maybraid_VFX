//! Sit-on basin filling the slot.

use crate::palette::enamel;
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{slab, PartKind, PlacedPart};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BasinParams {
	pub finish_seed: u64,
}

impl BasinParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Basin {
		Basin::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Basin {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Basin {
	pub fn from_params(params: BasinParams) -> Self {
		let seed = params.finish_seed;
		Self {
			finish_seed: seed,
			parts: vec![PlacedPart {
				kind: PartKind::Basin,
				placement: slab(1.0, 0.0, 1.0),
				material: enamel(seed, 1),
			}],
		}
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Basin,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}
