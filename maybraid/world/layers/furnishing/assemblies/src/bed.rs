//! Bed: frame, mattress, covers.

use crate::palette::{carcass, cloth, mattress};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{slab, PartKind, PlacedPart};

/// Finish-only knobs. Topology does not change with [`Self::finish_seed`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BedParams {
	pub finish_seed: u64,
}

impl BedParams {
	/// Unit-slot bed whose palette is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Bed {
		Bed::from_params(*self)
	}
}

/// Frame under an inset mattress. Covers use the **same** transform as the
/// mattress — the authored covers GLB already carries the cloth lip.
#[derive(Clone, Debug, PartialEq)]
pub struct Bed {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Bed {
	pub fn from_params(params: BedParams) -> Self {
		let seed = params.finish_seed;
		let mattress_placement = slab(0.92, 0.32, 0.92);
		Self {
			finish_seed: seed,
			parts: vec![
				PlacedPart {
					kind: PartKind::BedFrame,
					placement: slab(1.0, 0.0, 0.32),
					material: carcass(seed, 1),
				},
				PlacedPart {
					kind: PartKind::Mattress,
					placement: mattress_placement,
					material: mattress(seed, 2),
				},
				PlacedPart {
					kind: PartKind::Covers,
					placement: mattress_placement,
					material: cloth(seed, 3),
				},
			],
		}
	}

	pub fn unit_from_num(num: u32) -> Self {
		BedParams::unit_from_num(num).build()
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Bed,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn covers_share_the_mattress_transform() -> anyhow::Result<()> {
		let bed = BedParams::unit_from_num(1).build();
		let mattress = bed
			.parts
			.iter()
			.find(|p| p.kind == PartKind::Mattress)
			.ok_or_else(|| anyhow::anyhow!("missing mattress"))?;
		let covers = bed
			.parts
			.iter()
			.find(|p| p.kind == PartKind::Covers)
			.ok_or_else(|| anyhow::anyhow!("missing covers"))?;
		if mattress.placement != covers.placement {
			return Err(anyhow::anyhow!("covers must use the mattress transform"));
		}
		if mattress.material == covers.material {
			return Err(anyhow::anyhow!("mattress and covers must paint differently"));
		}
		Ok(())
	}

	#[test]
	fn unit_from_num_changes_paint_only() -> anyhow::Result<()> {
		let a = BedParams::unit_from_num(1).build();
		let b = BedParams::unit_from_num(99).build();
		if a.parts.len() != b.parts.len() {
			return Err(anyhow::anyhow!("topology length changed"));
		}
		let mut paint_changed = false;
		for (left, right) in a.parts.iter().zip(&b.parts) {
			if left.kind != right.kind || left.placement != right.placement {
				return Err(anyhow::anyhow!("topology changed with seed"));
			}
			paint_changed |= left.material != right.material;
		}
		if !paint_changed {
			return Err(anyhow::anyhow!("expected palette to change with seed"));
		}
		Ok(())
	}
}
