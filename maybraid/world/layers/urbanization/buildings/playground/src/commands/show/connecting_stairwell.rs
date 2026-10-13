//! `/show connecting-stairwell` — exclusive AABB well with a circular, rectangular,
//! or run-and-landing flight.

use bevy::prelude::*;
use clap::{Args, ValueEnum};

use buildings::{ConnectingStairwell as Stairwell, StairwellKind, WellAabb, WellSide};

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum StairwellCase {
	/// Same-plan shaft, walk-on and walk-off on −Z.
	#[default]
	Stacked,
	/// Walk-on −Z, walk-off +Z.
	Opposite,
	/// Walk-on −Z, walk-off −X.
	QuarterTurn,
	/// Tight well.
	Tiny,
	/// 6 m rise (extra turns to keep going).
	Tall,
	/// Two exclusive wells stacked; shared face is the join.
	StackedPair,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum StairwellFit {
	#[default]
	Circular,
	Rectangular,
	RunAndLanding,
}

impl From<StairwellFit> for StairwellKind {
	fn from(fit: StairwellFit) -> Self {
		match fit {
			StairwellFit::Circular => StairwellKind::Circular,
			StairwellFit::Rectangular => StairwellKind::Rectangular,
			StairwellFit::RunAndLanding => StairwellKind::RunAndLanding,
		}
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConnectingStairwell {
	#[command(flatten)]
	pub transform: ShowTransform,
	/// Well plan / doors / height.
	#[arg(long, value_enum, default_value_t = StairwellCase::Stacked)]
	pub case: StairwellCase,
	/// Tread span as a fraction of the tighter half-extent.
	#[arg(long, default_value_t = 0.4)]
	pub tread_fill: f32,
	/// Circular helix, wall-hugging rectangular, or half-well I + landing.
	#[arg(long, value_enum, default_value_t = StairwellFit::Circular)]
	pub kind: StairwellFit,
}

impl ConnectingStairwell {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ConnectingStairwell {
				case: self.case,
				tread_fill: self.tread_fill,
				kind: self.kind,
			},
			self.transform.transform(),
		)
	}
}

/// Exclusive wells for a playground case (one, or two for [`StairwellCase::StackedPair`]).
pub fn preview_wells(case: StairwellCase, tread_fill: f32) -> Vec<WellAabb> {
	let well = |min: Vec3, max: Vec3, on: WellSide, off: WellSide| {
		WellAabb::from_plan(min, max, on, off, tread_fill)
	};
	match case {
		StairwellCase::Stacked => vec![well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
		)],
		StairwellCase::Opposite => vec![well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::PosZ,
		)],
		StairwellCase::QuarterTurn => vec![well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegX,
		)],
		StairwellCase::Tiny => vec![well(
			Vec3::new(-0.6, 0.0, -0.6),
			Vec3::new(0.6, 1.5, 0.6),
			WellSide::NegZ,
			WellSide::NegZ,
		)],
		StairwellCase::Tall => vec![well(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 6.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
		)],
		StairwellCase::StackedPair => vec![
			well(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
			),
			well(
				Vec3::new(-1.2, 3.0, -1.2),
				Vec3::new(1.2, 6.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
			),
		],
	}
}

pub fn preview_stairwells(
	case: StairwellCase,
	tread_fill: f32,
	kind: StairwellFit,
) -> Vec<Stairwell> {
	use building_components::panels::PanelStyle;
	let kind = StairwellKind::from(kind);
	preview_wells(case, tread_fill)
		.into_iter()
		.enumerate()
		.map(|(i, w)| {
			let well = Stairwell::from_well_kind(PanelStyle::RoughStonework, w, kind);
			if case == StairwellCase::StackedPair && i == 0 {
				well.with_upper_landing(false)
			} else {
				well
			}
		})
		.collect()
}

/// One cell in [`crate::preview::PreviewSubject::ConnectingStairwellExamples`].
pub struct StairwellGalleryCell {
	pub offset: Vec3,
	pub stairwells: Vec<Stairwell>,
}

const GALLERY_COLS: usize = 4;
const GALLERY_GAP: f32 = 5.0;

/// Door-pair, aspect, fill, and stack cases that stress a fitter.
pub fn pathological_gallery(kind: StairwellFit) -> Vec<StairwellGalleryCell> {
	use building_components::panels::PanelStyle;

	let well = |min: Vec3, max: Vec3, on: WellSide, off: WellSide, fill: f32| {
		WellAabb::from_plan(min, max, on, off, fill)
	};
	let sq = |hy: f32, hz: f32, y1: f32, on: WellSide, off: WellSide, fill: f32| {
		well(Vec3::new(-hy, 0.0, -hz), Vec3::new(hy, y1, hz), on, off, fill)
	};
	const F: f32 = 0.4;
	let specs: [(Vec<WellAabb>, bool); 12] = [
		(vec![sq(1.2, 1.2, 3.0, WellSide::NegZ, WellSide::NegZ, F)], false),
		(vec![sq(1.2, 1.2, 3.0, WellSide::NegZ, WellSide::PosZ, F)], false),
		(vec![sq(1.2, 1.2, 3.0, WellSide::NegZ, WellSide::NegX, F)], false),
		(vec![sq(1.2, 1.2, 3.0, WellSide::NegZ, WellSide::PosX, F)], false),
		(vec![sq(0.6, 0.6, 1.5, WellSide::NegZ, WellSide::NegZ, F)], false),
		(vec![sq(0.35, 1.4, 3.0, WellSide::NegZ, WellSide::NegZ, F)], false),
		(vec![sq(2.0, 2.0, 0.4, WellSide::NegZ, WellSide::NegZ, F)], false),
		(vec![sq(1.2, 1.2, 12.0, WellSide::NegZ, WellSide::NegZ, F)], false),
		(vec![sq(1.2, 1.2, 3.0, WellSide::NegZ, WellSide::NegZ, 0.2)], false),
		(vec![sq(1.2, 1.2, 3.0, WellSide::NegZ, WellSide::NegZ, 0.95)], false),
		(
			vec![
				sq(1.2, 1.2, 3.0, WellSide::NegZ, WellSide::NegZ, F),
				well(
					Vec3::new(-1.2, 3.0, -1.2),
					Vec3::new(1.2, 6.0, 1.2),
					WellSide::NegZ,
					WellSide::NegZ,
					F,
				),
			],
			true,
		),
		(vec![sq(1.2, 1.2, 0.18, WellSide::NegZ, WellSide::NegZ, F)], false),
	];

	let extent = |wells: &[WellAabb]| {
		wells.iter().fold(Vec2::ZERO, |acc, w| {
			let s = w.max() - w.min();
			Vec2::new(acc.x.max(s.x), acc.y.max(s.z))
		})
	};
	(0..specs.len())
		.map(|i| {
			let offset = gallery_offset(|j| extent(&specs[j].0), specs.len(), i);
			let (wells, omit_first) = &specs[i];
			let stairwells = wells
				.iter()
				.copied()
				.enumerate()
				.map(|(k, w)| {
					let s = Stairwell::from_well_kind(
						PanelStyle::RoughStonework,
						w,
						StairwellKind::from(kind),
					);
					if *omit_first && k == 0 {
						s.with_upper_landing(false)
					} else {
						s
					}
				})
				.collect();
			StairwellGalleryCell { offset, stairwells }
		})
		.collect()
}

fn gallery_offset(extent_at: impl Fn(usize) -> Vec2, len: usize, index: usize) -> Vec3 {
	let col = index % GALLERY_COLS;
	let row = index / GALLERY_COLS;
	let mut x = 0.0;
	for c in 0..col {
		x += extent_at(row * GALLERY_COLS + c).x + GALLERY_GAP;
	}
	let mut z = 0.0;
	for r in 0..row {
		let mut row_depth = 0.0_f32;
		for c in 0..GALLERY_COLS {
			let idx = r * GALLERY_COLS + c;
			if idx < len {
				row_depth = row_depth.max(extent_at(idx).y);
			}
		}
		z += row_depth + GALLERY_GAP;
	}
	Vec3::new(x, 0.0, z)
}
