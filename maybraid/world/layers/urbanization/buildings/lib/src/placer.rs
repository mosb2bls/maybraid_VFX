//! Predicate-based rectangular layout trier.
//!
//! Callers own a catalog of [`kind::KindSpec`] rows (tier, propose knobs,
//! predicates, caps). The geometric loop is domain-agnostic: pick → propose →
//! try predicates → commit. Priority lives in kind selection
//! ([`tiers`]), not in separate packer passes. Suitable for usage areas,
//! stall interiors, and other AABB-in-host packing.

pub mod budget;
pub mod composition;
pub mod furniture;
pub mod kind;
pub mod pack;
pub mod predicates;
pub mod tiers;
pub mod walled_room;

pub use budget::OccupiedBudget;
pub use composition::{corner_l_runs, plans_touch, try_corner_l, try_peninsula_from_run, wall_of};
pub use furniture::{try_free_extent, try_wall_long, FreeExtentKnobs, WallLongKnobs};
pub use kind::{CommitEffect, KindSpec, Predicate, ProposeKnobs, SoftGoalRole};
pub use pack::{
	init_host, init_host_with, pack_kinds, propose_from_spec, soft_goal_from_placed, xz_area,
	InitHostOpts, PackHost, PackKnobs, WALL_EPS,
};
pub use predicates::{
	against_wall, approach_free, clear_of_keep_outs, in_host, long_face_on_wall, PredicateCtx,
};
pub use tiers::{enclosure_soft_goal_met, pick_kind, soft_goal_credit, ProgramTier};
pub use walled_room::WalledRoomFill;
