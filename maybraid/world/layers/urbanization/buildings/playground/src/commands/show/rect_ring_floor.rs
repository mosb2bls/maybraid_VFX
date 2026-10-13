//! `/show rect-ring-floor` — rectangular ring storey shell.
//!
//! Broad wall omissions are authored with openings (wide `Passage` / `Aperture`
//! AABBs or `side=`), not omit intervals. Each opening maps to one outer/inner
//! side only — a half-ring AABB will not clear every wall on that half; use one
//! `--opening` per side for a U / half-ring.

use bevy::prelude::*;
use clap::Args;

use super::opening::{ortho_openings, parse_opening_arg, OpeningArg, OrthoOpeningContext};
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RectRingFloor {
	#[arg(long, default_value_t = 8.0)]
	pub outer_x: f32,
	#[arg(long, default_value_t = 6.0)]
	pub outer_z: f32,
	#[arg(long, default_value_t = 4.0)]
	pub inner_x: f32,
	#[arg(long, default_value_t = 3.0)]
	pub inner_z: f32,
	#[arg(long, default_value_t = 3.0)]
	pub storey_height: f32,
	/// Opening plan entries. Repeatable. When set, overrides `--door-*` flags.
	///
	/// Formats: `id:label:minx,miny,minz:maxx,maxy,maxz` or `id:label:side=south`.
	/// One opening → one ring side (nearest). Wide passage/aperture opens that
	/// side; clear several sides with several openings.
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

impl RectRingFloor {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let outer = Vec2::new(self.outer_x, self.outer_z);
		let ctx = OrthoOpeningContext {
			center_xz: Vec3::ZERO,
			footprint: outer,
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
			PreviewSubject::RectRingFloor {
				outer_x: self.outer_x,
				outer_z: self.outer_z,
				inner_x: self.inner_x,
				inner_z: self.inner_z,
				storey_height: self.storey_height,
				openings,
				floor: self.floor,
				ceiling: self.ceiling,
			},
			self.transform.transform(),
		))
	}
}
