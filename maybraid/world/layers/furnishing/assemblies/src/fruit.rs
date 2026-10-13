//! Sit-on fruit: one apple / orange / pear / banana from the seed.

use crate::palette::{mix_seed, produce};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{slab, PartKind, PlacedPart};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FruitParams {
	pub finish_seed: u64,
}

impl FruitParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Fruit {
		Fruit::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fruit {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Fruit {
	pub fn from_params(params: FruitParams) -> Self {
		let seed = params.finish_seed;
		let kind = match mix_seed(seed, 0xF201) % 4 {
			0 => PartKind::Apple,
			1 => PartKind::Orange,
			2 => PartKind::Pear,
			_ => PartKind::Banana,
		};
		Self {
			finish_seed: seed,
			parts: vec![PlacedPart {
				kind,
				placement: slab(1.0, 0.0, 1.0),
				material: produce(seed, 1),
			}],
		}
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Fruit,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn seed_picks_more_than_one_kind() -> anyhow::Result<()> {
		let kinds: std::collections::HashSet<_> = (0..24_u64)
			.map(|seed| FruitParams { finish_seed: seed }.build().parts[0].kind)
			.collect();
		if kinds.len() < 3 {
			return Err(anyhow::anyhow!("expected several fruit kits, got {kinds:?}"));
		}
		Ok(())
	}
}
