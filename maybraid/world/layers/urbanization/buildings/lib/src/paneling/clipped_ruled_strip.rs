//! Ruled strip with per-bay optional clips.
//!
//! Stations are authored world points. Contiguous solid bays accumulate into one
//! [`PanelComplex`]; each clipped bay becomes a [`ClippedQuadPanel`]. Present by
//! flattening piece complexes via [`BuildingComponents`].

use bevy_math::Vec3;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::clipped_quad_panel::ClippedQuadPanel;
use crate::paneling::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, PanelPoint, PanelPointId,
};

/// One flushed region of a [`ClippedRuledStrip`].
#[derive(Debug, Clone, PartialEq)]
pub enum ClippedStripPiece {
	/// One or more contiguous solid bays.
	Solid(PanelComplex),
	/// Exactly one clipped bay.
	Clipped(ClippedQuadPanel),
}

impl ClippedStripPiece {
	pub fn as_complex(&self) -> &PanelComplex {
		match self {
			Self::Solid(c) => c,
			Self::Clipped(q) => q.as_complex(),
		}
	}
}

/// Equal-station ruled strip with optional per-bay clips.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedRuledStrip {
	style: PanelStyle,
	joint_policy: PanelComplexJointPolicy,
	/// Authored stations `(rail_a, rail_b)` in world space.
	authored: Vec<(PanelPoint, PanelPoint)>,
	pieces: Vec<ClippedStripPiece>,
	open_solid: Option<PanelComplex>,
	/// Station ids inside [`Self::open_solid`] only.
	open_station_ids: Vec<(PanelPointId, PanelPointId)>,
}

impl ClippedRuledStrip {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			authored: Vec::new(),
			pieces: Vec::new(),
			open_solid: None,
			open_station_ids: Vec::new(),
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
		if let Some(c) = self.open_solid.as_mut() {
			c.set_joint_policy(joint_policy);
		}
		for piece in &mut self.pieces {
			match piece {
				ClippedStripPiece::Solid(c) => {
					c.set_joint_policy(joint_policy);
				}
				ClippedStripPiece::Clipped(q) => {
					q.set_joint_policy(joint_policy);
				}
			}
		}
		self
	}

	/// Bulk construct. `clips.len()` must equal bay count (`stations - 1`); pad/truncate
	/// with `debug_assert` on mismatch.
	pub fn from_lines(
		style: PanelStyle,
		rail_a: impl IntoIterator<Item = impl Into<PanelPoint>>,
		rail_b: impl IntoIterator<Item = impl Into<PanelPoint>>,
		clips: impl IntoIterator<Item = Option<Vec<Vec3>>>,
	) -> Self {
		let rail_a: Vec<PanelPoint> = rail_a.into_iter().map(Into::into).collect();
		let rail_b: Vec<PanelPoint> = rail_b.into_iter().map(Into::into).collect();
		let mut clips: Vec<Option<Vec<Vec3>>> = clips.into_iter().collect();
		if rail_a.len() != rail_b.len() || rail_a.len() < 2 {
			debug_assert!(
				false,
				"ClippedRuledStrip::from_lines requires equal lengths >= 2 (got a={}, b={})",
				rail_a.len(),
				rail_b.len()
			);
			return Self::new(style);
		}
		let bay_count = rail_a.len() - 1;
		if clips.len() != bay_count {
			debug_assert!(
				false,
				"ClippedRuledStrip::from_lines clips.len()={} != bay_count={}",
				clips.len(),
				bay_count
			);
			clips.resize_with(bay_count, || None);
		}
		let mut strip = Self::new(style);
		strip.add_pair(rail_a[0], rail_b[0], None::<Vec<Vec3>>);
		for i in 0..bay_count {
			strip.add_pair(rail_a[i + 1], rail_b[i + 1], clips[i].clone());
		}
		strip.finish();
		strip
	}

	/// Append a station. When a previous station exists, forms a bay; `clip` of `Some`
	/// makes that bay a [`ClippedQuadPanel`], otherwise appends to the open solid complex.
	///
	/// The `clip` argument applies to the **new bay** (previous→current). It is ignored
	/// for the first station (no bay yet).
	pub fn add_pair(
		&mut self,
		rail_a: impl Into<PanelPoint>,
		rail_b: impl Into<PanelPoint>,
		clip: Option<impl IntoIterator<Item = impl Into<Vec3>>>,
	) -> &mut Self {
		let rail_a = rail_a.into();
		let rail_b = rail_b.into();
		let clip: Option<Vec<Vec3>> = clip.map(|c| c.into_iter().map(Into::into).collect());

		if self.authored.is_empty() {
			self.authored.push((rail_a, rail_b));
			return self;
		}

		let (prev_a, prev_b) = *self.authored.last().unwrap();
		self.authored.push((rail_a, rail_b));

		match clip {
			None => self.append_solid_bay(prev_a, rail_a, prev_b, rail_b),
			Some(c) => {
				self.flush_open_solid();
				let q = ClippedQuadPanel::new(self.style, prev_a, rail_a, prev_b, rail_b, c)
					.with_joint_policy(self.joint_policy);
				self.pieces.push(ClippedStripPiece::Clipped(q));
				self
			}
		}
	}

	/// Flush any open solid run into [`Self::pieces`].
	pub fn finish(&mut self) -> &mut Self {
		self.flush_open_solid();
		self
	}

	pub fn pieces(&self) -> &[ClippedStripPiece] {
		&self.pieces
	}

	pub fn authored_stations(&self) -> &[(PanelPoint, PanelPoint)] {
		&self.authored
	}

	pub fn rail_a_polyline(&self) -> Vec<PanelPoint> {
		self.authored.iter().map(|(a, _)| *a).collect()
	}

	pub fn rail_b_polyline(&self) -> Vec<PanelPoint> {
		self.authored.iter().map(|(_, b)| *b).collect()
	}

	fn append_solid_bay(
		&mut self,
		prev_a: PanelPoint,
		curr_a: PanelPoint,
		prev_b: PanelPoint,
		curr_b: PanelPoint,
	) -> &mut Self {
		if self.open_solid.is_none() {
			let mut c = PanelComplex::new(self.style).with_joint_policy(self.joint_policy);
			let id_pa = c.insert_point_thick(prev_a.position, prev_a.thickness);
			let id_pb = c.insert_point_thick(prev_b.position, prev_b.thickness);
			self.open_station_ids.push((id_pa, id_pb));
			self.open_solid = Some(c);
		}
		let complex = self.open_solid.as_mut().unwrap();
		let (prev_e, prev_r) = *self.open_station_ids.last().unwrap();
		let id_a = complex.insert_point_thick(curr_a.position, curr_a.thickness);
		let id_b = complex.insert_point_thick(curr_b.position, curr_b.thickness);
		// Bay: {rail_a[i], rail_a[i+1], rail_b[i], rail_b[i+1]} = (prev_e, id_a, prev_r, id_b)
		complex.add_quad(prev_e, id_a, prev_r, id_b);
		self.open_station_ids.push((id_a, id_b));
		self
	}

	fn flush_open_solid(&mut self) {
		if let Some(c) = self.open_solid.take() {
			if !c.triangles().is_empty() {
				self.pieces.push(ClippedStripPiece::Solid(c));
			}
			self.open_station_ids.clear();
		}
	}
}

impl BuildingComponents for ClippedRuledStrip {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for piece in &self.pieces {
			out.extend(piece.as_complex().panel_nodes_for_level(level));
		}
		// Also present any unflushed open solid (callers should `finish`, but be forgiving).
		if let Some(c) = &self.open_solid {
			out.extend(c.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for piece in &self.pieces {
			out.extend(piece.as_complex().joint_nodes_for_level(level));
		}
		if let Some(c) = &self.open_solid {
			out.extend(c.joint_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn rails_3() -> (Vec<Vec3>, Vec<Vec3>) {
		let a = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 4.0)];
		let b = vec![Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 2.0), Vec3::new(2.0, 0.0, 4.0)];
		(a, b)
	}

	#[test]
	fn all_solid_one_piece() {
		let (a, b) = rails_3();
		let strip = ClippedRuledStrip::from_lines(PanelStyle::RoughStonework, a, b, [None, None]);
		assert_eq!(strip.pieces().len(), 1);
		assert!(matches!(strip.pieces()[0], ClippedStripPiece::Solid(_)));
		assert_eq!(strip.pieces()[0].as_complex().triangles().len(), 4); // 2 bays × 2
	}

	#[test]
	fn middle_clip_splits_pieces() {
		let (a, b) = rails_3();
		// 3 stations → 2 bays; clip only bay 0
		let clip = vec![
			Vec3::new(0.5, 0.0, 0.5),
			Vec3::new(1.5, 0.0, 0.5),
			Vec3::new(1.5, 0.0, 1.5),
			Vec3::new(0.5, 0.0, 1.5),
		];
		let strip =
			ClippedRuledStrip::from_lines(PanelStyle::RoughStonework, a, b, [Some(clip), None]);
		assert_eq!(strip.pieces().len(), 2);
		assert!(matches!(strip.pieces()[0], ClippedStripPiece::Clipped(_)));
		assert!(matches!(strip.pieces()[1], ClippedStripPiece::Solid(_)));
		assert_eq!(strip.rail_a_polyline().len(), 3);
	}

	#[test]
	fn add_pair_finish_matches_from_lines() {
		let (a, b) = rails_3();
		let mut via = ClippedRuledStrip::rough_stone();
		via.add_pair(a[0], b[0], None::<Vec<Vec3>>);
		via.add_pair(a[1], b[1], None::<Vec<Vec3>>);
		via.add_pair(a[2], b[2], None::<Vec<Vec3>>);
		via.finish();
		let bulk = ClippedRuledStrip::from_lines(PanelStyle::RoughStonework, a, b, [None, None]);
		assert_eq!(via.pieces().len(), bulk.pieces().len());
		assert_eq!(
			via.pieces()[0].as_complex().triangles().len(),
			bulk.pieces()[0].as_complex().triangles().len()
		);
	}
}
