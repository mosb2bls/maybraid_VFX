//! Procedural primitives shared across Maybraid (noise, fields, …).

pub mod args;
pub mod bounds;
pub mod distributions;
pub mod easing;
pub mod hysteresis;
pub mod mu;
pub mod noise;
pub mod packing;
pub mod path;

pub use args::{
	parse_count_pair, parse_u32_range, parse_unit_range, parse_usize_range, CountPair, U32Range,
	UnitRange, UsizeRange,
};
pub use bounds::{
	aabb2_area, aabb3_to_plan, clamp_min_size2, grow_aabb2, grow_aabb2_pair, inflate_aabb2,
	inset_aabb2, intersects_aabb2, max_empty_aabb3_plan, max_empty_rect2, max_empty_rect2_by,
	max_empty_rect2_with_clearance, plan_to_aabb3, touches_aabb2, union_aabb3, union_aabb3_iter,
	PlanAxes,
};
pub use distributions::bucket_throw::{Bucket, BucketThrow, TypedBucketThrow};
pub use distributions::{perturb_weights, FirstFitIndices, MIN_BUCKET_WEIGHT};
pub use easing::Smoothstep01;
pub use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
pub use hysteresis::{Bounds2, HysteresisConfig, HysteresisGraph, SeededHash};
pub use mu::{sdf_band_margin, NUMERIC_SURFACE_EPSILON};
pub use noise::{
	noise_params_from_scalar_str, BuildWithNoise, FromScalarNoise, NoiseConfig, NoiseParams,
};
pub use packing::{Aabb2dPack, OptionalFaceBand, PlanOpeningFace};
pub use path::{noisy_path, AllowedAngles, NoisyPathParams, StepLenRange};
