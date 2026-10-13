//! `/show circ-ring-floor` — circular ring storey shell with annulus floor.

use bevy::prelude::*;
use clap::Args;

use super::opening::{parse_opening_arg, ArcOpeningContext, OpeningArg};
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct CircRingFloor {
	#[arg(long, default_value_t = 5.0)]
	pub outer_radius: f32,
	#[arg(long, default_value_t = 2.5)]
	pub inner_radius: f32,
	#[arg(long, default_value_t = 3.0)]
	pub storey_height: f32,
	/// Opening plan entries. Repeatable.
	///
	/// Formats: `id:label:minx,miny,minz:maxx,maxy,maxz` or `id:label:t=0.25`
	/// (optional `,ring=inner|outer`; default outer).
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[arg(long, default_value_t = false)]
	pub floor: bool,
	#[arg(long, default_value_t = false)]
	pub ceiling: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl CircRingFloor {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let openings = self
			.openings
			.into_iter()
			.map(|a| {
				let radius = match a.arc_ring_preference() {
					Some(crate::commands::show::opening::CircRingPreference::Inner) => {
						self.inner_radius
					}
					_ => self.outer_radius,
				};
				let ctx = ArcOpeningContext {
					center_xz: Vec3::ZERO,
					radius,
					storey_height: self.storey_height,
				};
				a.resolve_aabb(Some(ctx))
			})
			.collect::<Result<_, _>>()?;
		Ok((
			PreviewSubject::CircRingFloor {
				outer_radius: self.outer_radius,
				inner_radius: self.inner_radius,
				storey_height: self.storey_height,
				openings,
				floor: self.floor,
				ceiling: self.ceiling,
			},
			self.transform.transform(),
		))
	}
}
