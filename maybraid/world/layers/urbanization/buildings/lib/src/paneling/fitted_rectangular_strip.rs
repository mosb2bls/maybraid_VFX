//! Two-rail strip of best-fit [`PanelGeometry::Rectangle`] kits + crease joints.

use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::fitted_rectangle::FittedRectangle;
use crate::paneling::panel_complex::{PanelComplexJointPolicy, PanelPoint};
use crate::paneling::rect_crease::joint_along_fitted_bay_crease;

/// Equal-station strip; each bay is an independently fitted rectangle kit.
#[derive(Debug, Clone, PartialEq)]
pub struct FittedRectangularStrip {
	style: PanelStyle,
	joint_policy: PanelComplexJointPolicy,
	authored: Vec<(PanelPoint, PanelPoint)>,
	bays: Vec<FittedRectangle>,
}

impl FittedRectangularStrip {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			authored: Vec::new(),
			bays: Vec::new(),
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self
	}

	pub fn from_lines(
		style: PanelStyle,
		rail_a: impl IntoIterator<Item = impl Into<PanelPoint>>,
		rail_b: impl IntoIterator<Item = impl Into<PanelPoint>>,
	) -> Self {
		let rail_a: Vec<PanelPoint> = rail_a.into_iter().map(Into::into).collect();
		let rail_b: Vec<PanelPoint> = rail_b.into_iter().map(Into::into).collect();
		if rail_a.len() != rail_b.len() || rail_a.len() < 2 {
			debug_assert!(false, "FittedRectangularStrip::from_lines requires equal lengths >= 2");
			return Self::new(style);
		}
		let mut strip = Self::new(style);
		for (a, b) in rail_a.into_iter().zip(rail_b) {
			strip.add_pair(a, b);
		}
		strip
	}

	pub fn add_pair(
		&mut self,
		rail_a: impl Into<PanelPoint>,
		rail_b: impl Into<PanelPoint>,
	) -> &mut Self {
		let rail_a = rail_a.into();
		let rail_b = rail_b.into();
		self.authored.push((rail_a, rail_b));
		if self.authored.len() == 1 {
			return self;
		}
		let (prev_a, prev_b) = self.authored[self.authored.len() - 2];
		self.bays.push(FittedRectangle::new(self.style, prev_a, rail_a, prev_b, rail_b));
		self
	}

	pub fn bays(&self) -> &[FittedRectangle] {
		&self.bays
	}

	pub fn authored_stations(&self) -> &[(PanelPoint, PanelPoint)] {
		&self.authored
	}

	pub fn joint_nodes(&self) -> Vec<JointNode> {
		let mut out = Vec::new();
		for i in 0..self.bays.len().saturating_sub(1) {
			let prev = &self.bays[i];
			let next = &self.bays[i + 1];
			let thickness = (prev.end_thickness() + next.start_thickness()) * 0.5;
			if let Some(j) = joint_along_fitted_bay_crease(
				&prev.fitted,
				&next.fitted,
				thickness,
				self.joint_policy,
			) {
				out.push(j);
			}
		}
		out
	}
}

impl BuildingComponents for FittedRectangularStrip {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for bay in &self.bays {
			out.extend(bay.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<JointNode> {
		Layers::from_free(self.joint_nodes())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use building_components::panels::PanelGeometry;

	#[test]
	fn three_stations_two_rectangle_kits() {
		let a = [Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 4.0)];
		let b = [Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 2.0), Vec3::new(2.0, 0.0, 4.0)];
		let s = FittedRectangularStrip::from_lines(PanelStyle::RoughStonework, a, b);
		assert_eq!(s.bays().len(), 2);
		assert!(s
			.bays()
			.iter()
			.all(|b| matches!(b.panel_node().geometry, PanelGeometry::Rectangle(_))));
		assert_eq!(s.authored_stations().len(), 3);
		assert!(s.joint_nodes().is_empty());
	}

	#[test]
	fn folded_strip_emits_crease_joint() {
		let a = [Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 4.0)];
		let b = [Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 2.0), Vec3::new(2.0, 1.5, 4.0)];
		let s = FittedRectangularStrip::from_lines(PanelStyle::RoughStonework, a, b);
		assert_eq!(s.joint_nodes().len(), 1);
		let muted = s.clone().with_joint_policy(PanelComplexJointPolicy::never());
		assert!(muted.joint_nodes().is_empty());
	}
}
