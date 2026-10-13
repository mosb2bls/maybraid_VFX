//! Roof-facing wrapper over [`RuledStrip`]: `rail_a` = eave, `rail_b` = ridge.
//!
//! Locks the authoring convention so roof call sites do not invent which rail is
//! which. Geometry / joints / presentation all live on the inner strip.

use building_components::panels::PanelStyle;

use crate::paneling::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, PanelPoint, PanelPointId,
};
use crate::paneling::ruled_strip::RuledStrip;

/// Equal-station eave / ridge pitch: thin roof vocabulary over [`RuledStrip`].
///
/// Convention: **eave → `rail_a`**, **ridge → `rail_b`**. Generators are rafters.
#[derive(Debug, Clone, PartialEq)]
pub struct RuledPitch(RuledStrip);

impl RuledPitch {
	/// Empty pitch (no stations).
	pub fn new(style: PanelStyle) -> Self {
		Self(RuledStrip::new(style))
	}

	pub fn rough_stone() -> Self {
		Self(RuledStrip::rough_stone())
	}

	pub fn shepherds_thatch() -> Self {
		Self(RuledStrip::shepherds_thatch())
	}

	/// Bulk construct from eave + ridge (equal lengths ≥ 2).
	pub fn from_lines(
		style: PanelStyle,
		eave: impl IntoIterator<Item = impl Into<PanelPoint>>,
		ridge: impl IntoIterator<Item = impl Into<PanelPoint>>,
	) -> Self {
		Self(RuledStrip::from_lines(style, eave, ridge))
	}

	/// Sugar: rebuild from eave / ridge keeping style and joint policy.
	pub fn with_lines(
		self,
		eave: impl IntoIterator<Item = impl Into<PanelPoint>>,
		ridge: impl IntoIterator<Item = impl Into<PanelPoint>>,
	) -> Self {
		Self(self.0.with_lines(eave, ridge))
	}

	/// Append one eave / ridge station (adds a bay when a previous station exists).
	pub fn add_pair(
		&mut self,
		eave: impl Into<PanelPoint>,
		ridge: impl Into<PanelPoint>,
	) -> &mut Self {
		self.0.add_pair(eave, ridge);
		self
	}

	pub fn with_joint_policy(self, joint_policy: PanelComplexJointPolicy) -> Self {
		Self(self.0.with_joint_policy(joint_policy))
	}

	pub fn set_joint_policy(&mut self, joint_policy: PanelComplexJointPolicy) -> &mut Self {
		self.0.set_joint_policy(joint_policy);
		self
	}

	pub fn stations(&self) -> &[(PanelPointId, PanelPointId)] {
		self.0.stations()
	}

	pub fn eave_ids(&self) -> impl Iterator<Item = PanelPointId> + '_ {
		self.0.rail_a_ids()
	}

	pub fn ridge_ids(&self) -> impl Iterator<Item = PanelPointId> + '_ {
		self.0.rail_b_ids()
	}

	pub fn eave_polyline(&self) -> Vec<PanelPoint> {
		self.0.rail_a_polyline()
	}

	pub fn ridge_polyline(&self) -> Vec<PanelPoint> {
		self.0.rail_b_polyline()
	}

	pub fn as_strip(&self) -> &RuledStrip {
		&self.0
	}

	pub fn into_strip(self) -> RuledStrip {
		self.0
	}

	pub fn as_complex(&self) -> &PanelComplex {
		self.0.as_complex()
	}

	pub fn into_complex(self) -> PanelComplex {
		self.0.into_complex()
	}

	pub fn into_parts(self) -> (PanelComplex, Vec<(PanelPointId, PanelPointId)>) {
		self.0.into_parts()
	}
}

impl AsRef<RuledStrip> for RuledPitch {
	fn as_ref(&self) -> &RuledStrip {
		&self.0
	}
}

impl From<RuledPitch> for RuledStrip {
	fn from(value: RuledPitch) -> Self {
		value.into_strip()
	}
}

impl From<RuledPitch> for PanelComplex {
	fn from(value: RuledPitch) -> Self {
		value.into_complex()
	}
}

impl From<RuledStrip> for RuledPitch {
	/// Interpret an existing strip as a pitch: `rail_a` = eave, `rail_b` = ridge.
	fn from(value: RuledStrip) -> Self {
		Self(value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;

	#[test]
	fn eave_ridge_map_to_rails() {
		let eave = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 4.0)];
		let ridge = [Vec3::new(1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 2.0), Vec3::new(1.0, 1.0, 4.0)];
		let pitch = RuledPitch::from_lines(PanelStyle::ShepherdsThatch, eave, ridge);
		let strip = pitch.as_strip();
		assert_eq!(pitch.eave_ids().collect::<Vec<_>>(), strip.rail_a_ids().collect::<Vec<_>>());
		assert_eq!(pitch.ridge_ids().collect::<Vec<_>>(), strip.rail_b_ids().collect::<Vec<_>>());
		for (a, b) in pitch.eave_polyline().iter().zip(strip.rail_a_polyline()) {
			assert!((a.position - b.position).length() < 1e-5);
		}
		for (a, b) in pitch.ridge_polyline().iter().zip(strip.rail_b_polyline()) {
			assert!((a.position - b.position).length() < 1e-5);
		}
	}
}
