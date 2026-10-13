//! Cafe table: four-leg apron or diner pedestal.

use bevy::math::Vec3;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::palette::{carcass, lacquer, mix_seed};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{shift, slab, PartKind, PlacedPart};

const TOP_Y0: f32 = 0.88;
const APRON_Y0: f32 = 0.76;
const LEG_XZ: f32 = 0.10;
const LEG_INSET: f32 = 0.36;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TableParams {
	pub finish_seed: u64,
}

impl TableParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Table {
		Table::from_params(*self)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Table {
	pub fn from_params(params: TableParams) -> Self {
		let seed = params.finish_seed;
		let wood = carcass(seed, 1);
		let top_skin = lacquer(seed, 2);
		let mut parts = Vec::new();
		if mix_seed(seed, 0x7AB1) % 2 == 0 {
			parts.push(PlacedPart {
				kind: PartKind::TableTopRound,
				placement: slab(0.92, TOP_Y0, 1.0),
				material: top_skin,
			});
			parts.push(PlacedPart {
				kind: PartKind::TablePedestal,
				placement: slab(0.38, 0.0, TOP_Y0),
				material: wood,
			});
		} else {
			parts.push(PlacedPart {
				kind: PartKind::TableTop,
				placement: slab(1.0, TOP_Y0, 1.0),
				material: top_skin,
			});
			for yaw in [0.0, FRAC_PI_2, PI, -FRAC_PI_2] {
				let mut placement = slab(0.88, APRON_Y0, TOP_Y0);
				placement.yaw = yaw;
				parts.push(PlacedPart {
					kind: PartKind::TableApron,
					placement,
					material: wood.clone(),
				});
			}
			for (x, z) in [
				(-LEG_INSET, -LEG_INSET),
				(LEG_INSET, -LEG_INSET),
				(-LEG_INSET, LEG_INSET),
				(LEG_INSET, LEG_INSET),
			] {
				parts.push(PlacedPart {
					kind: PartKind::TableLeg,
					placement: shift(slab(LEG_XZ, 0.0, APRON_Y0), Vec3::new(x, 0.0, z)),
					material: wood.clone(),
				});
			}
		}
		Self { finish_seed: seed, parts }
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Table,
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
		let kinds: Vec<_> = (0..12)
			.map(|seed| {
				TableParams { finish_seed: seed }
					.build()
					.parts
					.iter()
					.any(|p| p.kind == PartKind::TablePedestal)
			})
			.collect();
		if kinds.iter().all(|p| *p) || kinds.iter().all(|p| !*p) {
			return Err(anyhow::anyhow!("expected both pedestal and four-leg tables"));
		}
		Ok(())
	}
}
