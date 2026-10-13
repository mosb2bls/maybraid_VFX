//! Wall shelf row filling the slot.

use crate::palette::carcass;
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{run_slab, PartKind, PlacedPart};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShelfParams {
	pub finish_seed: u64,
	pub flush_back: bool,
}

impl ShelfParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num), flush_back: true }
	}

	pub fn build(&self) -> Shelf {
		Shelf::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shelf {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Shelf {
	pub fn from_params(params: ShelfParams) -> Self {
		let seed = params.finish_seed;
		Self {
			finish_seed: seed,
			parts: vec![PlacedPart {
				kind: PartKind::ShelfRow,
				placement: run_slab(1.0, 1.0, 0.0, 1.0, params.flush_back),
				material: carcass(seed, 1),
			}],
		}
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Shelf,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}
