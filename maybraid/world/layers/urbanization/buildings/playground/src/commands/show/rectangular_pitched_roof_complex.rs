//! `/show rectangular-pitched-roof-complex` — orthogonal AABB pitched roofs with valleys.

use bevy::prelude::*;
use buildings::{
	EndCap, OpeningLabel, Overhang, RectangularPitchedRoofComplexParams, RidgeJunction,
};
use clap::{Args, ValueEnum};

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum RoofComplexPreset {
	Single,
	#[default]
	L,
	/// L with same eave plate, taller stem ridge.
	LSteppedRidge,
	/// L with raised stem eave plate and ridge.
	LSteppedEave,
	T,
	/// T with taller / higher stem than the cross-bar.
	TStepped,
	/// Large hall gable with three smaller perpendicular bay gables.
	HallAndBays,
	/// Several non-overlapping pitch masses.
	Disjoint,
	/// Intersecting L/T cluster plus disjoint satellites.
	Mixed,
	/// Closed rectangular courtyard ring.
	Ring,
	/// Courtyard ring with per-side ridge / eave heights.
	RingStepped,
	/// Ring with a long southern leg (P footprint).
	PShape,
	/// Parallel same-midline pitches (different height / eave span).
	CoaxialParallel,
	/// Full + cross — no L/T corners under current topology.
	PathologicalCross,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum EndCapKind {
	#[default]
	Hip,
	Gable,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RectangularPitchedRoofComplex {
	#[arg(long, value_enum, default_value_t = RoofComplexPreset::L)]
	pub preset: RoofComplexPreset,
	/// Side eave overhang in meters (fixed). Ignored when `--overhang-ratio` is set.
	#[arg(long, default_value_t = 0.3)]
	pub overhang_fixed: f32,
	/// Side eave overhang as a fraction of eave-to-eave span.
	#[arg(long)]
	pub overhang_ratio: Option<f32>,
	#[arg(long, value_enum, default_value_t = EndCapKind::Hip)]
	pub end_cap: EndCapKind,
	/// Gable ridge projection past the wall plate (meters) when `--end-cap gable`.
	#[arg(long, default_value_t = 0.8)]
	pub gable_ridge: f32,
	/// Gable eave projection past the wall plate (meters) when `--end-cap gable`.
	#[arg(long, default_value_t = 0.7)]
	pub gable_eave: f32,
	/// Ridge-junction blend: `0` = lower ridge, `1` = higher (`RidgeJunction::RunUp`).
	#[arg(long, default_value_t = 0.0)]
	pub run_up: f32,
	/// Place a demo skylight aperture on resolved roof 0 / half 0.
	#[arg(long, default_value_t = false)]
	pub skylight: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl RectangularPitchedRoofComplex {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		let preset = match self.preset {
			RoofComplexPreset::Single => "single",
			RoofComplexPreset::L => "l",
			RoofComplexPreset::LSteppedRidge => "l-stepped-ridge",
			RoofComplexPreset::LSteppedEave => "l-stepped-eave",
			RoofComplexPreset::T => "t",
			RoofComplexPreset::TStepped => "t-stepped",
			RoofComplexPreset::HallAndBays => "hall-and-bays",
			RoofComplexPreset::Disjoint => "disjoint",
			RoofComplexPreset::Mixed => "mixed",
			RoofComplexPreset::Ring => "ring",
			RoofComplexPreset::RingStepped => "ring-stepped",
			RoofComplexPreset::PShape => "p-shape",
			RoofComplexPreset::CoaxialParallel => "coaxial-parallel",
			RoofComplexPreset::PathologicalCross => "pathological-cross",
		};
		// Hall-and-bays defaults to gables even when the global end-cap default is hip.
		let end_cap_gable = matches!(self.end_cap, EndCapKind::Gable)
			|| matches!(self.preset, RoofComplexPreset::HallAndBays);
		(
			PreviewSubject::RectangularPitchedRoofComplex {
				preset: preset.into(),
				overhang_fixed: self.overhang_fixed,
				overhang_ratio: self.overhang_ratio,
				end_cap_gable,
				gable_ridge: self.gable_ridge,
				gable_eave: self.gable_eave,
				run_up: self.run_up,
				skylight: self.skylight,
			},
			self.transform.transform(),
		)
	}
}

pub fn build_params(
	preset: &str,
	overhang_fixed: f32,
	overhang_ratio: Option<f32>,
	end_cap_gable: bool,
	gable_ridge: f32,
	gable_eave: f32,
	run_up: f32,
	skylight: bool,
) -> RectangularPitchedRoofComplexParams {
	let mut params = match preset {
		"single" => RectangularPitchedRoofComplexParams::single(10.0, 6.0, 2.5, 4.5),
		"l-stepped-ridge" => RectangularPitchedRoofComplexParams::l_shape_stepped_ridge(),
		"l-stepped-eave" => RectangularPitchedRoofComplexParams::l_shape_stepped_eave(),
		"t" => RectangularPitchedRoofComplexParams::t_shape(),
		"t-stepped" => RectangularPitchedRoofComplexParams::t_shape_stepped(),
		"hall-and-bays" => RectangularPitchedRoofComplexParams::hall_and_bays(),
		"disjoint" => RectangularPitchedRoofComplexParams::disjoint(),
		"mixed" => RectangularPitchedRoofComplexParams::mixed(),
		"ring" => RectangularPitchedRoofComplexParams::ring(),
		"ring-stepped" => RectangularPitchedRoofComplexParams::ring_stepped(),
		"p-shape" => RectangularPitchedRoofComplexParams::p_shape(),
		"coaxial-parallel" => RectangularPitchedRoofComplexParams::coaxial_parallel(),
		"pathological-cross" => RectangularPitchedRoofComplexParams::pathological_cross(),
		_ => RectangularPitchedRoofComplexParams::l_shape(),
	};
	params.overhang = if let Some(r) = overhang_ratio {
		Overhang::Ratio(r)
	} else {
		Overhang::Fixed(overhang_fixed)
	};
	params.end_cap = if end_cap_gable {
		EndCap::Gable { ridge: Overhang::Fixed(gable_ridge), eave: Overhang::Fixed(gable_eave) }
	} else {
		EndCap::Hip
	};
	params.ridge_junction = RidgeJunction::RunUp(run_up);
	if skylight {
		params = params.with_pitch_opening(
			0,
			0,
			0.55,
			0.45,
			1.4,
			1.0,
			"skylight",
			OpeningLabel::Aperture,
		);
	}
	params
}
