//! `/show halls-to-shafts` — orthogonal hall carve demo (gizmo boxes only).

use bevy::prelude::*;
use clap::Args;

use super::opening::{parse_opening_arg, OpeningArg, PreviewOpening};
use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct HallsToShafts {
	/// Host rectangle size `x,y,z` (centered on XZ at the origin; Y from 0).
	#[arg(long, default_value = "24,3.5,18", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	/// FastNoise seed lane for hall width / bias sampling.
	#[arg(long, default_value_t = 1337)]
	pub seed: i32,
	/// Fixed hall clear width in meters (omit to sample 2–4 m from noise).
	#[arg(long)]
	pub hall_width: Option<f32>,
	/// Inbound openings (repeatable). Prefer AABB specs:
	/// `id:shaft:minx,miny,minz:maxx,maxy,maxz` or `id:passage:…`.
	///
	/// When omitted, the preview authors two corner shafts and two wall passages.
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl HallsToShafts {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let openings = resolve_openings(&self.openings, self.extent)?;
		Ok((
			PreviewSubject::HallsToShafts {
				extent: self.extent.max(Vec3::splat(1e-4)),
				seed: self.seed,
				hall_width: self.hall_width.filter(|w| *w > 1e-3),
				openings,
			},
			self.transform.transform(),
		))
	}
}

fn resolve_openings(args: &[OpeningArg], extent: Vec3) -> Result<Vec<PreviewOpening>, String> {
	if !args.is_empty() {
		return args.iter().cloned().map(|a| a.resolve_aabb(None)).collect();
	}
	Ok(default_demo_openings(extent))
}

/// Two corner shafts + two wall passages so HallsToShafts always has terminals.
pub(crate) fn default_demo_openings(extent: Vec3) -> Vec<PreviewOpening> {
	let hx = extent.x.max(4.0) * 0.5;
	let hz = extent.z.max(4.0) * 0.5;
	let h = extent.y.max(2.5);
	let shaft = 2.0;
	let half = shaft * 0.5;
	// Inset from walls so shafts sit in the pocket, not on the façade.
	let inset = 3.5_f32.min(hx - half - 0.5).min(hz - half - 0.5).max(half + 0.5);
	vec![
		PreviewOpening {
			id: "shaft_sw".into(),
			label: buildings::OpeningLabel::Shaft,
			min: Vec3::new(-inset - half, 0.0, -inset - half),
			max: Vec3::new(-inset + half, h, -inset + half),
		},
		PreviewOpening {
			id: "shaft_ne".into(),
			label: buildings::OpeningLabel::Shaft,
			min: Vec3::new(inset - half, 0.0, inset - half),
			max: Vec3::new(inset + half, h, inset + half),
		},
		PreviewOpening {
			id: "passage_north".into(),
			label: buildings::OpeningLabel::Passage,
			min: Vec3::new(-0.6, 0.0, hz - 0.25),
			max: Vec3::new(0.6, h.min(2.2), hz + 0.05),
		},
		PreviewOpening {
			id: "passage_west".into(),
			label: buildings::OpeningLabel::Passage,
			min: Vec3::new(-hx - 0.05, 0.0, -0.6),
			max: Vec3::new(-hx + 0.25, h.min(2.2), 0.6),
		},
	]
}
