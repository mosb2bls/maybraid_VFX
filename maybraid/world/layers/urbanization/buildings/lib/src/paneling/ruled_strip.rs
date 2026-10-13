//! Ruled strip: equal-length polylines with implicit index generators.
//!
//! Station \(i\) connects `rail_a[i]` ↔ `rail_b[i]`. Bay \(i\) is the quad
//! `{rail_a[i], rail_a[i+1], rail_b[i], rail_b[i+1]}` (diagonal \(a_0\)–\(b_1\)).
//!
//! Geometry lives in an owned [`PanelComplex`]; [`Self::stations`] keeps rail point
//! ids so both polylines remain queryable. Originally motivated by roof pitches
//! (see [`crate::RuledPitch`] for the eave / ridge convention); applicable anywhere
//! two equal-sampled rails need a ruled quad strip.

use building_components::panels::PanelStyle;

use crate::paneling::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, PanelPoint, PanelPointId,
};

/// Equal-station ruled strip between two polylines, backed by a [`PanelComplex`].
#[derive(Debug, Clone, PartialEq)]
pub struct RuledStrip {
	complex: PanelComplex,
	/// Parallel stations `(rail_a_id, rail_b_id)`.
	stations: Vec<(PanelPointId, PanelPointId)>,
}

impl RuledStrip {
	/// Empty strip (no stations). Style + default joint policy on the complex.
	pub fn new(style: PanelStyle) -> Self {
		Self { complex: PanelComplex::new(style), stations: Vec::new() }
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	/// Bulk construct: require `rail_a.len() == rail_b.len() >= 2`.
	///
	/// On mismatch or too-short inputs: `debug_assert` and return an empty strip
	/// with the given style (infallible presentation path).
	pub fn from_lines(
		style: PanelStyle,
		rail_a: impl IntoIterator<Item = impl Into<PanelPoint>>,
		rail_b: impl IntoIterator<Item = impl Into<PanelPoint>>,
	) -> Self {
		let rail_a: Vec<PanelPoint> = rail_a.into_iter().map(Into::into).collect();
		let rail_b: Vec<PanelPoint> = rail_b.into_iter().map(Into::into).collect();
		if rail_a.len() != rail_b.len() || rail_a.len() < 2 {
			debug_assert!(
				false,
				"RuledStrip::from_lines requires equal lengths >= 2 (got rail_a={}, rail_b={})",
				rail_a.len(),
				rail_b.len()
			);
			return Self::new(style);
		}
		let mut strip = Self::new(style);
		for (a, b) in rail_a.into_iter().zip(rail_b) {
			strip.add_pair(a, b);
		}
		strip
	}

	/// Sugar: rebuild from lines keeping current style and joint policy.
	pub fn with_lines(
		self,
		rail_a: impl IntoIterator<Item = impl Into<PanelPoint>>,
		rail_b: impl IntoIterator<Item = impl Into<PanelPoint>>,
	) -> Self {
		let style = self.complex.style;
		let policy = self.complex.joint_policy;
		Self::from_lines(style, rail_a, rail_b).with_joint_policy(policy)
	}

	/// Append one rail_a / rail_b station. When a previous station exists, adds the bay quad.
	pub fn add_pair(
		&mut self,
		rail_a: impl Into<PanelPoint>,
		rail_b: impl Into<PanelPoint>,
	) -> &mut Self {
		let rail_a = rail_a.into();
		let rail_b = rail_b.into();
		let a_id = self.complex.insert_point_thick(rail_a.position, rail_a.thickness);
		let b_id = self.complex.insert_point_thick(rail_b.position, rail_b.thickness);
		if let Some(&(prev_a, prev_b)) = self.stations.last() {
			// Bay: {rail_a[i], rail_a[i+1], rail_b[i], rail_b[i+1]}
			self.complex.add_quad(prev_a, a_id, prev_b, b_id);
		}
		self.stations.push((a_id, b_id));
		self
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.complex = self.complex.with_joint_policy(joint_policy);
		self
	}

	pub fn set_joint_policy(&mut self, joint_policy: PanelComplexJointPolicy) -> &mut Self {
		self.complex.set_joint_policy(joint_policy);
		self
	}

	pub fn stations(&self) -> &[(PanelPointId, PanelPointId)] {
		&self.stations
	}

	pub fn rail_a_ids(&self) -> impl Iterator<Item = PanelPointId> + '_ {
		self.stations.iter().map(|(a, _)| *a)
	}

	pub fn rail_b_ids(&self) -> impl Iterator<Item = PanelPointId> + '_ {
		self.stations.iter().map(|(_, b)| *b)
	}

	pub fn rail_a_polyline(&self) -> Vec<PanelPoint> {
		self.rail_a_ids().filter_map(|id| self.complex.point(id).copied()).collect()
	}

	pub fn rail_b_polyline(&self) -> Vec<PanelPoint> {
		self.rail_b_ids().filter_map(|id| self.complex.point(id).copied()).collect()
	}

	pub fn as_complex(&self) -> &PanelComplex {
		&self.complex
	}

	pub fn into_complex(self) -> PanelComplex {
		self.complex
	}

	pub fn into_parts(self) -> (PanelComplex, Vec<(PanelPointId, PanelPointId)>) {
		(self.complex, self.stations)
	}
}

impl AsRef<PanelComplex> for RuledStrip {
	fn as_ref(&self) -> &PanelComplex {
		&self.complex
	}
}

impl From<RuledStrip> for PanelComplex {
	fn from(value: RuledStrip) -> Self {
		value.into_complex()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use building_components::BuildingComponents;
	use lod::scene::LodSceneLevel;

	fn example_lines() -> (Vec<Vec3>, Vec<Vec3>) {
		let rail_a =
			vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 4.0)];
		let rail_b =
			vec![Vec3::new(1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 2.0), Vec3::new(1.0, 1.0, 4.0)];
		(rail_a, rail_b)
	}

	#[test]
	fn from_lines_builds_two_quads_and_retrieves_polylines() {
		let (rail_a, rail_b) = example_lines();
		let strip =
			RuledStrip::from_lines(PanelStyle::ShepherdsThatch, rail_a.clone(), rail_b.clone());
		assert_eq!(strip.stations().len(), 3);
		assert_eq!(strip.as_complex().triangles().len(), 4); // 2 quads × 2 tris
		let got_a = strip.rail_a_polyline();
		let got_b = strip.rail_b_polyline();
		assert_eq!(got_a.len(), 3);
		assert_eq!(got_b.len(), 3);
		for (a, b) in got_a.iter().zip(rail_a) {
			assert!((a.position - b).length() < 1e-5);
		}
		for (a, b) in got_b.iter().zip(rail_b) {
			assert!((a.position - b).length() < 1e-5);
		}
	}

	#[test]
	fn add_pair_matches_from_lines_topology() {
		let (rail_a, rail_b) = example_lines();
		let mut via_pairs = RuledStrip::shepherds_thatch();
		for (a, b) in rail_a.iter().copied().zip(rail_b.iter().copied()) {
			via_pairs.add_pair(a, b);
		}
		let via_lines = RuledStrip::from_lines(PanelStyle::ShepherdsThatch, rail_a, rail_b);
		assert_eq!(via_pairs.stations().len(), via_lines.stations().len());
		assert_eq!(
			via_pairs.as_complex().triangles().len(),
			via_lines.as_complex().triangles().len()
		);
		assert_eq!(
			via_pairs.as_complex().shared_edges().len(),
			via_lines.as_complex().shared_edges().len()
		);
	}

	#[test]
	fn interior_generators_are_shared_edges() {
		let (rail_a, rail_b) = example_lines();
		let strip = RuledStrip::from_lines(PanelStyle::RoughStonework, rail_a, rail_b);
		// 2 bays → 2 diagonals + 1 interior generator = 3 shared edges.
		assert_eq!(strip.as_complex().shared_edges().len(), 3);
		let mid = strip.stations()[1];
		let shared = strip.as_complex().shared_edges();
		assert!(
			shared.iter().any(|e| {
				let (a, b) = e.endpoints();
				(a, b) == mid || (b, a) == mid
			}),
			"expected shared generator at station 1 {mid:?}, got {shared:?}"
		);
	}

	#[test]
	fn never_policy_suppresses_joints() {
		let (rail_a, rail_b) = example_lines();
		let c = RuledStrip::from_lines(PanelStyle::ShepherdsThatch, rail_a, rail_b)
			.with_joint_policy(PanelComplexJointPolicy::never())
			.into_complex();
		assert!(c.joint_nodes().is_empty());
	}

	#[test]
	#[cfg(not(debug_assertions))]
	fn mismatched_from_lines_yields_empty() {
		let strip =
			RuledStrip::from_lines(PanelStyle::RoughStonework, [Vec3::ZERO, Vec3::X], [Vec3::Y]);
		assert!(strip.stations().is_empty());
		assert!(strip.as_complex().triangles().is_empty());
	}

	#[test]
	#[cfg(debug_assertions)]
	#[should_panic(expected = "RuledStrip::from_lines requires equal lengths >= 2")]
	fn mismatched_from_lines_debug_asserts() {
		let _ =
			RuledStrip::from_lines(PanelStyle::RoughStonework, [Vec3::ZERO, Vec3::X], [Vec3::Y]);
	}

	#[test]
	fn into_complex_escape_hatch() {
		let (rail_a, rail_b) = example_lines();
		let mut c =
			RuledStrip::from_lines(PanelStyle::RoughStonework, rail_a, rail_b).into_complex();
		let a = c.insert_point(Vec3::new(2.0, 0.0, 0.0));
		let b = c.insert_point(Vec3::new(2.0, 1.0, 0.0));
		let d = c.insert_point(Vec3::new(2.0, 0.0, 1.0));
		c.add_triangle(a, b, d);
		assert_eq!(
			c.panel_nodes_for_level(LodSceneLevel::High).flatten().len(),
			5 // 4 from strip + 1 extra
		);
	}

	#[test]
	fn into_parts_keeps_stations() {
		let (rail_a, rail_b) = example_lines();
		let strip = RuledStrip::from_lines(PanelStyle::RoughStonework, rail_a, rail_b);
		let (complex, stations) = strip.into_parts();
		assert_eq!(stations.len(), 3);
		assert_eq!(complex.triangles().len(), 4);
	}
}
