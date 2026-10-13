//! `/show les-halles-floor-plan` — noise-fitted Les Halles ring shell + plan.

use bevy::prelude::*;
use clap::Args;

use super::opening::{parse_opening_arg, OpeningArg, PreviewOpening};
use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LesHallesFloorPlan {
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
	/// When set, only these openings are passed into the fit (so shafts appear
	/// only where inbound `shaft` openings map).
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl LesHallesFloorPlan {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let openings = resolve_les_halles_openings(&self.openings)?;
		Ok((
			PreviewSubject::LesHallesFloorPlan {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
				ceiling: self.ceiling,
				openings,
			},
			self.transform.transform(),
		))
	}
}

pub(crate) fn resolve_les_halles_openings(
	args: &[OpeningArg],
) -> Result<Vec<PreviewOpening>, String> {
	args.iter().cloned().map(|a| a.resolve_aabb(None)).collect()
}
