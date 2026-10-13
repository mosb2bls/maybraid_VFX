//! Per-edge wall openings for [`IFloor`].
//!
//! Connectable openings map onto the best-scoring edge by projected along-span
//! (then distance / score). **Multiple openings may land on the same edge** —
//! the wall strip is subdivided into solid and opening bays along the run
//! (same policy as [`crate::shells::RectRingFloor`]).

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use crate::openings::{MappedOpenings, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::rect_fit::RectInset;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::shells::ortho::{edge_score_for_bounds, standing_face_opening, WallEdge, EPS};

use super::{IFloor, IFloorParams};

/// Max meters a connectable leaf may shrink to fit a wall span before that edge
/// is rejected.
pub const OPENING_SPAN_TRUNCATE_MAX: f32 = 0.4;

impl IFloor {
	/// Thin passage AABB centered on a wall edge (by index into [`IFloor::edges`]).
	pub fn edge_passage_opening(edge: WallEdge, width: f32, height: f32) -> Opening {
		edge_opening_at(edge, edge.length() * 0.5, width, height, OpeningLabel::Passage, 0.0)
	}

	/// Thin aperture AABB on a wall edge with sill height (centered).
	pub fn edge_aperture_opening(edge: WallEdge, width: f32, height: f32, sill: f32) -> Opening {
		edge_opening_at(
			edge,
			edge.length() * 0.5,
			width,
			height,
			OpeningLabel::Aperture,
			sill.max(0.0),
		)
	}

	/// Passage AABB centered at `along` meters from [`WallEdge::start`].
	pub fn edge_passage_opening_at(edge: WallEdge, along: f32, width: f32, height: f32) -> Opening {
		edge_opening_at(edge, along, width, height, OpeningLabel::Passage, 0.0)
	}

	/// Aperture AABB centered at `along` meters from [`WallEdge::start`].
	pub fn edge_aperture_opening_at(
		edge: WallEdge,
		along: f32,
		width: f32,
		height: f32,
		sill: f32,
	) -> Opening {
		edge_opening_at(edge, along, width, height, OpeningLabel::Aperture, sill.max(0.0))
	}
}

fn edge_opening_at(
	edge: WallEdge,
	along: f32,
	width: f32,
	height: f32,
	label: OpeningLabel,
	sill: f32,
) -> Opening {
	let width = width.max(1e-3).min(edge.length() * 0.95);
	let height = height.max(1e-3).min(edge.height * 0.95);
	let tang = edge.tangent();
	let outward = Vec3::new(edge.outward.x, 0.0, edge.outward.y);
	let along = along.clamp(width * 0.5, (edge.length() - width * 0.5).max(width * 0.5));
	let mid = edge.start + tang * along + Vec3::Y * sill;
	let half_w = width * 0.5;
	let depth = 0.4;
	let min = mid - tang * half_w - outward * depth * 0.25;
	let max = mid + tang * half_w + outward * depth + Vec3::Y * height;
	Opening::new(Aabb3d::from_min_max(min.min(max), min.max(max)), label)
}

struct EdgeOpening {
	id: OpeningId,
	opening: Opening,
	s_lo: f32,
	s_hi: f32,
	sill: f32,
	header: f32,
	mapped: crate::openings::MappedOpening,
}

impl IFloorParams {
	pub(super) fn resolve_walls(
		&self,
		edges: &[WallEdge],
	) -> (Vec<ClippedRectangularStrip>, Openings, MappedOpenings) {
		let thickness = self.joint_thickness.max(1e-4);
		let n = edges.len();
		let mut per_edge: Vec<Vec<EdgeOpening>> = (0..n).map(|_| Vec::new()).collect();

		for (id, opening) in self.openings.iter() {
			if !opening.label.is_connectable() {
				continue;
			}
			let mut winner: Option<(usize, f32, f32, f32, EdgeOpening)> = None;
			for (i, edge) in edges.iter().enumerate() {
				if edge.length() < EPS {
					continue;
				}
				let Some(face) = standing_face_opening(*edge, &opening.bounds, thickness) else {
					continue;
				};
				let len = edge.length();
				let s_lo = face.inset.bottom.clamp(0.0, len);
				let s_hi = (len - face.inset.top).clamp(0.0, len);
				let span = s_hi - s_lo;
				if span < EPS {
					continue;
				}
				let intended = opening_along_width(&opening.bounds, *edge);
				let mut mapped_opening = opening.clone();
				if span + 1e-3 < intended {
					let shrink = intended - span;
					if shrink > OPENING_SPAN_TRUNCATE_MAX + 1e-3 {
						continue;
					}
					mapped_opening =
						truncate_opening_to_edge_span(mapped_opening, *edge, s_lo, s_hi);
				}
				let (dist, score) = edge_score_for_bounds(&mapped_opening.bounds, *edge);
				let replace = match winner {
					None => true,
					Some((_, prev_span, prev_d, prev_s, _)) => {
						span > prev_span + 0.15
							|| (span + 0.15 >= prev_span
								&& (dist < prev_d - 1e-4
									|| (dist <= prev_d + 1e-4 && score > prev_s)))
					}
				};
				if replace {
					winner = Some((
						i,
						span,
						dist,
						score,
						EdgeOpening {
							id: id.clone(),
							opening: mapped_opening,
							s_lo,
							s_hi,
							sill: face.inset.left,
							header: face.inset.right,
							mapped: face.mapped,
						},
					));
				}
			}
			let Some((idx, _, _, _, edge_op)) = winner else {
				continue;
			};
			per_edge[idx].push(edge_op);
		}

		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		let mut walls = Vec::with_capacity(n);
		for (i, edge) in edges.iter().enumerate() {
			if edge.length() < EPS {
				continue;
			}
			let mut assigned = std::mem::take(&mut per_edge[i]);
			assigned.sort_by(|a, b| {
				connectable_priority(&b.opening.label)
					.cmp(&connectable_priority(&a.opening.label))
					.then_with(|| a.s_lo.partial_cmp(&b.s_lo).unwrap_or(std::cmp::Ordering::Equal))
			});
			let mut kept = Vec::new();
			for op in assigned {
				if kept
					.iter()
					.any(|k: &EdgeOpening| spans_overlap(k.s_lo, k.s_hi, op.s_lo, op.s_hi))
				{
					continue;
				}
				kept.push(op);
			}
			kept.sort_by(|a, b| a.s_lo.partial_cmp(&b.s_lo).unwrap_or(std::cmp::Ordering::Equal));
			for op in &kept {
				mapped.insert(op.id.clone(), op.mapped.clone());
				openings.insert(op.id.clone(), op.opening.clone());
			}
			walls.push(wall_strip_for_edge(*edge, &kept, self.style, thickness));
		}

		(walls, openings, mapped)
	}
}

fn connectable_priority(label: &OpeningLabel) -> u8 {
	match label {
		OpeningLabel::Passage | OpeningLabel::Shaft => 2,
		OpeningLabel::Aperture => 1,
		_ => 0,
	}
}

fn opening_along_width(bounds: &Aabb3d, edge: WallEdge) -> f32 {
	let e = Vec3::from(bounds.max - bounds.min);
	let along_x = edge.tangent().x.abs() > edge.tangent().z.abs();
	let w = if along_x { e.x } else { e.z };
	w.max(0.0)
}

fn truncate_opening_to_edge_span(
	mut opening: Opening,
	edge: WallEdge,
	s_lo: f32,
	s_hi: f32,
) -> Opening {
	let omin = Vec3::from(opening.bounds.min);
	let omax = Vec3::from(opening.bounds.max);
	let tang = edge.tangent();
	let along_x = tang.x.abs() > tang.z.abs();
	let p0 = edge.start + tang * s_lo;
	let p1 = edge.start + tang * s_hi;
	let (a0, a1) =
		if along_x { (p0.x.min(p1.x), p0.x.max(p1.x)) } else { (p0.z.min(p1.z), p0.z.max(p1.z)) };
	opening.bounds = if along_x {
		Aabb3d::from_min_max(Vec3::new(a0, omin.y, omin.z), Vec3::new(a1, omax.y, omax.z))
	} else {
		Aabb3d::from_min_max(Vec3::new(omin.x, omin.y, a0), Vec3::new(omax.x, omax.y, a1))
	};
	opening
}

fn spans_overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> bool {
	a0 < b1 - EPS && b0 < a1 - EPS
}

fn wall_strip_for_edge(
	edge: WallEdge,
	openings: &[EdgeOpening],
	style: building_components::panels::PanelStyle,
	thickness: f32,
) -> ClippedRectangularStrip {
	let len = edge.length();
	let tang = edge.tangent();
	let h = edge.height;
	let t = thickness.max(1e-4);

	if openings.is_empty() {
		return ClippedRectangularStrip::from_nodes(
			style,
			[
				RectangularStripNode::new(edge.start, h, t, 0.0),
				RectangularStripNode::new(edge.end, h, t, 0.0),
			],
			[None],
		);
	}

	let mut nodes = Vec::new();
	let mut insets: Vec<Option<RectInset>> = Vec::new();
	nodes.push(RectangularStripNode::new(edge.start, h, t, 0.0));
	let mut cursor = 0.0_f32;

	for op in openings {
		if op.s_lo > cursor + EPS {
			nodes.push(RectangularStripNode::new(edge.start + tang * op.s_lo, h, t, 0.0));
			insets.push(None);
			cursor = op.s_lo;
		}
		let s_hi = op.s_hi.max(cursor + EPS);
		nodes.push(RectangularStripNode::new(edge.start + tang * s_hi, h, t, 0.0));
		let jamb = 0.02_f32.min((s_hi - cursor) * 0.1);
		insets.push(Some(RectInset::new(op.sill, op.header, jamb, jamb)));
		cursor = s_hi;
	}

	if cursor < len - EPS {
		nodes.push(RectangularStripNode::new(edge.end, h, t, 0.0));
		insets.push(None);
	} else if let Some(last) = nodes.last_mut() {
		last.position = edge.end;
	}

	ClippedRectangularStrip::from_nodes(style, nodes, insets)
}

impl MapsOpenings for IFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&crate::openings::MappedOpening> {
		self.mapped.get(id)
	}
}

#[cfg(test)]
mod multi_opening_tests {
	use bevy_math::{Vec2, Vec3};

	use crate::openings::{MapsOpenings, OpeningId, Openings};
	use crate::paneling::ClippedRectangularStripPiece;
	use crate::shells::i_floor::{IFloor, IFloorParams};

	#[test]
	fn multiple_apertures_on_long_edge_all_map() {
		let base = IFloorParams::new(Vec3::ZERO, Vec2::new(2.0, 10.0), 3.0)
			.top_left_length(None)
			.top_right_length(None)
			.bottom_left_length(None)
			.bottom_right_length(None)
			.build();
		let edge = base
			.edges()
			.iter()
			.copied()
			.max_by(|a, b| a.length().partial_cmp(&b.length()).unwrap_or(std::cmp::Ordering::Equal))
			.expect("edge");
		let mut openings = Openings::new();
		openings.insert(
			"w0",
			IFloor::edge_aperture_opening_at(edge, edge.length() * 0.25, 1.2, 1.2, 0.9),
		);
		openings.insert(
			"w1",
			IFloor::edge_aperture_opening_at(edge, edge.length() * 0.75, 1.2, 1.2, 0.9),
		);
		let shell = IFloorParams::new(Vec3::ZERO, Vec2::new(2.0, 10.0), 3.0)
			.top_left_length(None)
			.top_right_length(None)
			.bottom_left_length(None)
			.bottom_right_length(None)
			.openings(openings)
			.build();
		assert!(shell.mapped_opening(&OpeningId::new("w0")).is_some());
		assert!(shell.mapped_opening(&OpeningId::new("w1")).is_some());
		assert!(shell.walls().iter().any(|w| {
			w.pieces().iter().any(|p| matches!(p, ClippedRectangularStripPiece::Clipped(_)))
				|| w.pieces().len() > 1
		}));
	}
}
