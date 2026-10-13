//! `/show arc-floor` — one circular storey shell with optional openings.
//!
//! With no `--opening` flags the shell is solid (full wall sweeps + optional slabs).

use bevy::prelude::*;
use clap::Args;

use super::opening::{parse_opening_arg, ArcOpeningContext, OpeningArg};
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ArcFloor {
	#[arg(long, default_value_t = 4.0)]
	pub radius: f32,
	#[arg(long, default_value_t = 3.0)]
	pub storey_height: f32,
	/// Emit a solid floor slab (openings may still cut / remove it).
	#[arg(long, default_value_t = true)]
	pub floor: bool,
	/// Emit a solid ceiling slab (openings may still cut / remove it).
	#[arg(long, default_value_t = true)]
	pub ceiling: bool,
	/// Opening plan entries. Repeatable. When omitted, the shell stays solid.
	///
	/// Formats: `id:label:minx,miny,minz:maxx,maxy,maxz` or `id:label:t=0.5`.
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ArcFloor {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let ctx = ArcOpeningContext {
			center_xz: Vec3::ZERO,
			radius: self.radius,
			storey_height: self.storey_height,
		};
		let openings = self
			.openings
			.into_iter()
			.map(|a| a.resolve_aabb(Some(ctx)))
			.collect::<Result<_, _>>()?;
		Ok((
			PreviewSubject::ArcFloor {
				radius: self.radius,
				storey_height: self.storey_height,
				floor: self.floor,
				ceiling: self.ceiling,
				openings,
			},
			self.transform.transform(),
		))
	}
}
