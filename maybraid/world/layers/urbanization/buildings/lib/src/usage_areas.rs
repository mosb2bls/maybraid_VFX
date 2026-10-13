//! Usage areas: fill residual [`crate::fit::Confines`] with program (rooms, shops).
//!
//! Higher-order storeys (e.g. Les Halles) emit shell geometry +
//! [`FillableRegions`]; usage areas consume those confines — packing furniture /
//! subdivisions / commercial interiors — and may emit nested residuals
//! (offices, toilet stalls, …). Shared helpers such as [`clearance`] and
//! [`enclosed_room`] live here for passage keep-outs and wall-seeded private
//! rooms. Rectangular place-and-commit packing uses the crate-root
//! [`crate::placer`] KindSpec trier.

pub mod boundary_openings;
pub mod clearance;
pub mod commercial_stall_strip;
pub mod common_bedroom;
pub mod enclosed_room;
pub mod enclosure_panels;
pub mod furniture_util;
pub mod hall_connected_suites;
pub mod halls_to_shafts;
pub mod janitorial;
pub mod label_util;
pub mod livable_apartment;
pub mod livable_apartments;
pub mod livable_quarters;
pub mod plan_access;
pub mod plan_cells;
pub mod plan_geom;
pub mod rect_passage_cluster;
pub mod rectangular_livable_area;

pub use clearance::{
	abuts_clearance, approach_blocked, approach_zone, commit_door_clear,
	max_empty_abutting_clearance, max_empty_abutting_clearance_sized, pack_abutting_clearance,
	snap_face_to_host_wall, PassageClearance, PlanHost, PASSAGE_APPROACH_PAD, PASSAGE_CLEARANCE,
	PASSAGE_WALL_LIP,
};
pub use commercial_stall_strip::commercial_stall::{
	BitesSitdownStall, BitesStall, CommercialStall, CommercialStallInterior,
	CommercialStallParameterized, CommercialStallPlan, KnickKnackStall,
	KnickKnackStallParameterized, KnickKnackStallPlan, Lounge, MiniMart, PartsStall,
	PublicRestroom, PublicRestroomParameterized, PublicRestroomPlan,
};
pub use commercial_stall_strip::{
	CommercialStallStrip, CommercialStallStripParameterized, CommercialStallStripPlan,
};
pub use common_bedroom::{
	CommonBedroom, CommonBedroomParameterized, CommonBedroomPlan, SCOPE as COMMON_BEDROOM_SCOPE,
};
pub use enclosed_room::{EnclosedRoom, EnclosedRoomMins, EnclosedRoomParams};
pub use hall_connected_suites::{
	HallConnectedGroups, HallEnclosedSuites, HallSuiteEncloseParams, HallSuitePackParams,
};
pub use halls_to_shafts::{HallsToShafts, HallsToShaftsOptions, MAX_HALL_WIDTH, MIN_HALL_WIDTH};
pub use janitorial::Janitorial;
pub use livable_apartment::LivableApartment;
pub use livable_apartments::{
	LivableApartments, LivableApartmentsOptions, LivableApartmentsParameterized,
};
pub use livable_quarters::{
	DiningRoom, DiningRoomParameterized, DiningRoomPlan, EatingArea, EatingAreaParameterized,
	EatingAreaPlan, Kitchen, KitchenCounterLayout, KitchenParameterized, KitchenPlan, LivingRoom,
	LivingRoomParameterized, LivingRoomPlan, ResidentialBathroom, ResidentialBathroomParameterized,
	ResidentialBathroomPlan, ResidentialHalfBathroom, ResidentialHalfBathroomParameterized,
	ResidentialHalfBathroomPlan, SittingRoom, SittingRoomParameterized, SittingRoomPlan, Study,
	StudyParameterized, StudyPlan, DINING_ROOM_SCOPE, EATING_AREA_SCOPE, KITCHEN_SCOPE,
	LIVING_ROOM_SCOPE, RESIDENTIAL_BATHROOM_SCOPE, RESIDENTIAL_HALF_BATHROOM_SCOPE,
	SITTING_ROOM_SCOPE, STUDY_SCOPE,
};
pub use plan_access::{
	door_leaf_height, GroupFootprint, PlanAccessParams, DEFAULT_SOFT_ASPECT, DEFAULT_WALK_CLEAR,
	DOOR_HEIGHT, DOOR_HEIGHT_MIN, DOOR_HEIGHT_SOFT_MAX, DOOR_SIZE_SCALE, DOOR_WIDTH,
	DOOR_WIDTH_MAX, MIN_GROUP_CONNECTIVITY, MIN_ROOM,
};
pub use plan_cells::{
	cell_has_hall_frontage, cells_edge_adjacent, cells_well_connected, decompose_max_rects,
	group_cells_to_apartments, hall_frontage_length, pack_apartments_to_targets, rect_in_union,
	shared_edge_length, shared_edge_span, split_oversized_cells, split_toward_min_room,
	subtract_aabb2, PlanCell,
};
pub use plan_geom::{confines_from_xz, host_xz};
pub use rect_passage_cluster::{RectPassageCluster, RectPassageClusterParams};
pub use rectangular_livable_area::{
	passages_on_faces, CardinalFace, RectAreaRoom, RectLivableStrategy, RectQuarterKind,
	RectangularLivableArea, RectangularLivableAreaParameterized, RectangularLivableAreaPlan,
	DEFAULT_CLOSED_MAX_AREA, DEFAULT_MIN_HALL, SCOPE as RECT_LIVABLE_SCOPE,
};
