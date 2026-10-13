//! Private roof kit tessellation (not part of the public IR).

use std::f32::consts::PI;

use bevy_math::Vec3;
use scene_ref::MirrorAxis;

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::panels::{fitted_tile_count, PanelKitCaps};
use crate::placed::{Placed, Placement};
use crate::roofs::geometry::{Pitch, RoofGeometry};
use crate::roofs::style::RoofStyle;

/// Atomic roof kit pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RoofKit {
	RightTriangle { mirror: Option<MirrorAxis> },
	Rectangle,
	DomeArc(ArcKit),
}

impl RoofGeometry {
	pub(crate) fn kit_pieces_for_style(&self, style: RoofStyle) -> Vec<Placed<RoofKit>> {
		let panel_caps = PanelKitCaps::from(style);
		match self {
			Self::Pitch(p) => pitch_kits(*p, panel_caps),
			Self::Dome(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(RoofKit::DomeArc(kit), Vec3::ZERO, yaw))
				.collect(),
		}
	}
}

fn pitch_kits(pitch: Pitch, caps: PanelKitCaps) -> Vec<Placed<RoofKit>> {
	let run = pitch.run.max(0.0);
	let tile_width = pitch.tile_width.max(1e-4);
	let mut out = Vec::new();
	let left_w = pitch.left.map(|b| b.abs()).unwrap_or(0.0);
	let rect_x0 = left_w;

	if let Some(base) = pitch.left {
		if base.abs() > 1e-6 && run > 1e-6 {
			out.extend(end_triangles(EndSide::Left, 0.0, base, run));
		}
	}

	if let Some(length) = pitch.length {
		if length > 1e-6 && run > 1e-6 {
			out.extend(body_tiles(rect_x0, length, run, tile_width, caps));
		}
	}

	if let Some(base) = pitch.right {
		if base.abs() > 1e-6 && run > 1e-6 {
			let x_min = rect_x0 + pitch.length.unwrap_or(0.0);
			out.extend(end_triangles(EndSide::Right, x_min, base, run));
		}
	}

	out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndSide {
	Left,
	Right,
}

fn tile_scale(width: f32, run: f32) -> Vec3 {
	Vec3::new(width.max(1e-4), 1.0, run.max(1e-4))
}

fn unit_square_pair(x: f32, width: f32, run: f32) -> [Placed<RoofKit>; 2] {
	let scale = tile_scale(width, run);
	[
		Placed::with_placement(
			RoofKit::RightTriangle { mirror: None },
			Placement::new(Vec3::new(x, 0.0, 0.0), 0.0).with_scale(scale),
		),
		Placed::with_placement(
			RoofKit::RightTriangle { mirror: None },
			Placement::new(Vec3::new(x + width, 0.0, -run), PI).with_scale(scale),
		),
	]
}

fn body_tiles(
	x0: f32,
	length: f32,
	run: f32,
	tile_width: f32,
	caps: PanelKitCaps,
) -> Vec<Placed<RoofKit>> {
	let nx = fitted_tile_count(length, tile_width);
	let width = length / nx as f32;
	let mut out = Vec::with_capacity((nx * 2) as usize);
	for i in 0..nx {
		let x = x0 + i as f32 * width;
		if caps.has_rectangle {
			out.push(Placed::with_placement(
				RoofKit::Rectangle,
				Placement::new(Vec3::new(x, 0.0, 0.0), 0.0).with_scale(tile_scale(width, run)),
			));
		} else {
			out.extend(unit_square_pair(x, width, run));
		}
	}
	out
}

/// One end triangle. Positive base → upright; negative → flipped.
fn end_triangles(side: EndSide, x_min: f32, base: f32, altitude: f32) -> Vec<Placed<RoofKit>> {
	let width = base.abs().max(1e-4);
	let altitude = altitude.max(1e-4);
	let upright = base >= 0.0;
	let scale = tile_scale(width, altitude);
	let kit = match (side, upright) {
		(EndSide::Left, true) => Placed::with_placement(
			RoofKit::RightTriangle { mirror: Some(MirrorAxis::X) },
			Placement::new(Vec3::new(x_min + width, 0.0, 0.0), 0.0).with_scale(scale),
		),
		(EndSide::Left, false) => Placed::with_placement(
			RoofKit::RightTriangle { mirror: None },
			Placement::new(Vec3::new(x_min + width, 0.0, -altitude), PI).with_scale(scale),
		),
		(EndSide::Right, true) => Placed::with_placement(
			RoofKit::RightTriangle { mirror: None },
			Placement::new(Vec3::new(x_min, 0.0, 0.0), 0.0).with_scale(scale),
		),
		(EndSide::Right, false) => Placed::with_placement(
			RoofKit::RightTriangle { mirror: Some(MirrorAxis::X) },
			Placement::new(Vec3::new(x_min, 0.0, -altitude), PI).with_scale(scale),
		),
	};
	vec![kit]
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::arc_kit::ArcKit;
	use crate::roofs::geometry::Pitch;
	use crate::roofs::style::RoofStyle;
	use std::f32::consts::PI;

	fn kit_pieces(g: &RoofGeometry) -> Vec<Placed<RoofKit>> {
		g.kit_pieces_for_style(RoofStyle::ShepherdsThatch)
	}

	#[test]
	fn rectangle_only_fits_tiles_to_length() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 2.0, 1.0).with_length(3.0);
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		assert_eq!(pieces.len(), 3);
		assert_eq!(pieces[0].geom, RoofKit::Rectangle);
		assert_eq!(pieces[0].translation().x, 0.0);
		assert_eq!(pieces[0].scale(), Vec3::new(1.0, 1.0, 2.0));
		assert_eq!(pieces[2].translation().x, 2.0);
		Ok(())
	}

	#[test]
	fn tile_width_suggestion_rounds_and_stretches() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 1.0, 1.0).with_length(2.4);
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		assert_eq!(pieces.len(), 2);
		assert_eq!(pieces[0].geom, RoofKit::Rectangle);
		assert!((pieces[0].scale().x - 1.2).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn left_end_shifts_rectangle_and_anchors_at_zero() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 1.0, 1.0).with_length(2.0).with_left(0.5);
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		assert_eq!(pieces[0].translation().x, 0.5);
		assert_eq!(pieces[0].scale().x, 0.5);
		assert_eq!(pieces[0].geom, RoofKit::RightTriangle { mirror: Some(MirrorAxis::X) });
		assert_eq!(pieces[1].translation().x, 0.5);
		Ok(())
	}

	#[test]
	fn negative_right_uses_flipped_complement() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 2.0, 1.0).with_length(1.0).with_right(-0.75);
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		let end = pieces.last().expect("right end");
		assert_eq!(end.yaw(), PI);
		assert!((end.translation().x - 1.0).abs() < 1e-4);
		assert!((end.translation().z - (-2.0)).abs() < 1e-4);
		assert!((end.scale().x - 0.75).abs() < 1e-4);
		assert_eq!(end.geom, RoofKit::RightTriangle { mirror: Some(MirrorAxis::X) });
		Ok(())
	}

	#[test]
	fn from_eave_ridge_ridge_longer_flips_both_ends() -> anyhow::Result<()> {
		let pitch = Pitch::from_eave_ridge(1.0, 2.0, 4.0, 6.0, 1.0);
		assert_eq!(pitch.length, Some(4.0));
		assert_eq!(pitch.left, Some(-1.0));
		assert_eq!(pitch.right, Some(-1.0));
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		assert_eq!(pieces.len(), 1 + 4 + 1);
		assert_eq!(pieces[0].yaw(), PI);
		assert_eq!(pieces.last().expect("right").yaw(), PI);
		Ok(())
	}

	#[test]
	fn from_eave_ridge_eave_longer_upright_ends() -> anyhow::Result<()> {
		let pitch = Pitch::from_eave_ridge(1.0, 2.0, 6.0, 4.0, 1.0);
		assert_eq!(pitch.length, Some(4.0));
		assert_eq!(pitch.left, Some(1.0));
		assert_eq!(pitch.right, Some(1.0));
		Ok(())
	}

	#[test]
	fn length_none_omits_rectangle() -> anyhow::Result<()> {
		let pitch = Pitch::new(1.0, 1.0, 1.0).with_left(1.0).with_right(1.0);
		let pieces = kit_pieces(&RoofGeometry::pitch(pitch));
		assert_eq!(pieces.len(), 2);
		Ok(())
	}

	#[test]
	fn dome_forty_five_is_three_fifteens() -> anyhow::Result<()> {
		let pieces = kit_pieces(&RoofGeometry::dome(45.0));
		assert_eq!(pieces.len(), 3);
		assert!(pieces.iter().all(|p| p.geom == RoofKit::DomeArc(ArcKit::D15)));
		Ok(())
	}

	#[test]
	fn dome_pitch_is_zero() -> anyhow::Result<()> {
		assert_eq!(RoofGeometry::dome(90.0).pitch_degrees(), 0.0);
		Ok(())
	}

	#[test]
	fn pitch_radians_from_rise_run() -> anyhow::Result<()> {
		let p = Pitch::new(1.0, 1.0, 1.0).with_length(1.0);
		assert!((p.pitch_radians() - std::f32::consts::FRAC_PI_4).abs() < 1e-4);
		Ok(())
	}
}
