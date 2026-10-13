//! Kind identifiers as catalog rows: tier, propose knobs, predicates, caps.

use super::tiers::ProgramTier;

/// How a successful place contributes to the enclosure soft-goal gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftGoalRole {
	/// Does not count toward opening fillers.
	None,
	/// Counts as a closet-like enclosure.
	ClosetLike,
	/// Counts as an appointed primary (opens fillers like closet-like).
	Appointed,
	/// Counts as an ensuite / wet-room enclosure.
	Ensuite,
}

/// Geometric propose style (domain-agnostic knobs; rooms interpret extents).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProposeKnobs {
	/// Free-standing AABB sampled in the host.
	FreeExtent { min_x: f32, max_x: f32, min_z: f32, max_z: f32, height: f32, prefer_wall: bool },
	/// Long face against a host wall.
	WallLong { along_min: f32, along_max: f32, depth_min: f32, depth_max: f32, height: f32 },
	/// Free extent as fractions of host long/short plan spans.
	FreeExtentFrac {
		long_frac_min: f32,
		long_frac_max: f32,
		short_frac_min: f32,
		short_frac_max: f32,
		height: f32,
		prefer_wall: bool,
		/// Align long table axis with host's longer plan span.
		align_long_to_host: bool,
	},
	/// Wall-long run as fractions of the wall's along-span / depth in meters.
	WallLongFrac {
		along_frac_min: f32,
		along_frac_max: f32,
		depth_min: f32,
		depth_max: f32,
		height: f32,
	},
	/// Wall-seeded [`crate::usage_areas::enclosed_room::EnclosedRoom`].
	EnclosedRoom,
}

/// Shared geometric predicates applied to a candidate footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Predicate {
	InHost,
	ClearOfKeepOuts,
	AgainstWall,
	LongFaceOnWall,
	ApproachFree,
}

/// What to push into clearances on a successful place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommitEffect {
	/// Solid furniture footprint only.
	SolidFootprint,
	/// Footprint + door keep-out (pad from clearance policy).
	WalledWithDoor { door_approach_pad: f32 },
}

/// One placeable kind in a room catalog.
///
/// `Id` is typically a room-local enum (`BedroomKind`, `KitchenKind`, …).
#[derive(Debug, Clone, PartialEq)]
pub struct KindSpec<Id> {
	pub id: Id,
	pub tier: ProgramTier,
	/// Relative weight among eligible kinds in the same pick.
	pub weight: f32,
	pub max_count: Option<usize>,
	pub soft_goal: SoftGoalRole,
	pub propose: ProposeKnobs,
	pub predicates: &'static [Predicate],
	pub commit: CommitEffect,
	/// When true, area may exceed the furniture occupancy cap (structure).
	pub structure_budget: bool,
}
