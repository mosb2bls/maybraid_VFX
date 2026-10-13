//! Shared plan packing for commercial stall interiors.
//!
//! Passage keep-outs live in [`crate::usage_areas::clearance`] (usage-area-wide).
//! Each submodule owns one interior’s geometry knobs + packer; domain constants
//! stay next to the packer that uses them. This root re-exports types shared
//! across interior modules.
//!
//! | Module | Packs |
//! |--------|--------|
//! | [`bites`] | Passage-face counters, sit-down seating, kitchen remainder |
//! | [`enclosed_room`] | Re-export of [`crate::usage_areas::enclosed_room`] |
//! | [`mini_mart`] | Clearances, office (+door), register, aisles, wall shelves |
//! | [`parts`] | Clearances, office (+door), parts pockets |
//! | [`knick_knack`] | Clearances, wall-aligned display bands |
//! | [`public_restroom`] | Clearances, walled toilet stalls (+door), sinks |

pub mod bites;
pub mod enclosed_room;
pub mod knick_knack;
pub mod mini_mart;
pub mod parts;
pub mod public_restroom;

pub use bites::{
	BitesCounterChoice, BitesKitchen, BitesPassageSpec, BitesSitdownRegions, EligibleBitesPassage,
	PackedBitesCounters,
};
pub use mini_mart::{MiniMartPacked, MiniMartRegions, MiniMartShelfSpec};
