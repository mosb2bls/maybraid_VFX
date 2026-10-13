//! Two-rail strip of best-fit rectangles with optional per-bay inset openings + crease joints.

use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::{PanelComplexJointPolicy, PanelPoint};
use crate::paneling::rect_crease::joint_along_fitted_bay_crease;
use crate::paneling::rect_fit::{FittedRect, RectInset};

/// One bay of a [`ClippedFittedRectangularStrip`].
#[derive(Debug, Clone, PartialEq)]
pub enum ClippedFittedRectangularStripPiece {
	Solid(FittedRectangle),
	Clipped(ClippedFittedRectangle),
}

impl ClippedFittedRectangularStripPiece {
	pub fn panels(&self) -> Vec<&PanelNode> {
		match self {
			Self::Solid(r) => vec![r.panel_node()],
			Self::Clipped(r) => r.panels().iter().collect(),
		}
	}

	pub fn fitted(&self) -> &FittedRect {
		match self {
			Self::Solid(r) => &r.fitted,
			Self::Clipped(r) => &r.fitted,
		}
	}

	pub fn start_thickness(&self) -> f32 {
		match self {
			Self::Solid(r) => r.start_thickness(),
			Self::Clipped(r) => r.start_thickness(),
		}
	}

	pub fn end_thickness(&self) -> f32 {
		match self {
			Self::Solid(r) => r.end_thickness(),
			Self::Clipped(r) => r.end_thickness(),
		}
	}
}

/// Two-rail rectangular strip with optional per-bay [`RectInset`] openings.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedFittedRectangularStrip {
	style: PanelStyle,
	joint_policy: PanelComplexJointPolicy,
	authored: Vec<(PanelPoint, PanelPoint)>,
	pieces: Vec<ClippedFittedRectangularStripPiece>,
}

impl ClippedFittedRectangularStrip {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			authored: Vec::new(),
			pieces: Vec::new(),
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
		insets: impl IntoIterator<Item = Option<RectInset>>,
	) -> Self {
		let rail_a: Vec<PanelPoint> = rail_a.into_iter().map(Into::into).collect();
		let rail_b: Vec<PanelPoint> = rail_b.into_iter().map(Into::into).collect();
		let mut insets: Vec<Option<RectInset>> = insets.into_iter().collect();
		if rail_a.len() != rail_b.len() || rail_a.len() < 2 {
			debug_assert!(false, "ClippedFittedRectangularStrip::from_lines bad rail lengths");
			return Self::new(style);
		}
		let bay_count = rail_a.len() - 1;
		if insets.len() != bay_count {
			debug_assert!(false, "ClippedFittedRectangularStrip::from_lines insets/bay mismatch");
			insets.resize_with(bay_count, || None);
		}
		let mut strip = Self::new(style);
		strip.add_pair(rail_a[0], rail_b[0], None);
		for i in 0..bay_count {
			strip.add_pair(rail_a[i + 1], rail_b[i + 1], insets[i]);
		}
		strip
	}

	pub fn add_pair(
		&mut self,
		rail_a: impl Into<PanelPoint>,
		rail_b: impl Into<PanelPoint>,
		inset: Option<RectInset>,
	) -> &mut Self {
		let rail_a = rail_a.into();
		let rail_b = rail_b.into();

		if self.authored.is_empty() {
			self.authored.push((rail_a, rail_b));
			return self;
		}
		let (prev_a, prev_b) = *self.authored.last().unwrap();
		self.authored.push((rail_a, rail_b));

		match inset {
			None => {
				self.pieces.push(ClippedFittedRectangularStripPiece::Solid(FittedRectangle::new(
					self.style, prev_a, rail_a, prev_b, rail_b,
				)));
			}
			Some(inset) => {
				self.pieces.push(ClippedFittedRectangularStripPiece::Clipped(
					ClippedFittedRectangle::new(self.style, prev_a, rail_a, prev_b, rail_b, inset),
				));
			}
		}
		self
	}

	pub fn pieces(&self) -> &[ClippedFittedRectangularStripPiece] {
		&self.pieces
	}

	pub fn authored_stations(&self) -> &[(PanelPoint, PanelPoint)] {
		&self.authored
	}

	pub fn joint_nodes(&self) -> Vec<JointNode> {
		let mut out = Vec::new();
		for i in 0..self.pieces.len().saturating_sub(1) {
			let prev = &self.pieces[i];
			let next = &self.pieces[i + 1];
			let thickness = (prev.end_thickness() + next.start_thickness()) * 0.5;
			if let Some(j) = joint_along_fitted_bay_crease(
				prev.fitted(),
				next.fitted(),
				thickness,
				self.joint_policy,
			) {
				out.push(j);
			}
		}
		out
	}
}

impl BuildingComponents for ClippedFittedRectangularStrip {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for piece in &self.pieces {
			match piece {
				ClippedFittedRectangularStripPiece::Solid(r) => {
					out.extend(r.panel_nodes_for_level(level));
				}
				ClippedFittedRectangularStripPiece::Clipped(r) => {
					out.extend(r.panel_nodes_for_level(level));
				}
			}
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
	fn middle_inset_splits_pieces() {
		let a = [Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 4.0)];
		let b = [Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 2.0), Vec3::new(2.0, 0.0, 4.0)];
		let s = ClippedFittedRectangularStrip::from_lines(
			PanelStyle::RoughStonework,
			a,
			b,
			[Some(RectInset::uniform(0.35)), None],
		);
		assert_eq!(s.pieces().len(), 2);
		assert!(matches!(s.pieces()[0], ClippedFittedRectangularStripPiece::Clipped(_)));
		assert!(matches!(s.pieces()[1], ClippedFittedRectangularStripPiece::Solid(_)));
		let clipped = match &s.pieces()[0] {
			ClippedFittedRectangularStripPiece::Clipped(c) => c,
			_ => unreachable!(),
		};
		assert_eq!(clipped.panels().len(), 4);
		assert!(clipped
			.panels()
			.iter()
			.all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
	}

	#[test]
	fn folded_strip_emits_crease_joint() {
		let a = [
			Vec3::ZERO,
			Vec3::new(0.0, 0.0, 2.0),
			Vec3::new(0.0, 0.0, 4.0),
			Vec3::new(0.0, 0.0, 6.0),
		];
		let b = [
			Vec3::new(2.5, 0.0, 0.0),
			Vec3::new(2.5, 0.0, 2.0),
			Vec3::new(2.5, 1.2, 4.0),
			Vec3::new(2.5, 1.2, 6.0),
		];
		let s = ClippedFittedRectangularStrip::from_lines(
			PanelStyle::RoughStonework,
			a,
			b,
			[None, Some(RectInset::uniform(0.35)), None],
		)
		.with_joint_policy(PanelComplexJointPolicy::default());
		assert!(!s.joint_nodes().is_empty());
	}
}
