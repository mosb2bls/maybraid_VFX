//! Part stack geometry derived from [`DoricColumnSpec`].

use bevy::prelude::*;
use building_components::Placement;

use crate::spec::DoricColumnSpec;

/// Resolved world-unit sizes after [`DoricColumnSpec::scale`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoricColumnLayout {
	pub pedestal: PartLayout,
	pub shaft: PartLayout,
	pub capital: PartLayout,
	pub chain_anchor_outset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartLayout {
	pub height: f32,
	pub radius_bottom: f32,
	pub radius_top: f32,
}

impl DoricColumnLayout {
	pub fn from_spec(spec: &DoricColumnSpec) -> Self {
		let s = spec.scale;
		let d = spec.dims;
		Self {
			pedestal: PartLayout {
				height: d.pedestal_height * s.y,
				radius_bottom: d.pedestal_radius_bottom * s.x.max(s.z),
				radius_top: d.pedestal_radius_top * s.x.max(s.z),
			},
			shaft: PartLayout {
				height: d.shaft_height * s.y,
				radius_bottom: d.shaft_radius * s.x.max(s.z),
				radius_top: d.shaft_radius * s.x.max(s.z),
			},
			capital: PartLayout {
				height: d.capital_height * s.y,
				radius_bottom: d.capital_radius * s.x.max(s.z),
				radius_top: d.capital_radius * s.x.max(s.z),
			},
			chain_anchor_outset: d.chain_anchor_outset * s.x.max(s.z),
		}
	}

	pub fn total_height(&self) -> f32 {
		self.pedestal.height + self.shaft.height + self.capital.height
	}
}

pub struct PartPlacements {
	pub pedestal: Placement,
	pub shaft: Placement,
	pub capital: Placement,
}

pub fn part_placements(layout: &DoricColumnLayout) -> PartPlacements {
	let pedestal_y = 0.0;
	let shaft_y = layout.pedestal.height;
	let capital_y = shaft_y + layout.shaft.height;
	PartPlacements {
		pedestal: part_placement(pedestal_y, layout.pedestal),
		shaft: part_placement(shaft_y, layout.shaft),
		capital: part_placement(capital_y, layout.capital),
	}
}

fn part_placement(base_y: f32, part: PartLayout) -> Placement {
	let diameter = (part.radius_bottom.max(part.radius_top)) * 2.0;
	Placement::new(Vec3::new(0.0, base_y, 0.0), 0.0)
		.with_scale(Vec3::new(diameter, part.height, diameter))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::spec::{DoricColumnDimensions, DoricColumnSpec};

	#[test]
	fn assembly_height_is_sum_of_parts() {
		let spec = DoricColumnSpec {
			dims: DoricColumnDimensions {
				pedestal_height: 1.0,
				shaft_height: 10.0,
				capital_height: 0.5,
				..DoricColumnDimensions::default()
			},
			scale: Vec3::splat(2.0),
			yaw: 0.0,
			materials: Default::default(),
		};
		let layout = DoricColumnLayout::from_spec(&spec);
		assert!((layout.total_height() - (1.0 + 10.0 + 0.5) * 2.0).abs() < 1e-4);
	}
}
