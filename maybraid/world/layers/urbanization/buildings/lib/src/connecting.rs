//! Connectors that are not envelope shells: exclusive stairwells, later halls.

pub mod stairwell;

pub use stairwell::{
	ConnectingStairwell, StairwellKind, StairwellOpening, TreadEnd, WellAabb, WellSide, RUN_IN_M,
	SLAB_THICKNESS_M, TREAD_FILL_DEFAULT,
};
