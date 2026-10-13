//! `/show pitched-rectangular-roof` — two-half pitched roof (rectangular hip by default).

use bevy::prelude::*;
use buildings::{OpeningLabel, PitchedRoof, PitchedRoofParams};
use clap::Args;

use super::opening::{parse_opening_arg, OpeningArg, PreviewOpening};
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PitchedRectangularRoof {
	/// Length along the ridge / eaves (X).
	#[arg(long, default_value_t = 10.0)]
	pub footprint_x: f32,
	/// Full span between the two eaves (Z).
	#[arg(long, default_value_t = 6.0)]
	pub footprint_z: f32,
	#[arg(long, default_value_t = 4.0)]
	pub ridge_height: f32,
	#[arg(long, default_value_t = 2.5)]
	pub eave_height: f32,
	/// How far each ridge end sits in from the eave ends.
	#[arg(long, default_value_t = 1.5)]
	pub ridge_inset: f32,
	/// Also draw half-gable end walling under the hips.
	#[arg(long, default_value_t = false)]
	pub gables: bool,
	/// Omit the wall-plate → eave strips.
	#[arg(long, default_value_t = false)]
	pub no_walls: bool,
	/// Omit half-hip facets (open gable-style ends if `--gables`).
	#[arg(long, default_value_t = false)]
	pub no_hips: bool,
	/// Opening plan entries. Repeatable.
	///
	/// Format: `id:label:minx,miny,minz:maxx,maxy,maxz`
	///
	/// Passages / apertures map to the nearest pitch half or drawn gable end.
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	/// Convenience: centered aperture on the +Z pitch half.
	#[arg(long, default_value_t = false)]
	pub skylight: bool,
	/// Convenience: aperture on the +X gable end wall (implies `--gables`).
	#[arg(long, default_value_t = false)]
	pub gable_window: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl PitchedRectangularRoof {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let gables = self.gables || self.gable_window;
		let mut openings = self
			.openings
			.iter()
			.cloned()
			.map(|a| a.resolve_aabb(None))
			.collect::<Result<Vec<_>, _>>()?;

		let mut params = if self.no_hips && gables && self.ridge_inset <= 1e-4 {
			PitchedRoofParams::rectangular_gable(
				Vec2::new(self.footprint_x, self.footprint_z),
				self.ridge_height,
				self.eave_height,
			)
		} else {
			PitchedRoofParams::rectangular_hip(
				Vec2::new(self.footprint_x, self.footprint_z),
				self.ridge_height,
				self.eave_height,
				self.ridge_inset,
			)
		};
		for half in &mut params.halves {
			half.draw_in_wall_line = !self.no_walls;
			half.draw_in_half_hip = if self.no_hips { (false, false) } else { (true, true) };
			half.draw_in_half_gable_end = if gables { (true, true) } else { (false, false) };
		}

		if self.skylight {
			let opening = PitchedRoof::pitch_opening(
				&params.halves[0],
				0.5,
				0.4,
				2.8,
				1.6,
				OpeningLabel::Aperture,
			);
			openings.push(PreviewOpening {
				id: "skylight".into(),
				label: OpeningLabel::Aperture,
				min: Vec3::from(opening.bounds.min),
				max: Vec3::from(opening.bounds.max),
			});
		}
		if self.gable_window {
			let opening =
				PitchedRoof::gable_end_opening(&params.halves, 1, 2.8, 2.2, OpeningLabel::Aperture);
			openings.push(PreviewOpening {
				id: "gable_win".into(),
				label: OpeningLabel::Aperture,
				min: Vec3::from(opening.bounds.min),
				max: Vec3::from(opening.bounds.max),
			});
		}

		Ok((
			PreviewSubject::PitchedRectangularRoof {
				footprint_x: self.footprint_x,
				footprint_z: self.footprint_z,
				ridge_height: self.ridge_height,
				eave_height: self.eave_height,
				ridge_inset: self.ridge_inset,
				gables,
				no_walls: self.no_walls,
				no_hips: self.no_hips,
				openings,
			},
			self.transform.transform(),
		))
	}
}
