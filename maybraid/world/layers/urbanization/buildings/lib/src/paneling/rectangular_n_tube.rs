//! Rectangular n-tube: polyline of closed n-point cross-sections.
//!
//! Each station has exactly `n` corners (`n >= 3`). Between stations `A` and `B`,
//! face `i` is an oriented rectangle whose lowest edge is `A[i] → B[i]`, height
//! is `|A[(i+1)%n] - A[i]|`, and roll aligns height with that cross-section edge.
//! Face strips wrap around the cross-section.
//!
//! Cross-section edges may be omitted from presentation via
//! [`RectangularNTube::without_face_edges`]: index `i` means the face on
//! `a_i → a_{(i+1)%n}` (e.g. floor/ceiling on a square tube). Geometry for every
//! face is still authored; disabled faces skip panel/joint emission only.

use bevy_math::Vec3;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::rect_fit::{roll_to_align_height, RectInset};
use crate::paneling::rectangular_strip::RectangularStripNode;

/// One corner of a closed cross-section station.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularNTubeCorner {
	pub position: Vec3,
	pub thickness: f32,
}

impl RectangularNTubeCorner {
	pub fn new(position: Vec3, thickness: f32) -> Self {
		Self { position, thickness }
	}
}

/// One authored station: a closed n-gon of corners.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularNTubeStation {
	pub corners: Vec<RectangularNTubeCorner>,
}

impl RectangularNTubeStation {
	pub fn new(corners: impl IntoIterator<Item = RectangularNTubeCorner>) -> Self {
		Self { corners: corners.into_iter().collect() }
	}

	pub fn n(&self) -> usize {
		self.corners.len()
	}
}

/// Polyline tube: `n` [`ClippedRectangularStrip`] faces from closed n-gon stations.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularNTube {
	style: PanelStyle,
	joint_policy: PanelComplexJointPolicy,
	stations: Vec<RectangularNTubeStation>,
	faces: Vec<ClippedRectangularStrip>,
	/// Per face index: whether [`BuildingComponents`] emits panels/joints.
	/// Face `i` is the strip along cross-section edge `corners[i] → corners[(i+1)%n]`.
	emit_face: Vec<bool>,
}

impl RectangularNTube {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			stations: Vec::new(),
			faces: Vec::new(),
			emit_face: Vec::new(),
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	/// Bulk construct with solid faces (no bay insets).
	pub fn from_stations(
		style: PanelStyle,
		stations: impl IntoIterator<Item = RectangularNTubeStation>,
	) -> Self {
		Self::from_stations_with_insets(style, stations, std::iter::empty())
	}

	/// Bulk construct. `face_insets` is outer length `n` (one per face); each
	/// inner list matches bay count (`stations - 1`). Empty or mismatched lengths
	/// pad/truncate with `debug_assert`.
	pub fn from_stations_with_insets(
		style: PanelStyle,
		stations: impl IntoIterator<Item = RectangularNTubeStation>,
		face_insets: impl IntoIterator<Item = Vec<Option<RectInset>>>,
	) -> Self {
		let stations: Vec<RectangularNTubeStation> = stations.into_iter().collect();
		if stations.len() < 2 {
			debug_assert!(
				false,
				"RectangularNTube::from_stations requires at least 2 stations (got {})",
				stations.len()
			);
			return Self::new(style);
		}
		let n = stations[0].n();
		if n < 3 {
			debug_assert!(false, "RectangularNTube::from_stations requires n >= 3 (got {})", n);
			return Self::new(style);
		}
		if stations.iter().any(|s| s.n() != n) {
			debug_assert!(false, "RectangularNTube::from_stations requires equal corner counts");
			return Self::new(style);
		}

		let bay_count = stations.len() - 1;
		let mut face_insets: Vec<Vec<Option<RectInset>>> = face_insets.into_iter().collect();
		if face_insets.is_empty() {
			face_insets = (0..n).map(|_| vec![None; bay_count]).collect();
		} else if face_insets.len() != n {
			debug_assert!(
				false,
				"RectangularNTube face_insets.len()={} != n={}",
				face_insets.len(),
				n
			);
			face_insets.resize_with(n, || vec![None; bay_count]);
		}
		for (fi, insets) in face_insets.iter_mut().enumerate() {
			if insets.len() != bay_count {
				debug_assert!(
					false,
					"RectangularNTube face {} insets.len()={} != bay_count={}",
					fi,
					insets.len(),
					bay_count
				);
				insets.resize_with(bay_count, || None);
			}
		}

		let mut faces = Vec::with_capacity(n);
		for i in 0..n {
			let mut strip_nodes: Vec<RectangularStripNode> = Vec::with_capacity(stations.len());
			for (k, station) in stations.iter().enumerate() {
				let a = &station.corners[i];
				let a_next = &station.corners[(i + 1) % n];
				let height_dir = a_next.position - a.position;
				let height = height_dir.length().max(1e-4);
				let roll = if k < bay_count {
					let edge = stations[k + 1].corners[i].position - a.position;
					if edge.length_squared() > 1e-12 {
						roll_to_align_height(edge.normalize(), height_dir).unwrap_or(0.0)
					} else {
						0.0
					}
				} else if let Some(prev) = strip_nodes.last() {
					// Last station: unused for face emission; mirror prior outbound roll.
					prev.roll
				} else {
					0.0
				};
				strip_nodes.push(RectangularStripNode {
					position: a.position,
					height,
					thickness: a.thickness,
					roll,
				});
			}

			faces.push(ClippedRectangularStrip::from_nodes(
				style,
				strip_nodes,
				face_insets[i].clone(),
			));
		}

		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			stations,
			faces,
			emit_face: vec![true; n],
		}
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self.faces = self.faces.into_iter().map(|f| f.with_joint_policy(joint_policy)).collect();
		self
	}

	/// Omit faces on the listed cross-section edges from panel/joint emission.
	///
	/// Index `i` is the face along `corners[i] → corners[(i+1)%n]`. Geometry for
	/// every face remains in [`Self::faces`]. Out-of-range indices are ignored
	/// (`debug_assert` in debug builds).
	pub fn without_face_edges(mut self, edges: impl IntoIterator<Item = usize>) -> Self {
		let n = self.faces.len();
		for i in edges {
			if i < n {
				self.emit_face[i] = false;
			} else {
				debug_assert!(false, "RectangularNTube::without_face_edges index {i} >= n={n}");
			}
		}
		self
	}

	/// Whether face `i` (`corners[i] → corners[(i+1)%n]`) emits panels/joints.
	pub fn emits_face(&self, i: usize) -> bool {
		self.emit_face.get(i).copied().unwrap_or(false)
	}

	/// Cross-section edge indices currently omitted from presentation.
	pub fn omitted_face_edges(&self) -> impl Iterator<Item = usize> + '_ {
		self.emit_face.iter().enumerate().filter_map(|(i, emit)| (!emit).then_some(i))
	}

	pub fn stations(&self) -> &[RectangularNTubeStation] {
		&self.stations
	}

	pub fn faces(&self) -> &[ClippedRectangularStrip] {
		&self.faces
	}

	pub fn n(&self) -> usize {
		self.faces.len()
	}
}

impl BuildingComponents for RectangularNTube {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for (i, face) in self.faces.iter().enumerate() {
			if self.emits_face(i) {
				out.extend(face.panel_nodes_for_level(level));
			}
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for (i, face) in self.faces.iter().enumerate() {
			if self.emits_face(i) {
				out.extend(face.joint_nodes_for_level(level));
			}
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::paneling::clipped_rectangular_strip::ClippedRectangularStripPiece;
	use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;
	use building_components::panels::PanelGeometry;

	fn approx_eq(a: Vec3, b: Vec3) -> bool {
		(a - b).length() < 1e-3
	}

	fn square_station(z: f32, half_w: f32, height: f32) -> RectangularNTubeStation {
		let t = DEFAULT_PANEL_THICKNESS;
		// CCW when looking along +Z: floor at y=0, ceiling at y=height.
		RectangularNTubeStation::new([
			RectangularNTubeCorner::new(Vec3::new(-half_w, 0.0, z), t),
			RectangularNTubeCorner::new(Vec3::new(half_w, 0.0, z), t),
			RectangularNTubeCorner::new(Vec3::new(half_w, height, z), t),
			RectangularNTubeCorner::new(Vec3::new(-half_w, height, z), t),
		])
	}

	#[test]
	fn square_tube_four_faces_two_bays() {
		let tube = RectangularNTube::from_stations(
			PanelStyle::RoughStonework,
			[
				square_station(0.0, 1.0, 2.0),
				square_station(2.0, 1.0, 2.0),
				square_station(4.0, 1.0, 2.0),
			],
		);
		assert_eq!(tube.n(), 4);
		assert_eq!(tube.stations().len(), 3);
		for face in tube.faces() {
			assert_eq!(face.pieces().len(), 2);
			assert!(face
				.pieces()
				.iter()
				.flat_map(|p| p.panels())
				.all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
		}
		// Face 0: bottom, height along +X from BL→BR
		let bay0 = match &tube.faces()[0].pieces()[0] {
			ClippedRectangularStripPiece::Solid(r) => r,
			_ => unreachable!(),
		};
		assert!((bay0.height - 2.0).abs() < 1e-3);
		assert!(approx_eq(bay0.origin, Vec3::new(-1.0, 0.0, 0.0)));
		assert!(approx_eq(bay0.edge, Vec3::new(0.0, 0.0, 2.0)));
		assert!(
			bay0.oriented.e0.dot(Vec3::X) > 0.99,
			"bottom face height should align +X, e0={:?}",
			bay0.oriented.e0
		);
	}

	#[test]
	fn triangle_tube_wraps() {
		let t = DEFAULT_PANEL_THICKNESS;
		let station = |z: f32| {
			RectangularNTubeStation::new([
				RectangularNTubeCorner::new(Vec3::new(0.0, 0.0, z), t),
				RectangularNTubeCorner::new(Vec3::new(2.0, 0.0, z), t),
				RectangularNTubeCorner::new(Vec3::new(1.0, 1.5, z), t),
			])
		};
		let tube = RectangularNTube::from_stations(
			PanelStyle::RoughStonework,
			[station(0.0), station(3.0)],
		);
		assert_eq!(tube.n(), 3);
		assert_eq!(tube.faces()[2].pieces().len(), 1);
		// Face 2 wraps last→first: height from (1,1.5)→(0,0)
		let bay = match &tube.faces()[2].pieces()[0] {
			ClippedRectangularStripPiece::Solid(r) => r,
			_ => unreachable!(),
		};
		let expect_h = (Vec3::new(0.0, 0.0, 0.0) - Vec3::new(1.0, 1.5, 0.0)).length();
		assert!((bay.height - expect_h).abs() < 1e-3);
	}

	#[test]
	fn middle_bay_face_inset_clips() {
		let tube = RectangularNTube::from_stations_with_insets(
			PanelStyle::RoughStonework,
			[
				square_station(0.0, 1.0, 2.0),
				square_station(2.0, 1.0, 2.0),
				square_station(4.0, 1.0, 2.0),
				square_station(6.0, 1.0, 2.0),
			],
			[
				vec![None, None, None],
				vec![None, Some(RectInset::uniform(0.35)), None],
				vec![None, None, None],
				vec![None, None, None],
			],
		);
		assert!(matches!(tube.faces()[1].pieces()[1], ClippedRectangularStripPiece::Clipped(_)));
		let clipped = match &tube.faces()[1].pieces()[1] {
			ClippedRectangularStripPiece::Clipped(c) => c,
			_ => unreachable!(),
		};
		assert_eq!(clipped.panels().len(), 4);
	}

	#[test]
	#[cfg(not(debug_assertions))]
	fn short_input_yields_empty() {
		let tube = RectangularNTube::from_stations(
			PanelStyle::RoughStonework,
			[square_station(0.0, 1.0, 2.0)],
		);
		assert!(tube.stations().is_empty());
		assert!(tube.faces().is_empty());
	}

	#[test]
	#[cfg(debug_assertions)]
	#[should_panic(expected = "at least 2 stations")]
	fn short_input_debug_asserts() {
		let _ = RectangularNTube::from_stations(
			PanelStyle::RoughStonework,
			[square_station(0.0, 1.0, 2.0)],
		);
	}

	#[test]
	fn omitted_face_edges_skip_presentation() {
		use building_components::BuildingComponents;
		use lod::scene::LodSceneLevel;

		let full = RectangularNTube::from_stations(
			PanelStyle::RoughStonework,
			[square_station(0.0, 1.0, 2.0), square_station(2.0, 1.0, 2.0)],
		);
		// Square: 0 = floor (BL→BR), 2 = ceiling (TR→TL).
		let open = full.clone().without_face_edges([0, 2]);
		assert!(!open.emits_face(0) && !open.emits_face(2));
		assert!(open.emits_face(1) && open.emits_face(3));
		assert_eq!(open.omitted_face_edges().collect::<Vec<_>>(), vec![0, 2]);
		// Strips still authored for every cross-section edge.
		assert_eq!(open.faces().len(), 4);
		assert!(!open.faces()[0].pieces().is_empty());
		assert!(!open.faces()[2].pieces().is_empty());

		let full_n = full.panel_nodes_for_level(LodSceneLevel::High).len();
		let open_n = open.panel_nodes_for_level(LodSceneLevel::High).len();
		assert!(open_n < full_n);
		assert!(open_n > 0);
	}
}
