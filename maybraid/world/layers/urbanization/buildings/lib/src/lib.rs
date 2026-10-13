//! A common constraint representation for building generation.
//!
//! Often, for complicated patterns, authors will use a combination of
//! this representation and direct access to parent types to build
//! the authored type.
//!
//! At the same time, common constructions built only requiring this representation
//! can be reused within authored types.
//!
//! The cells are rectangular prisms, describing authoring bounds.
//! The authored types do not need, however, to author strictly rectangular geometry.

pub mod arc_spire;
pub mod arcs;
pub mod connecting;
pub mod constraints;
pub mod demos;
pub mod fit;
pub mod monotower;
pub mod openings;
pub mod paneling;
pub mod placer;
pub mod portals;
pub mod shells;
pub mod stacked_rings;
pub mod storeys;
pub mod usage_areas;
pub mod wall_demo;
pub mod wizards_tower;

// Compatibility module paths (pre-paneling layout).
pub use paneling::clipped_quad_panel;
pub use paneling::clipped_ruled_strip;
pub use paneling::clipped_tessellated_triangle;
pub use paneling::panel_complex;
pub use paneling::panel_plane;
pub use paneling::quad_panel;
pub use paneling::quad_panel_complex;
pub use paneling::ruled_pitch;
pub use paneling::ruled_strip;
pub use paneling::tessellated_triangle_panel;
pub use paneling::tube;

pub use arc_spire::{
	best_fit_y_bindings, uniform_storey_bindings, ArcSpire, ArcSpireParams, FitTolerance,
};
pub use arcs::{portal_ring_wall, ArcSweep, ClippedArcSweep, PortalRingParams, PortalRingWall};
pub use connecting::{
	ConnectingStairwell, StairwellKind, StairwellOpening, TreadEnd, WellAabb, WellSide, RUN_IN_M,
	SLAB_THICKNESS_M, TREAD_FILL_DEFAULT,
};
pub use constraints::{
	BoundaryOwnershipEntry, BoundaryOwnershipStatus, BoundaryRegionList, BoundaryThicknessEntry,
	CellBoundaryTable, CellConstraints, CirculationEntry, CirculationRequestStatus, FaceKind,
	JointCoordinate, JointEntry, PreJointSweep, SubsetError,
};
pub use demos::ConnectingShells;
pub use fit::{
	aabb_near_plane, aabb_xz_area, aabb_xz_center, aabb_xz_extent, aabb_xz_near_eq,
	aabb_xz_overlap_area, Confines, FillRegion, FillableRegions, Fit, FitError, FitTarget,
	MultiConfines, MultiFit, SpaceKind, StackRegion,
};
pub use monotower::{
	ApartmentMonotower, MixedUseLesHallesMonotower, MixedUseLesHallesStorey, SingleHighrise,
	SingleHighriseFloorPlan, SingleHighrisePlan, SingleHighriseShaftSlot, SingleHighriseStorey,
};
pub use openings::{
	fit_bays_on_run, fit_windows_on_run, generate_stall_doors, generate_windows,
	sync_connectable_openings_from_mapped, BaySpec, MappedOpening, MappedOpeningQuad,
	MappedOpenings, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings, PlacedBay,
};
pub use paneling::{
	fallback_oriented, fit_rectangle, fit_rectangle_corners, orient_rectangle,
	roll_to_align_height, shared_edges, zero_roll_height_axis, ApproximatedCircle,
	ClippedFittedRectangle, ClippedFittedRectangularStrip, ClippedFittedRectangularStripPiece,
	ClippedQuadPanel, ClippedRectangle, ClippedRectangularStrip, ClippedRectangularStripPiece,
	ClippedRuledStrip, ClippedStripPiece, ClippedTessellatedTriangle, FittedRect, FittedRectangle,
	FittedRectangularStrip, OrientedRect, PanelComplex, PanelComplexJointPolicy,
	PanelComplexValidation, PanelMesh, PanelPillar, PanelPillarLine, PanelPoint, PanelPointId,
	PanelQuadMesh, PanelTriangle, ParsePanelComplexError, QuadPanel, QuadPanelComplex, RectInset,
	Rectangle, RectangularNTube, RectangularNTubeCorner, RectangularNTubeStation, RectangularStrip,
	RectangularStripNode, RuledPitch, RuledStrip, SharedEdge, TessellatedTrianglePanel, Tube,
	TubeCorners, TubeCrossSectionNode, TubeFaces, TubeFrame, DEFAULT_PANEL_THICKNESS,
	DEFAULT_SEGMENTS, MIN_SEGMENTS,
};
pub use placer::{
	enclosure_soft_goal_met, init_host, pack_kinds, pick_kind, propose_from_spec,
	soft_goal_from_placed, try_corner_l, try_free_extent, try_peninsula_from_run, try_wall_long,
	CommitEffect, FreeExtentKnobs, KindSpec, OccupiedBudget, PackHost, PackKnobs, Predicate,
	ProgramTier, ProposeKnobs, SoftGoalRole, WallLongKnobs, WalledRoomFill, WALL_EPS,
};
pub use portals::{
	ArcRegion, AssignedPortal, MustAssignPortal, Portal, PortalFootprint, WallRegion, SLICE_Y_FRAC,
};
pub use shells::{
	ArcFloor, ArcFloorParams, ArcFloorSlab, ArcTower, ArcTowerParams, CircRingFloor,
	CircRingFloorParams, CircRingFloorSlab, ConnectingHall, EndCap, IFloor, IFloorParams,
	IFloorPlanRect, IFloorSlab, Overhang, PitchedRoof, PitchedRoofParams, RectFloor,
	RectFloorParams, RectFloorSide, RectFloorSlab, RectRingFloor, RectRingFloorParams,
	RectRingFloorSide, RectRingFloorSlab, RectangularPitchedRoofComplex,
	RectangularPitchedRoofComplexParams, RidgeJunction, RoofHalf, RoundedRectCorner,
	RoundedRectFloor, RoundedRectFloorParams, RoundedRectFloorSide, RoundedRectFloorSlab,
	Trazaloid, TrazaloidParams, TrazaloidSide, TrazaloidSlab, ValleySegment,
};
pub use stacked_rings::{StackedRing, StackedRings};
pub use storeys::i_apartment::{
	IApartmentFloorPlan, IApartmentFullStorey, IApartmentParameterized, SCOPE as I_APARTMENT_SCOPE,
};
pub use storeys::i_livable::{
	ILivableFloorPlan, ILivableParameterized, ILivableStorey, MIN_POCKET_SHAFT_SIDE,
};
pub use storeys::les_halles::{
	LesHallesArcadeStorey, LesHallesArcadeUsage, LesHallesCommercialUsage, LesHallesFloorPlan,
	LesHallesFullStorey, LesHallesLivableFullStorey, LesHallesLivableUsage,
	LesHallesOpeningProgram, LesHallesParameterized, LesHallesPlacedDoor, LesHallesShaftPlacement,
	LesHallesStallDoor, LesHallesUsagePlan, SCOPE as LES_HALLES_SCOPE,
};
pub use usage_areas::{
	passages_on_faces, BitesSitdownStall, BitesStall, CardinalFace, CommercialStall,
	CommercialStallInterior, CommercialStallParameterized, CommercialStallPlan,
	CommercialStallStrip, CommercialStallStripParameterized, CommercialStallStripPlan,
	CommonBedroom, CommonBedroomParameterized, CommonBedroomPlan, DiningRoom,
	DiningRoomParameterized, DiningRoomPlan, EatingArea, EatingAreaParameterized, EatingAreaPlan,
	EnclosedRoom, EnclosedRoomMins, EnclosedRoomParams, HallConnectedGroups, HallEnclosedSuites,
	HallSuiteEncloseParams, HallSuitePackParams, HallsToShafts, HallsToShaftsOptions, Janitorial,
	Kitchen, KitchenCounterLayout, KitchenParameterized, KitchenPlan, KnickKnackStall,
	KnickKnackStallParameterized, KnickKnackStallPlan, LivableApartment, LivableApartments,
	LivableApartmentsOptions, LivableApartmentsParameterized, LivingRoom, LivingRoomParameterized,
	LivingRoomPlan, MiniMart, PartsStall, PassageClearance, PlanCell, PlanHost, PublicRestroom,
	PublicRestroomParameterized, PublicRestroomPlan, RectAreaRoom, RectLivableStrategy,
	RectQuarterKind, RectangularLivableArea, RectangularLivableAreaParameterized,
	RectangularLivableAreaPlan, ResidentialBathroom, ResidentialBathroomParameterized,
	ResidentialBathroomPlan, ResidentialHalfBathroom, ResidentialHalfBathroomParameterized,
	ResidentialHalfBathroomPlan, SittingRoom, SittingRoomParameterized, SittingRoomPlan, Study,
	StudyParameterized, StudyPlan, COMMON_BEDROOM_SCOPE, DEFAULT_CLOSED_MAX_AREA, DEFAULT_MIN_HALL,
	DINING_ROOM_SCOPE, DOOR_HEIGHT, DOOR_SIZE_SCALE, DOOR_WIDTH, EATING_AREA_SCOPE, KITCHEN_SCOPE,
	LIVING_ROOM_SCOPE, MAX_HALL_WIDTH, MIN_HALL_WIDTH, PASSAGE_CLEARANCE, RECT_LIVABLE_SCOPE,
	RESIDENTIAL_BATHROOM_SCOPE, RESIDENTIAL_HALF_BATHROOM_SCOPE, SITTING_ROOM_SCOPE, STUDY_SCOPE,
};
pub use wall_demo::{NoisyRectangularWall, NoisyRectangularWallParams};
