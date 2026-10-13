//! Sit-on pot / skillet / saucepan; pot keeps a lid.

use crate::palette::{hardware, metal, mix_seed};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{slab, PartKind, PlacedPart};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CookwareParams {
	pub finish_seed: u64,
}

impl CookwareParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Cookware {
		Cookware::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cookware {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Cookware {
	pub fn from_params(params: CookwareParams) -> Self {
		let seed = params.finish_seed;
		let skin = metal(seed, 1);
		let mut parts = Vec::new();
		match mix_seed(seed, 0xC00C) % 3 {
			0 => {
				parts.push(PlacedPart {
					kind: PartKind::Pot,
					placement: slab(1.0, 0.0, 0.82),
					material: skin.clone(),
				});
				parts.push(PlacedPart {
					kind: PartKind::PotLid,
					placement: slab(0.92, 0.82, 1.0),
					material: hardware(seed, 2),
				});
			}
			1 => parts.push(PlacedPart {
				kind: PartKind::Skillet,
				placement: slab(1.0, 0.0, 1.0),
				material: skin,
			}),
			_ => parts.push(PlacedPart {
				kind: PartKind::Saucepan,
				placement: slab(1.0, 0.0, 1.0),
				material: skin,
			}),
		}
		Self { finish_seed: seed, parts }
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Cookware,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}
