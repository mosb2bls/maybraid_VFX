//! `/show trazaloid` — two-band trapezoidal-pyramid shell.
//!
//! With no `--opening` / `--door-*` flags the shell is solid (waist gap still present).

use bevy::prelude::*;
use clap::Args;

use super::opening::{parse_opening_arg, trazaloid_openings, OpeningArg};
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Trazaloid {
	#[arg(long, default_value_t = 8.0)]
	pub footprint_x: f32,
	#[arg(long, default_value_t = 6.0)]
	pub footprint_z: f32,
	#[arg(long, default_value_t = 4.0)]
	pub ridge_x: f32,
	#[arg(long, default_value_t = 3.0)]
	pub ridge_z: f32,
	#[arg(long, default_value_t = 3.0)]
	pub lower_height: f32,
	#[arg(long, default_value_t = 2.5)]
	pub upper_height: f32,
	#[arg(long, default_value_t = 0.35)]
	pub band_vertical_offset: f32,
	#[arg(long, default_value_t = 0.25)]
	pub waist_horizontal_offset: f32,
	/// Opening plan entries. Repeatable. When set, overrides `--door-*` flags.
	///
	/// Format: `id:label:minx,miny,minz:maxx,maxy,maxz`
	///
	/// Passages map to centered lower-band doors (largest per side wins).
	/// Apertures are ignored for wall mapping (the waist gap is the window).
	/// Shaft / boundary / exclusion / custom can cut Solid floor / ceiling.
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
	/// Convenience door width in meters when using `--door-*` flags.
	#[arg(long, default_value_t = 1.2)]
	pub door_width: f32,
	/// Convenience door height in meters when using `--door-*` flags.
	#[arg(long, default_value_t = 2.1)]
	pub door_height: f32,
	/// Emit a solid footprint floor (openings may still cut / remove it).
	#[arg(long, default_value_t = false)]
	pub floor: bool,
	/// Omit the ridge ceiling.
	#[arg(long, default_value_t = false)]
	pub no_ceiling: bool,
	#[arg(long, default_value_t = 2)]
	pub face_post_count: u32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Trazaloid {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let footprint = Vec2::new(self.footprint_x, self.footprint_z);
		let openings = trazaloid_openings(
			&self.openings,
			footprint,
			self.door_width,
			self.door_height,
			self.door_north,
			self.door_east,
			self.door_south,
			self.door_west,
		)?;
		Ok((
			PreviewSubject::Trazaloid {
				footprint_x: self.footprint_x,
				footprint_z: self.footprint_z,
				ridge_x: self.ridge_x,
				ridge_z: self.ridge_z,
				lower_height: self.lower_height,
				upper_height: self.upper_height,
				band_vertical_offset: self.band_vertical_offset,
				waist_horizontal_offset: self.waist_horizontal_offset,
				openings,
				floor: self.floor,
				no_ceiling: self.no_ceiling,
				face_post_count: self.face_post_count,
			},
			self.transform.transform(),
		))
	}
}
