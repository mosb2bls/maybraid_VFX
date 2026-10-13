//! Les Halles storey.
//!
//! The basic Les Halles layout is an outer ring gallery with an inner ring balcony.
//! The outer ring gallery is commercial space: a rectangular ring with outer facade
//! walls, inner walls facing the balcony, and a floor (ceiling optional / off by default).
//! The inner balcony ring is walking access: floor only, no walls, open to the courtyard.
//!
//! The Les Halles floor plan...
//! - Uses the full rectangular width and height of the confines.
//! - Decides reasonable widths for the gallery and balcony.
//! - Adds apertures on the outer facade and stall doors on the gallery’s inner wall.
//! - Chooses whether to allocate shafts (for stairs) in the four corners or in the middle of each side.
//! - Remaps inbound shaft openings onto fitted slots (quadrants for corners;
//!   N/S end bands + E/W middle bands for mid-sides). A shaft is only authored
//!   when at least one inbound opening maps to that slot.
//!
//! Full\* variants share the same floor plan: commercial
//! ([`LesHallesFullStorey`]) fills gallery strips with stalls; livable
//! ([`LesHallesLivableFullStorey`]) fills them with lengthwise
//! [`crate::RectangularLivableArea`] bays; the ground arcade
//! ([`LesHallesArcadeStorey`]) leaves the gallery open with midspan breezeways
//! and no inner courtyard walls.

pub mod arcade_storey;
pub mod floor_plan;
pub mod full_storey;
pub mod livable_full_storey;
pub mod parameterized;
pub mod usage_plan;

pub use arcade_storey::LesHallesArcadeStorey;
pub use floor_plan::{LesHallesFloorPlan, LesHallesOpeningProgram};
pub use full_storey::LesHallesFullStorey;
pub use livable_full_storey::LesHallesLivableFullStorey;
pub use parameterized::{
	LesHallesParameterized, LesHallesPlacedDoor, LesHallesShaftPlacement, LesHallesStallDoor,
	MAX_MONOTOWER_STOREYS, MIN_MONOTOWER_STOREYS,
};
pub use usage_plan::{
	LesHallesArcadeUsage, LesHallesCommercialUsage, LesHallesLivableUsage, LesHallesUsagePlan,
};

/// Scope prefix for [`crate::OpeningId::scoped`] openings authored by this typology.
pub const SCOPE: &str = "les_halles";
