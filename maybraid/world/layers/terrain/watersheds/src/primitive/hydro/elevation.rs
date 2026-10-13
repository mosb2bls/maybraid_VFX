//! Depth / surface fields over a hydraulic footprint.

pub mod radial_bowl;
pub mod reach_profile;

pub use radial_bowl::RadialBowl;
pub use reach_profile::ReachProfile;

/// Depth / surface field over the footprint (local coordinates).
#[derive(Debug, Clone)]
pub enum HydroElevation {
	/// Graded reach with transverse bowl.
	Reach(ReachProfile),
	/// Flat lake / pool bowl.
	Radial(RadialBowl),
}
