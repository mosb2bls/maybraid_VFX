//! Floor-to-ceiling slat screen. Reuses counter / shelf carcass kits.

use bevy::math::Vec3;

use crate::palette::carcass;
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{shift, slab_xz, PartKind, PlacedPart};

const SLATS: usize = 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartitionParams {
	pub finish_seed: u64,
}

impl PartitionParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Partition {
		Partition::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Partition {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Partition {
	pub fn from_params(params: PartitionParams) -> Self {
		let seed = params.finish_seed;
		let wood = carcass(seed, 1);
		let mut parts = Vec::new();
		for i in 0..SLATS {
			let t = if SLATS <= 1 { 0.5 } else { i as f32 / (SLATS as f32 - 1.0) };
			let x = (t - 0.5) * 0.86;
			parts.push(PlacedPart {
				kind: PartKind::CounterVolume,
				placement: shift(slab_xz(0.07, 0.58, 0.0, 1.0), Vec3::new(x, 0.0, 0.0)),
				material: wood.clone(),
			});
		}
		for (y0, y1) in [(0.0, 0.055), (0.47, 0.53), (0.945, 1.0)] {
			parts.push(PlacedPart {
				kind: PartKind::ShelfRow,
				placement: slab_xz(0.98, 0.68, y0, y1),
				material: wood.clone(),
			});
		}
		Self { finish_seed: seed, parts }
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Partition,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn screen_has_slats_and_rails() -> anyhow::Result<()> {
		let screen = PartitionParams::unit_from_num(1).build();
		let slats = screen.parts.iter().filter(|p| p.kind == PartKind::CounterVolume).count();
		let rails = screen.parts.iter().filter(|p| p.kind == PartKind::ShelfRow).count();
		if slats != SLATS {
			return Err(anyhow::anyhow!("expected {SLATS} slats, got {slats}"));
		}
		if rails != 3 {
			return Err(anyhow::anyhow!("expected three rails, got {rails}"));
		}
		Ok(())
	}
}
