//! Counter: toekick, cabinet, top — flush ends, depth-only kick.

use crate::palette::{carcass, marble};
use crate::Assembly;
use building_components::FurnitureGeometry;
use furniture_components::{run_slab, PartKind, PlacedPart};

/// Toekick insets only on the room-side depth, never the run length.
pub const TOEKICK_Z: f32 = 0.72;
/// Cabinet and top fill the packed AABB (flush ends and, when abutted, the wall).
pub const CABINET_Z: f32 = 1.0;

/// Finish knobs plus whether the run sits on a wall.
///
/// Topology does not change with [`Self::finish_seed`]. [`Self::flush_back`]
/// only shifts the toekick toward kit \(+Z\).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CounterParams {
	pub finish_seed: u64,
	/// Push the \(+Z\) face to the unit-cube wall (kit facing / abutment).
	pub flush_back: bool,
}

impl CounterParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num), flush_back: true }
	}

	pub fn build(&self) -> Counter {
		Counter::from_params(*self)
	}
}

/// Footer is a depth-only toekick; volume and top fill the packed box.
#[derive(Clone, Debug, PartialEq)]
pub struct Counter {
	pub finish_seed: u64,
	pub flush_back: bool,
	pub parts: Vec<PlacedPart>,
}

impl Counter {
	pub fn from_params(params: CounterParams) -> Self {
		let seed = params.finish_seed;
		let flush = params.flush_back;
		Self {
			finish_seed: seed,
			flush_back: flush,
			parts: vec![
				PlacedPart {
					kind: PartKind::CounterFooter,
					placement: run_slab(1.0, TOEKICK_Z, 0.0, 0.12, flush),
					material: carcass(seed, 1),
				},
				PlacedPart {
					kind: PartKind::CounterVolume,
					placement: run_slab(1.0, CABINET_Z, 0.12, 0.88, flush),
					material: carcass(seed, 2),
				},
				PlacedPart {
					kind: PartKind::CounterTop,
					placement: run_slab(1.0, CABINET_Z, 0.88, 1.0, flush),
					material: marble(seed, 3),
				},
			],
		}
	}

	pub fn unit_from_num(num: u32) -> Self {
		CounterParams::unit_from_num(num).build()
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Counter,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn part<'a>(counter: &'a Counter, kind: PartKind) -> anyhow::Result<&'a PlacedPart> {
		counter
			.parts
			.iter()
			.find(|p| p.kind == kind)
			.ok_or_else(|| anyhow::anyhow!("missing {kind:?}"))
	}

	fn pos_z_face(part: &PlacedPart) -> f32 {
		part.placement.translation.z + part.placement.scale.z * 0.5
	}

	#[test]
	fn toekick_insets_depth_not_length() -> anyhow::Result<()> {
		let counter = CounterParams { finish_seed: 8, flush_back: false }.build();
		let footer = part(&counter, PartKind::CounterFooter)?;
		let volume = part(&counter, PartKind::CounterVolume)?;
		let top = part(&counter, PartKind::CounterTop)?;
		for band in [footer, volume, top] {
			if (band.placement.scale.x - 1.0).abs() > 1e-5 {
				return Err(anyhow::anyhow!(
					"{:?} must fill run length, got scale.x={}",
					band.kind,
					band.placement.scale.x
				));
			}
		}
		if footer.placement.scale.z >= volume.placement.scale.z {
			return Err(anyhow::anyhow!("toekick must be shallower than the volume"));
		}
		if (volume.placement.scale.z - 1.0).abs() > 1e-5
			|| (top.placement.scale.z - 1.0).abs() > 1e-5
		{
			return Err(anyhow::anyhow!("cabinet and top must fill the packed depth"));
		}
		if footer.placement.translation.z.abs() > 1e-5 {
			return Err(anyhow::anyhow!("island toekick should stay centered"));
		}
		Ok(())
	}

	#[test]
	fn flush_back_puts_bands_on_the_wall() -> anyhow::Result<()> {
		let counter = CounterParams::unit_from_num(8).build();
		for kind in [PartKind::CounterFooter, PartKind::CounterVolume, PartKind::CounterTop] {
			let band = part(&counter, kind)?;
			let face = pos_z_face(band);
			if (face - 0.5).abs() > 1e-5 {
				return Err(anyhow::anyhow!("{kind:?} +Z face should sit at 0.5, got {face}"));
			}
		}
		let footer = part(&counter, PartKind::CounterFooter)?;
		if footer.placement.scale.z >= CABINET_Z {
			return Err(anyhow::anyhow!("flush toekick should still inset the room side"));
		}
		Ok(())
	}

	#[test]
	fn top_uses_marble() -> anyhow::Result<()> {
		let counter = CounterParams::unit_from_num(8).build();
		let top = part(&counter, PartKind::CounterTop)?;
		match &top.material.name {
			material_ref::MaterialId::Name(name)
				if name == furniture_shaders::RECIPE_FURNITURE_MARBLE =>
			{
				Ok(())
			}
			other => Err(anyhow::anyhow!("countertop should be marble, got {other:?}")),
		}
	}
}
