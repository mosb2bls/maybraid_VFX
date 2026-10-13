//! Deck-mount faucet filling the slot.

use crate::palette::hardware;
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{slab, PartKind, PlacedPart};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FaucetParams {
	pub finish_seed: u64,
}

impl FaucetParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Faucet {
		Faucet::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Faucet {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Faucet {
	pub fn from_params(params: FaucetParams) -> Self {
		let seed = params.finish_seed;
		Self {
			finish_seed: seed,
			parts: vec![PlacedPart {
				kind: PartKind::Faucet,
				placement: slab(1.0, 0.0, 1.0),
				material: hardware(seed, 1),
			}],
		}
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Faucet,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}
