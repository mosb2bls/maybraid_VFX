//! `/show mixed-use-les-halles-development` — flattened floors, stairs, roof.

use bevy::prelude::*;
use clap::Args;

use super::les_halles_floor_plan::resolve_les_halles_openings;
use super::opening::{parse_opening_arg, OpeningArg};
use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct MixedUseLesHallesDevelopment {
	/// Confines size `x,y,z` (centered on XZ at the origin; Y from 0).
	/// Tall enough for several 3–5 m storeys on a large Les Halles footprint.
	#[arg(long, default_value = "72,16,54", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	/// FastNoise seed lane for spatial sampling.
	#[arg(long, default_value_t = 1337)]
	pub seed: i32,
	/// Inbound openings (repeatable). Prefer AABB specs for shafts:
	/// `id:shaft:minx,miny,minz:maxx,maxy,maxz`.
	///
	/// When omitted, the monotower samples 1–4 shaft slots. When set, inbound
	/// shafts are preserved and complemented up to the sampled count.
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl MixedUseLesHallesDevelopment {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let openings = resolve_les_halles_openings(&self.openings)?;
		Ok((
			PreviewSubject::MixedUseLesHallesDevelopment {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
				openings,
			},
			self.transform.transform(),
		))
	}
}
