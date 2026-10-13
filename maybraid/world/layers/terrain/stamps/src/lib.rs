//! Stamp terrain stamps ([RFC-105 §3.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain)).
//!
//! Pure stamp / modulation construction — no LOD `GenerationScheme` wiring.
//! Durham models consume [`StampModulation`] and stamp outputs.
//!
//! Hydrology-shaped families emit height ops and semantic tags only; wet
//! surface rendering is deferred (see Watershed / RFC-127).

pub mod config;
pub mod modulation;
pub mod region;
pub mod stamp;
pub mod stamps;

pub use config::{
	DownhillPair, FractalAnchors, HysteresisSpine, JitteredCenter, MidpointGrading,
	SoftmaskAlongSpine,
};
pub use modulation::{
	soft_voronoi_weights, CellDomainMask, MultiPolylineBandModulation, MultiPolylineBandPart,
	MultiPolylineOffsetModulation, MultiPolylineOffsetPart, PolylineGradeMode,
	RegionAffineModulation, RegionBowlModulation, RegionGradingModulation,
	RegionPolylineGradingModulation, StampModulation, CELL_DOMAIN_EASE_FRAC,
	DEFAULT_CELL_DOMAIN_EASE,
};
pub use region::{
	closest_on_polyline, grade_along_polyline, CircleRegion, ClosestOnPolyline, EllipseRegion,
	PolylineRegion, RectRegion, Region2D, RegionNoise,
};
pub use stamp::{
	scale_additive, scale_near_one, StampSemantics, StampSet, StampStrength,
	SOFTMASK_REFERENCE_SHORT,
};
pub use stamps::{
	BasinWater, BasinWaterParams, Canyon, CanyonParams, CanyonVariant, CaveNetwork,
	CaveNetworkParams, CaveSegment, CaveSegmentKind, HydrologyComplex, HydrologyComplexKind,
	HydrologyComplexParams, KarstNavClass, KarstPocket, KarstPocketParams, MassifStyle, PlateauCap,
	PlateauCapParams, PlateauFootprint, PlateauSurfaceClass, PocketTermination, PocketWater,
	PocketWaterParams, RollingGround, RollingGroundParams, RuggedMassif, RuggedMassifParams,
	ValleyBasin, ValleyBasinParams, ValleyCrossSection, ValleyFloorKind, ValleyTrain,
	ValleyTrainParams, ValleyTrainSegment, ValleyTrainSegmentRole,
};
