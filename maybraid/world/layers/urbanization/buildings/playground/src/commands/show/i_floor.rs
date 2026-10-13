//! `/show i-floor` — I / T / U / L / Z storey shell with openings.

use bevy::prelude::*;
use buildings::{IFloor, IFloorParams, IFloorSlab};
use clap::Args;

use super::opening::{i_floor_openings, parse_opening_arg, OpeningArg, OrthoOpeningContext};
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct IFloorCmd {
	#[arg(long, default_value_t = 2.0)]
	pub central_x: f32,
	#[arg(long, default_value_t = 6.0)]
	pub central_z: f32,
	#[arg(long, default_value_t = 3.0)]
	pub storey_height: f32,
	/// Top-left flange length (meters). Ignored when `--stem-only` / `--plan-l`.
	#[arg(long, default_value_t = 2.0)]
	pub top_left: f32,
	#[arg(long, default_value_t = 2.0)]
	pub top_right: f32,
	#[arg(long, default_value_t = 2.0)]
	pub bottom_left: f32,
	#[arg(long, default_value_t = 2.0)]
	pub bottom_right: f32,
	#[arg(long, default_value_t = false)]
	pub no_top_left: bool,
	#[arg(long, default_value_t = false)]
	pub no_top_right: bool,
	#[arg(long, default_value_t = false)]
	pub no_bottom_left: bool,
	#[arg(long, default_value_t = false)]
	pub no_bottom_right: bool,
	/// Stem-only rectangle (no flanges).
	#[arg(long, default_value_t = false)]
	pub stem_only: bool,
	/// L-plan: bottom-left flange only.
	#[arg(long, default_value_t = false)]
	pub plan_l: bool,
	/// Opening plan entries. Repeatable. When set, overrides `--door-*` flags.
	///
	/// Formats: `id:label:minx,miny,minz:maxx,maxy,maxz` or `id:label:side=south`
	/// (`side=` picks the nearest outer edge on that cardinal).
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[arg(long, default_value_t = false)]
	pub door_north: bool,
	#[arg(long, default_value_t = false)]
	pub door_east: bool,
	#[arg(long, default_value_t = false)]
	pub door_south: bool,
	#[arg(long, default_value_t = false)]
	pub door_west: bool,
	#[arg(long, default_value_t = 1.2)]
	pub door_width: f32,
	#[arg(long, default_value_t = 2.1)]
	pub door_height: f32,
	#[arg(long, default_value_t = false)]
	pub floor: bool,
	#[arg(long, default_value_t = false)]
	pub ceiling: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl IFloorCmd {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let (tl, tr, bl, br) = if self.stem_only {
			(None, None, None, None)
		} else if self.plan_l {
			(None, None, Some(self.bottom_left.max(0.1)), None)
		} else {
			(
				(!self.no_top_left).then_some(self.top_left.max(0.0)),
				(!self.no_top_right).then_some(self.top_right.max(0.0)),
				(!self.no_bottom_left).then_some(self.bottom_left.max(0.0)),
				(!self.no_bottom_right).then_some(self.bottom_right.max(0.0)),
			)
		};

		let probe = IFloor::new(IFloorParams {
			center_xz: Vec3::ZERO,
			top_left_length: tl,
			top_right_length: tr,
			central_rectangle: Vec2::new(self.central_x, self.central_z),
			bottom_left_length: bl,
			bottom_right_length: br,
			storey_height: self.storey_height,
			floor: IFloorSlab::None,
			ceiling: IFloorSlab::None,
			..IFloorParams::default()
		});

		let (min_x, max_x, min_z, max_z) = edge_bounds(&probe);
		let footprint = Vec2::new((max_x - min_x).max(1e-3), (max_z - min_z).max(1e-3));
		let ctx = OrthoOpeningContext {
			center_xz: Vec3::ZERO,
			footprint,
			storey_height: self.storey_height,
			door_width: self.door_width,
			door_height: self.door_height,
		};
		let openings = i_floor_openings(
			&self.openings,
			&probe,
			ctx,
			self.door_north,
			self.door_east,
			self.door_south,
			self.door_west,
		)?;

		Ok((
			PreviewSubject::IFloor {
				central_x: self.central_x,
				central_z: self.central_z,
				storey_height: self.storey_height,
				top_left: tl,
				top_right: tr,
				bottom_left: bl,
				bottom_right: br,
				openings,
				floor: self.floor,
				ceiling: self.ceiling,
			},
			self.transform.transform(),
		))
	}
}

fn edge_bounds(shell: &IFloor) -> (f32, f32, f32, f32) {
	let mut min_x = f32::INFINITY;
	let mut max_x = f32::NEG_INFINITY;
	let mut min_z = f32::INFINITY;
	let mut max_z = f32::NEG_INFINITY;
	for e in shell.edges() {
		for p in [e.start, e.end] {
			min_x = min_x.min(p.x);
			max_x = max_x.max(p.x);
			min_z = min_z.min(p.z);
			max_z = max_z.max(p.z);
		}
	}
	if !min_x.is_finite() {
		return (-1.0, 1.0, -1.0, 1.0);
	}
	(min_x, max_x, min_z, max_z)
}
