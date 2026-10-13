//! Wall-edge strip builders that turn opening cuts into [`ClippedRectangularStrip`] bays.

use building_components::panels::PanelStyle;

use crate::openings::{OpeningLabel, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::rect_fit::RectInset;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::shells::ortho::{standing_face_opening, WallEdge};

const EPS: f32 = 1e-3;

impl ClippedRectangularStrip {
	/// Build a wall strip along `edge`, clipping bays for [`OpeningLabel::Passage`] openings.
	pub fn from_wall_edge_with_passage_openings(
		edge: WallEdge,
		openings: &Openings,
		thickness: f32,
	) -> Self {
		let thickness = thickness.max(1e-4);
		let len = edge.length();
		let h = edge.height;
		let tang = edge.tangent();
		let style = PanelStyle::RoughStonework;

		let mut cuts: Vec<(f32, f32, f32, f32)> = Vec::new();
		for (_id, opening) in openings.iter() {
			if !matches!(opening.label, OpeningLabel::Passage) {
				continue;
			}
			let Some(face) = standing_face_opening(edge, &opening.bounds, thickness) else {
				continue;
			};
			let s_lo = face.inset.bottom.clamp(0.0, len);
			let s_hi = (len - face.inset.top).clamp(0.0, len);
			if s_hi - s_lo < EPS {
				continue;
			}
			cuts.push((s_lo, s_hi, face.inset.left, face.inset.right));
		}
		cuts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

		if cuts.is_empty() {
			return ClippedRectangularStrip::from_nodes(
				style,
				[
					RectangularStripNode::new(edge.start, h, thickness, 0.0),
					RectangularStripNode::new(edge.end, h, thickness, 0.0),
				],
				[None],
			);
		}

		let mut nodes = Vec::new();
		let mut insets: Vec<Option<RectInset>> = Vec::new();
		nodes.push(RectangularStripNode::new(edge.start, h, thickness, 0.0));
		let mut cursor = 0.0_f32;
		for (s_lo, s_hi, sill, header) in cuts {
			if s_lo > cursor + EPS {
				nodes.push(RectangularStripNode::new(edge.start + tang * s_lo, h, thickness, 0.0));
				insets.push(None);
				cursor = s_lo;
			}
			let s_hi = s_hi.max(cursor + EPS);
			nodes.push(RectangularStripNode::new(edge.start + tang * s_hi, h, thickness, 0.0));
			let jamb = 0.02_f32.min((s_hi - cursor) * 0.1);
			insets.push(Some(RectInset::new(sill, header, jamb, jamb)));
			cursor = s_hi;
		}
		if cursor < len - EPS {
			nodes.push(RectangularStripNode::new(edge.end, h, thickness, 0.0));
			insets.push(None);
		} else if let Some(last) = nodes.last_mut() {
			last.position = edge.end;
		}
		ClippedRectangularStrip::from_nodes(style, nodes, insets)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::{Opening, OpeningId, OpeningLabel};
	use bevy_math::bounding::Aabb3d;
	use bevy_math::{Vec2, Vec3};

	#[test]
	fn empty_openings_yields_solid_strip() {
		let edge = WallEdge::new(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), 2.5, Vec2::X);
		let openings = Openings::new();
		let strip =
			ClippedRectangularStrip::from_wall_edge_with_passage_openings(edge, &openings, 0.2);
		assert_eq!(strip.pieces().len(), 1);
		assert!(matches!(
			strip.pieces()[0],
			crate::paneling::clipped_rectangular_strip::ClippedRectangularStripPiece::Solid(_)
		));
	}

	#[test]
	fn passage_opening_splits_strip() {
		let edge = WallEdge::new(Vec3::ZERO, Vec3::new(6.0, 0.0, 0.0), 2.5, Vec2::X);
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("passage"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(2.0, 0.0, -0.5), Vec3::new(4.0, 2.2, 0.5)),
				OpeningLabel::Passage,
			),
		);
		let strip =
			ClippedRectangularStrip::from_wall_edge_with_passage_openings(edge, &openings, 0.2);
		assert!(strip.pieces().len() >= 2);
	}
}
