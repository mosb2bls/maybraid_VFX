//! Livable quarters: residential wet rooms + social / work usage areas.
//!
//! Each room follows the parameterized → plan → Fit pattern shared with
//! [`super::common_bedroom`], using the crate-root [`crate::placer`] trier for
//! furniture rooms and empty shells for bathrooms until fixture packing lands.

#[cfg(test)]
mod tests;

pub mod dining_room;
pub mod eating_area;
pub mod kitchen;
pub mod living_room;
pub mod residential_bathroom;
pub mod residential_half_bathroom;
pub mod sitting_room;
pub mod study;

pub use dining_room::{
	DiningRoom, DiningRoomParameterized, DiningRoomPlan, SCOPE as DINING_ROOM_SCOPE,
};
pub use eating_area::{
	EatingArea, EatingAreaParameterized, EatingAreaPlan, SCOPE as EATING_AREA_SCOPE,
};
pub use kitchen::{
	Kitchen, KitchenCounterLayout, KitchenParameterized, KitchenPlan, SCOPE as KITCHEN_SCOPE,
};
pub use living_room::{
	LivingRoom, LivingRoomParameterized, LivingRoomPlan, SCOPE as LIVING_ROOM_SCOPE,
};
pub use residential_bathroom::{
	ResidentialBathroom, ResidentialBathroomParameterized, ResidentialBathroomPlan,
	SCOPE as RESIDENTIAL_BATHROOM_SCOPE,
};
pub use residential_half_bathroom::{
	ResidentialHalfBathroom, ResidentialHalfBathroomParameterized, ResidentialHalfBathroomPlan,
	SCOPE as RESIDENTIAL_HALF_BATHROOM_SCOPE,
};
pub use sitting_room::{
	SittingRoom, SittingRoomParameterized, SittingRoomPlan, SCOPE as SITTING_ROOM_SCOPE,
};
pub use study::{Study, StudyParameterized, StudyPlan, SCOPE as STUDY_SCOPE};
