//! Sit-on bread: loaf / baguette / bun / boule from the seed.

use crate::palette::{crust, mix_seed};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{slab, PartKind, PlacedPart};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BreadParams {
	pub finish_seed: u64,
}

impl BreadParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Bread {
		Bread::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bread {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Bread {
	pub fn from_params(params: BreadParams) -> Self {
		let seed = params.finish_seed;
		let kind = match mix_seed(seed, 0xB0AD) % 4 {
			0 => PartKind::Loaf,
			1 => PartKind::Baguette,
			2 => PartKind::Bun,
			_ => PartKind::Boule,
		};
		Self {
			finish_seed: seed,
			parts: vec![PlacedPart {
				kind,
				placement: slab(1.0, 0.0, 1.0),
				material: crust(seed, 1),
			}],
		}
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Bread,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}
