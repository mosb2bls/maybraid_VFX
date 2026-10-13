//! Range: carcass, oven door, four burners, splash knobs.

use bevy::math::Vec3;

use crate::palette::{hardware, metal};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{run_slab, shift, slab, PartKind, PlacedPart};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RangeParams {
	pub finish_seed: u64,
	pub flush_back: bool,
}

impl RangeParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num), flush_back: true }
	}

	pub fn build(&self) -> Range {
		Range::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Range {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Range {
	pub fn from_params(params: RangeParams) -> Self {
		let seed = params.finish_seed;
		let flush = params.flush_back;
		let skin = metal(seed, 1);
		let mut parts = vec![
			PlacedPart {
				kind: PartKind::RangeBody,
				placement: run_slab(1.0, 1.0, 0.0, 1.0, flush),
				material: skin.clone(),
			},
			PlacedPart {
				kind: PartKind::RangeDoor,
				placement: run_slab(0.92, 0.08, 0.10, 0.70, false),
				material: hardware(seed, 2),
			},
		];
		for (x, z) in [(-0.26, -0.14), (0.26, -0.14), (-0.26, 0.12), (0.26, 0.12)] {
			parts.push(PlacedPart {
				kind: PartKind::RangeBurner,
				placement: shift(slab(0.28, 0.84, 1.0), Vec3::new(x, 0.0, z)),
				material: hardware(seed, 3),
			});
		}
		for x in [-0.22, 0.0, 0.22] {
			parts.push(PlacedPart {
				kind: PartKind::RangeKnob,
				placement: shift(slab(0.10, 0.78, 0.92), Vec3::new(x, 0.0, 0.38)),
				material: hardware(seed, 4),
			});
		}
		Self { finish_seed: seed, parts }
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Range,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn range_has_body_door_and_burners() -> anyhow::Result<()> {
		let range = RangeParams::unit_from_num(3).build();
		let body = range.parts.iter().filter(|p| p.kind == PartKind::RangeBody).count();
		let door = range.parts.iter().filter(|p| p.kind == PartKind::RangeDoor).count();
		let burners = range.parts.iter().filter(|p| p.kind == PartKind::RangeBurner).count();
		if body != 1 || door != 1 || burners != 4 {
			return Err(anyhow::anyhow!(
				"range parts drifted: body={body} door={door} burners={burners}"
			));
		}
		Ok(())
	}
}
