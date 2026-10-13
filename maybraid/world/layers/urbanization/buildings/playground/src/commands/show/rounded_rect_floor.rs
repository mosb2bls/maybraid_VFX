//! `/show rounded-rect-floor` — rounded-rectangle storey shell with openings.

use bevy::prelude::*;
use clap::Args;

use super::opening::{ortho_openings, parse_opening_arg, OpeningArg, OrthoOpeningContext};
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RoundedRectFloor {
	#[arg(long, default_value_t = 8.0)]
	pub footprint_x: f32,
	#[arg(long, default_value_t = 6.0)]
	pub footprint_z: f32,
	#[arg(long, default_value_t = 3.0)]
	pub storey_height: f32,
	#[arg(long, default_value_t = 1.0)]
	pub corner_radius: f32,
	#[arg(long, default_value_t = 4)]
	pub corner_segments: u32,
	/// Opening plan entries. Repeatable. When set, overrides `--door-*` flags.
	///
	/// Formats: `id:label:minx,miny,minz:maxx,maxy,maxz` or `id:label:side=south`.
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

impl RoundedRectFloor {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let footprint = Vec2::new(self.footprint_x, self.footprint_z);
		let ctx = OrthoOpeningContext {
			center_xz: Vec3::ZERO,
			footprint,
			storey_height: self.storey_height,
			door_width: self.door_width,
			door_height: self.door_height,
		};
		let openings = ortho_openings(
			&self.openings,
			ctx,
			self.door_north,
			self.door_east,
			self.door_south,
			self.door_west,
		)?;
		Ok((
			PreviewSubject::RoundedRectFloor {
				footprint_x: self.footprint_x,
				footprint_z: self.footprint_z,
				storey_height: self.storey_height,
				corner_radius: self.corner_radius,
				corner_segments: self.corner_segments,
				openings,
				floor: self.floor,
				ceiling: self.ceiling,
			},
			self.transform.transform(),
		))
	}
}
