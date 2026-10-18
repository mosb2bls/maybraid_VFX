//! Authoring knobs for Watershed pocket-water lakes, streams, and bogs (dual-band).
//!
//! Defaults for likelihood / cell size / correlation come from each band's
//! layout consts (`define_marazion_band!` in [`super::low_pass`] /
//! [`super::high_pass`]) — same pattern as Stamp `define_stamp_family!`.

use crate::terrain::watersheds::high_pass::PrePocketHighPassLayout;
use crate::terrain::watersheds::low_pass::PrePocketLowPassLayout;
use bevy::math::Vec2;
use bevy::prelude::*;
use terrain_watersheds::{
	BogParams, LakeParams, PocketGuillotineParams, PrePocketParams, StreamParams,
	StreamsGraphParams,
};

/// One occupancy / scale band (low-pass = small, high-pass = large).
#[derive(Debug, Clone)]
pub struct WatershedBandConfig {
	pub pre_pocket: PrePocketParams,
	pub guillotine: PocketGuillotineParams,
	pub lake: LakeParams,
	pub stream: StreamParams,
	pub bog: BogParams,
	pub streams_graph: StreamsGraphParams,
	/// Fraction of occupied leaves typed as stream (`0.0..=1.0`).
	pub stream_frac: f32,
	/// Fraction of occupied leaves typed as streams-graph (`0.0..=1.0`); applied
	/// after [`Self::stream_frac`].
	pub streams_graph_frac: f32,
	/// Fraction of occupied leaves typed as bog (`0.0..=1.0`); applied after
	/// [`Self::streams_graph_frac`]. Remainder are lakes.
	pub bog_frac: f32,
	/// Approximate leaf acceptance rate (`0.0..=1.0`). Prefer setting defaults
	/// in `define_marazion_band!` (`likelihood:`); this field is the runtime override.
	pub likelihood: f32,
	pub spatial_correlation: f32,
	pub family_salt: u32,
}

impl WatershedBandConfig {
	/// Build runtime knobs from a band layout's authored consts.
	pub fn from_layout<L>(seed: u32, lake: LakeParams, stream: StreamParams, bog: BogParams) -> Self
	where
		L: WatershedBandLayoutConsts,
	{
		let mut streams_graph = StreamsGraphParams::default();
		streams_graph.stream = stream;
		streams_graph.stream.rim.height_amp_min = 0.0;
		streams_graph.stream.rim.height_amp_max = streams_graph.stream.rim.height_amp_max.min(1.25);
		streams_graph.rim_uplift_cap = streams_graph.rim_uplift_cap.min(1.5);
		Self {
			pre_pocket: PrePocketParams {
				pitch: L::PRE_POCKET_PITCH,
				origin: Vec2::new(L::ORIGIN_OFFSET.0, L::ORIGIN_OFFSET.1),
				pocket_pitches: L::POCKET_PITCHES,
				seed,
			},
			guillotine: PocketGuillotineParams {
				max_depth: 3,
				min_span: L::CELL_SIZE_MIN,
				seed,
				..Default::default()
			},
			lake,
			stream,
			bog,
			streams_graph,
			stream_frac: 0.22,
			streams_graph_frac: 0.12,
			bog_frac: 0.20,
			likelihood: L::LIKELIHOOD.clamp(0.0, 1.0),
			spatial_correlation: L::SPATIAL_CORRELATION,
			family_salt: L::FAMILY_SALT,
		}
	}
}

/// Layout consts authored by `define_marazion_band!` in the band modules.
pub trait WatershedBandLayoutConsts {
	const CELL_SIZE_MIN: f32;
	const CELL_SIZE_MAX: f32;
	const PRE_POCKET_PITCH: f32;
	const POCKET_PITCHES: [f32; 4];
	const ORIGIN_OFFSET: (f32, f32);
	const LIKELIHOOD: f32;
	const SPATIAL_CORRELATION: f32;
	const FAMILY_SALT: u32;
}

macro_rules! impl_band_layout_consts {
	($Layout:ty) => {
		impl WatershedBandLayoutConsts for $Layout {
			const CELL_SIZE_MIN: f32 = <$Layout>::CELL_SIZE_MIN;
			const CELL_SIZE_MAX: f32 = <$Layout>::CELL_SIZE_MAX;
			const PRE_POCKET_PITCH: f32 = <$Layout>::PRE_POCKET_PITCH;
			const POCKET_PITCHES: [f32; 4] = <$Layout>::POCKET_PITCHES;
			const ORIGIN_OFFSET: (f32, f32) = <$Layout>::ORIGIN_OFFSET;
			const LIKELIHOOD: f32 = <$Layout>::LIKELIHOOD;
			const SPATIAL_CORRELATION: f32 = <$Layout>::SPATIAL_CORRELATION;
			const FAMILY_SALT: u32 = <$Layout>::FAMILY_SALT;
		}
	};
}

impl_band_layout_consts!(PrePocketLowPassLayout);
impl_band_layout_consts!(PrePocketHighPassLayout);

/// Universal Watershed configs: parallel low-pass + high-pass stacks.
#[derive(Resource, Debug, Clone)]
pub struct WatershedConfigs {
	pub seed: u32,
	pub low_pass: WatershedBandConfig,
	pub high_pass: WatershedBandConfig,
}

impl Default for WatershedConfigs {
	fn default() -> Self {
		let seed = 127;
		let mut low_lake = LakeParams::default();
		low_lake.depth = 11.0;
		low_lake.water_scale_min = 0.45;
		let mut high_lake = LakeParams::default();
		high_lake.depth = 18.0;
		high_lake.water_scale_min = 0.40;
		let mut low_stream = StreamParams::default();
		low_stream.depth = 6.5;
		let mut high_stream = StreamParams::default();
		high_stream.depth = 10.0;
		high_stream.half_width_frac = 0.06;
		let mut low_bog = BogParams::default();
		low_bog.lake.depth = 1.9;
		low_bog.fill.peak_above_w = 0.75;
		let mut high_bog = BogParams::default();
		high_bog.lake.depth = 2.65;
		high_bog.fill.peak_above_w = 1.0;
		Self {
			seed,
			low_pass: WatershedBandConfig::from_layout::<PrePocketLowPassLayout>(
				seed, low_lake, low_stream, low_bog,
			),
			high_pass: WatershedBandConfig::from_layout::<PrePocketHighPassLayout>(
				seed,
				high_lake,
				high_stream,
				high_bog,
			),
		}
	}
}

impl WatershedConfigs {
	/// Retarget occupancy / cut seeds without changing stamp knobs.
	pub fn with_seed(mut self, seed: u32) -> Self {
		self.seed = seed;
		self.low_pass.pre_pocket.seed = seed;
		self.low_pass.guillotine.seed = seed;
		self.high_pass.pre_pocket.seed = seed;
		self.high_pass.guillotine.seed = seed;
		self
	}
}

lod::seeded_root!(WatershedConfigs);
