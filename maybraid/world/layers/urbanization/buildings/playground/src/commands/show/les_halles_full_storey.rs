//! `/show les-halles-full-storey` — Les Halles FullStorey (plan shell + residual fills).

use bevy::prelude::*;
use clap::Args;

use super::les_halles_floor_plan::resolve_les_halles_openings;
use super::opening::{parse_opening_arg, OpeningArg};
use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LesHallesFullStorey {
	/// Confines size `x,y,z` (centered on XZ at the origin; Y from 0).
	#[arg(long, default_value = "48,4,36", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	/// FastNoise seed lane for spatial sampling.
	#[arg(long, default_value_t = 1337)]
	pub seed: i32,
	/// Solid gallery ceiling (off by default).
	#[arg(long, default_value_t = false)]
	pub ceiling: bool,
	/// Inbound openings (repeatable). Prefer AABB specs for shafts:
	/// `id:shaft:minx,miny,minz:maxx,maxy,maxz`.
	///
	/// When omitted, the preview requests all four shaft slots for demos.
	/// When set, only these openings are passed into the fit.
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl LesHallesFullStorey {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let openings = resolve_les_halles_openings(&self.openings)?;
		Ok((
			PreviewSubject::LesHallesFullStorey {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
				ceiling: self.ceiling,
				openings,
			},
			self.transform.transform(),
		))
	}
}
