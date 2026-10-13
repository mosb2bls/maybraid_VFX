//! Roof-half rails, hip/gable corners, and pitch resolution.

use bevy_math::Vec3;
use building_components::panels::PanelStyle;

use crate::paneling::clipped_ruled_strip::ClippedRuledStrip;
use crate::paneling::clipped_tessellated_triangle::ClippedTessellatedTriangle;
use crate::paneling::panel_complex::{PanelComplex, PanelComplexJointPolicy};
use crate::paneling::ruled_strip::RuledStrip;
use crate::paneling::tessellated_triangle_panel::TessellatedTrianglePanel;

/// One pitch of a [`super::PitchedRoof`]: ridge, eave, and wall-plate segment.
#[derive(Debug, Clone, PartialEq)]
pub struct RoofHalf {
	pub ridge_line: (Vec3, Vec3),
	pub eave_line: (Vec3, Vec3),
	pub wall_line: (Vec3, Vec3),
	/// Longitudinal strip between wall plate and eave.
	pub draw_in_wall_line: bool,
	/// End walling at line endpoints `.0` / `.1` (not exclusive with hip).
	pub draw_in_half_gable_end: (bool, bool),
	/// End hip facets at line endpoints `.0` / `.1`.
	pub draw_in_half_hip: (bool, bool),
}

impl RoofHalf {
	pub fn new(ridge_line: (Vec3, Vec3), eave_line: (Vec3, Vec3), wall_line: (Vec3, Vec3)) -> Self {
		Self {
			ridge_line,
			eave_line,
			wall_line,
			draw_in_wall_line: false,
			draw_in_half_gable_end: (false, false),
			draw_in_half_hip: (false, false),
		}
	}

	pub fn draw_in_wall_line(mut self, draw: bool) -> Self {
		self.draw_in_wall_line = draw;
		self
	}

	pub fn draw_in_half_gable_end(mut self, ends: (bool, bool)) -> Self {
		self.draw_in_half_gable_end = ends;
		self
	}

	pub fn draw_in_half_hip(mut self, ends: (bool, bool)) -> Self {
		self.draw_in_half_hip = ends;
		self
	}

	pub(super) fn line_end(line: (Vec3, Vec3), end: usize) -> Vec3 {
		if end == 0 {
			line.0
		} else {
			line.1
		}
	}

	/// Local frame from the eave: **X** along eave, **Y** = world up, **Z** = Y×X.
	pub(super) fn eave_frame(eave_line: (Vec3, Vec3)) -> (Vec3, Vec3) {
		let x = (eave_line.1 - eave_line.0).normalize_or_zero();
		let z = Vec3::Y.cross(x).normalize_or_zero();
		(x, z)
	}

	/// Outward plan orientation for this half (away from the ridge along −Z).
	pub(super) fn outward_orientation(eave_line: (Vec3, Vec3)) -> bevy_math::Vec2 {
		let (_x, z) = Self::eave_frame(eave_line);
		bevy_math::Vec2::new(-z.x, -z.z)
	}

	/// Pitch face centroid (eave/ridge midpoint).
	pub(super) fn pitch_centroid(&self) -> Vec3 {
		let (e0, e1) = self.eave_line;
		let (r0, r1) = self.ridge_line;
		(e0 + e1 + r0 + r1) * 0.25
	}

	/// Sample the pitch face: \(u\) along eave, \(v\) eave→ridge.
	pub(super) fn pitch_point(&self, u: f32, v: f32) -> Vec3 {
		let (e0, e1) = self.eave_line;
		let (r0, r1) = self.ridge_line;
		let eave = e0.lerp(e1, u);
		let ridge = r0.lerp(r1, u);
		eave.lerp(ridge, v)
	}

	/// Third hip corner: from the eave endpoint along **Z** to the ridge plane.
	pub(super) fn hip_drop(ridge_end: Vec3, eave_end: Vec3, eave_z: Vec3) -> Vec3 {
		let along_z = (ridge_end - eave_end).dot(eave_z);
		eave_end + eave_z * along_z
	}

	fn ridge_at_wall_height(ridge_end: Vec3, wall_end: Vec3) -> Vec3 {
		Vec3::new(ridge_end.x, wall_end.y, ridge_end.z)
	}

	/// Drop `p` onto the vertical plane through `wall_end` with normal `wall_long`.
	fn project_onto_wall_end_plane(p: Vec3, wall_end: Vec3, wall_long: Vec3) -> Vec3 {
		p - wall_long * (p - wall_end).dot(wall_long)
	}

	pub(super) fn resolve(
		&self,
		style: PanelStyle,
		joint_policy: PanelComplexJointPolicy,
		pitch_clip: Option<Vec<Vec3>>,
		gable_end_clips: [Option<Vec<Vec3>>; 2],
	) -> ResolvedRoofHalf {
		let (e0, e1) = self.eave_line;
		let (r0, r1) = self.ridge_line;
		let (w0, w1) = self.wall_line;
		let (_eave_x, eave_z) = Self::eave_frame(self.eave_line);

		let pitch = ClippedRuledStrip::from_lines(style, [e0, e1], [r0, r1], [pitch_clip])
			.with_joint_policy(joint_policy);

		let wall = if self.draw_in_wall_line {
			Some(
				RuledStrip::from_lines(style, [w0, w1], [e0, e1])
					.with_joint_policy(joint_policy)
					.into_complex(),
			)
		} else {
			None
		};

		let mut hips = Vec::new();
		for (end, draw) in [(0usize, self.draw_in_half_hip.0), (1, self.draw_in_half_hip.1)] {
			if !draw {
				continue;
			}
			let e = Self::line_end(self.eave_line, end);
			let r = Self::line_end(self.ridge_line, end);
			let p = Self::hip_drop(r, e, eave_z);
			hips.push(TessellatedTrianglePanel::new(style, e, r, p));
		}

		let mut gables = Vec::new();
		// Gable walling stays in the vertical plane of the wall end. Ridge/eave may
		// project past that plane for barge overhang; only the pitch uses those ends.
		let wall_long = (w1 - w0).normalize_or_zero();
		for (end, draw) in
			[(0usize, self.draw_in_half_gable_end.0), (1, self.draw_in_half_gable_end.1)]
		{
			if !draw {
				continue;
			}
			let w = Self::line_end(self.wall_line, end);
			let e = Self::line_end(self.eave_line, end);
			let r = Self::line_end(self.ridge_line, end);
			let e_g = Self::project_onto_wall_end_plane(e, w, wall_long);
			let r_g = Self::project_onto_wall_end_plane(r, w, wall_long);
			let r_wall = Self::ridge_at_wall_height(r_g, w);
			let clip = gable_end_clips[end].clone().unwrap_or_default();
			gables.push(
				ClippedTessellatedTriangle::new(style, w, e_g, r_g, clip.clone())
					.with_joint_policy(joint_policy),
			);
			gables.push(
				ClippedTessellatedTriangle::new(style, w, r_g, r_wall, clip)
					.with_joint_policy(joint_policy),
			);
		}

		ResolvedRoofHalf { pitch, wall, hips, gables }
	}
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ResolvedRoofHalf {
	pub pitch: ClippedRuledStrip,
	pub wall: Option<PanelComplex>,
	pub hips: Vec<TessellatedTrianglePanel>,
	pub gables: Vec<ClippedTessellatedTriangle>,
}
