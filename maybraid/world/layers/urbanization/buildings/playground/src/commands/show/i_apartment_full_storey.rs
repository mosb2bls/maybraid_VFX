//! `/show i-apartment-full-storey` — I-frame + LivableApartment per primary rect.

use bevy::prelude::*;
use clap::Args;

use super::i_apartment_floor_plan::resolve_i_apartment_openings;
use super::opening::{parse_opening_arg, OpeningArg};
use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct IApartmentFullStorey {
	/// Confines size `x,y,z` (centered on XZ at the origin; Y from 0).
	#[arg(long, default_value = "44,3.5,36", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	/// FastNoise seed lane for spatial sampling.
	#[arg(long, default_value_t = 1337)]
	pub seed: i32,
	/// Solid IFloor ceiling (off by default).
	#[arg(long, default_value_t = false)]
	pub ceiling: bool,
	/// Inbound openings (repeatable). Optional; forwarded onto the IFloor shell.
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl IApartmentFullStorey {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let openings = resolve_i_apartment_openings(&self.openings)?;
		Ok((
			PreviewSubject::IApartmentFullStorey {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
				ceiling: self.ceiling,
				openings,
			},
			self.transform.transform(),
		))
	}
}
