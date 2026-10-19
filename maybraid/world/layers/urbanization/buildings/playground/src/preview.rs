//! Preview subject sync. Viewer tracking lives in [`lod::LodRefreshCorePlugin`].

use crate::commands::show::opening::{openings_from_preview, PreviewOpening};
use crate::commands::show::rectangular_pitched_roof_complex::build_params as build_roof_complex_params;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use bevy_math::bounding::Aabb3d;
use bevy_math::{Isometry3d, Vec2};
use building_components::panels::{PanelGeometry, PanelNode, TessellatedTriangle};
use building_components::partitions::rough_stonework::{
	RoughStonework180, RoughStonework90, RoughStoneworkLinear, RoughStoneworkSlice90,
};
use building_components::partitions::{Partition, PartitionNode};
use building_components::roofs::{Pitch, RoofGeometry, RoofNode};
use building_components::Placement;
use building_components::{pose, BuildingComponents, LabelNode};
use buildings::panel_complex::{PanelComplex, PanelComplexJointPolicy, PanelPoint};
use buildings::portals::{MustAssignPortal, Portal};
use buildings::quad_panel::QuadPanel;
use buildings::quad_panel_complex::QuadPanelComplex;
use buildings::stacked_rings::StackedRings;
use buildings::tessellated_triangle_panel::TessellatedTrianglePanel;
use buildings::wall_demo::{NoisyRectangularWall, NoisyRectangularWallParams};
use buildings::wizards_tower::WizardsTower;
use buildings::{
	passages_on_faces, BitesSitdownStall, BitesStall, CardinalFace, CellConstraints,
	CommercialStall, CommercialStallStrip, CommonBedroom, Confines, DiningRoom, FillRegion,
	FillableRegions, Fit, FitError, HallsToShafts as HallsToShaftsFit, HallsToShaftsOptions,
	IApartmentFloorPlan, IApartmentFullStorey, IApartmentParameterized, Kitchen, KnickKnackStall,
	LesHallesFloorPlan, LesHallesFullStorey, LesHallesLivableFullStorey, LesHallesParameterized,
	LesHallesShaftPlacement, LivableApartment, LivableApartments, LivableApartmentsOptions,
	LivingRoom, MiniMart, MixedUseLesHallesMonotower, MultiConfines, PartsStall, PublicRestroom,
	RectAreaRoom, RectLivableStrategy, RectQuarterKind, RectangularLivableArea,
	RectangularLivableAreaParameterized, ResidentialBathroom, ResidentialHalfBathroom, SittingRoom,
	SpaceKind, Study,
};
use buildings::{
	ApproximatedCircle, ArcFloor, ArcFloorParams, ArcFloorSlab, ArcSweep, ArcTower, ArcTowerParams,
	CircRingFloor, CircRingFloorParams, CircRingFloorSlab, ClippedArcSweep, ClippedFittedRectangle,
	ClippedFittedRectangularStrip, ClippedQuadPanel, ClippedRectangle, ClippedRectangularStrip,
	ClippedRuledStrip, ClippedTessellatedTriangle, ConnectingHall, ConnectingShells,
	FittedRectangle, IFloor, IFloorParams, IFloorSlab, MappedOpening, MappedOpeningQuad,
	MapsOpenings, Opening, OpeningId, OpeningLabel, Openings, PitchedRoof, PitchedRoofParams,
	RectFloor, RectFloorParams, RectFloorSlab, RectInset, RectRingFloor, RectRingFloorParams,
	RectRingFloorSlab, Rectangle, RectangularNTube, RectangularNTubeCorner,
	RectangularNTubeStation, RectangularPitchedRoofComplex, RectangularStripNode, RoundedRectFloor,
	RoundedRectFloorParams, RoundedRectFloorSlab, RuledPitch, Trazaloid, TrazaloidParams,
	TrazaloidSlab, Tube, TubeCrossSectionNode, TubeFaces, WellAabb, WellSide,
	DEFAULT_PANEL_THICKNESS,
};
use lod::scene::LodScene;
use lod::{point_bounds, LodNode, LodNodeBounds, LodNodePose, LodRef, LodViewer};
use procedural_common::{AllowedAngles, NoiseParams, StepLenRange};
use urbanization_developments::MixedUseLesHallesDevelopment;
#[derive(Component)]
pub struct PreviewRoot;

/// Cardinal façade for demo bites-stall passages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitesDoorSide {
	South,
	North,
	East,
	West,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PreviewSubject {
	None,
	Linear,
	Arc90,
	Arc180,
	Slice90,
	Pitch {
		rise: f32,
		run: f32,
		length: Option<f32>,
		tile_width: f32,
		left: Option<f32>,
		right: Option<f32>,
	},
	TessellatedTriangle {
		a: Vec2,
		b: Vec2,
		c: Vec2,
	},
	TessellatedTriangle3d {
		a: Vec3,
		b: Vec3,
		c: Vec3,
	},
	ClippedTessellatedTriangle {
		a: Vec3,
		b: Vec3,
		c: Vec3,
		clip: Vec<Vec3>,
		min_dihedral: f32,
		no_joint: bool,
	},
	ClippedQuadPanel {
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
		clip: Vec<Vec3>,
		min_dihedral: f32,
		no_joint: bool,
	},
	ClippedRuledStrip {
		min_dihedral: f32,
		no_joint: bool,
	},
	Tube {
		min_dihedral: f32,
		no_joint: bool,
		no_floor: bool,
		no_ceiling: bool,
		no_left: bool,
		no_right: bool,
	},
	ConnectingHall,
	ConnectingStairwell {
		case: crate::commands::show::connecting_stairwell::StairwellCase,
		tread_fill: f32,
		kind: crate::commands::show::connecting_stairwell::StairwellFit,
	},
	/// Pathological wells in a grid.
	ConnectingStairwellExamples {
		kind: crate::commands::show::connecting_stairwell::StairwellFit,
	},
	ArcFloor {
		radius: f32,
		storey_height: f32,
		floor: bool,
		ceiling: bool,
		openings: Vec<PreviewOpening>,
	},
	ArcTower {
		radius: f32,
		floor_count: u32,
		storey_height: f32,
		floor_hole: f32,
		no_base_floor: bool,
		no_ceiling: bool,
	},
	ConnectingShells,
	Trazaloid {
		footprint_x: f32,
		footprint_z: f32,
		ridge_x: f32,
		ridge_z: f32,
		lower_height: f32,
		upper_height: f32,
		band_vertical_offset: f32,
		waist_horizontal_offset: f32,
		openings: Vec<PreviewOpening>,
		floor: bool,
		no_ceiling: bool,
		face_post_count: u32,
	},
	PitchedRectangularRoof {
		footprint_x: f32,
		footprint_z: f32,
		ridge_height: f32,
		eave_height: f32,
		ridge_inset: f32,
		gables: bool,
		no_walls: bool,
		no_hips: bool,
		openings: Vec<PreviewOpening>,
	},
	RectangularPitchedRoofComplex {
		preset: String,
		overhang_fixed: f32,
		overhang_ratio: Option<f32>,
		end_cap_gable: bool,
		gable_ridge: f32,
		gable_eave: f32,
		run_up: f32,
		/// Demo aperture on roof 0 / half 0 after geometry solve.
		skylight: bool,
	},
	RectFloor {
		footprint_x: f32,
		footprint_z: f32,
		storey_height: f32,
		openings: Vec<PreviewOpening>,
		floor: bool,
		ceiling: bool,
	},
	RoundedRectFloor {
		footprint_x: f32,
		footprint_z: f32,
		storey_height: f32,
		corner_radius: f32,
		corner_segments: u32,
		openings: Vec<PreviewOpening>,
		floor: bool,
		ceiling: bool,
	},
	IFloor {
		central_x: f32,
		central_z: f32,
		storey_height: f32,
		top_left: Option<f32>,
		top_right: Option<f32>,
		bottom_left: Option<f32>,
		bottom_right: Option<f32>,
		openings: Vec<PreviewOpening>,
		floor: bool,
		ceiling: bool,
	},
	RectRingFloor {
		outer_x: f32,
		outer_z: f32,
		inner_x: f32,
		inner_z: f32,
		storey_height: f32,
		openings: Vec<PreviewOpening>,
		floor: bool,
		ceiling: bool,
	},
	CircRingFloor {
		outer_radius: f32,
		inner_radius: f32,
		storey_height: f32,
		openings: Vec<PreviewOpening>,
		floor: bool,
		ceiling: bool,
	},
	Rectangle {
		origin: Vec3,
		edge: Vec3,
		height: f32,
		thickness: f32,
		roll: f32,
	},
	ClippedRectangle {
		origin: Vec3,
		edge: Vec3,
		height: f32,
		thickness: f32,
		roll: f32,
		left: f32,
		right: f32,
		bottom: f32,
		top: f32,
	},
	ClippedRectangularStrip {
		inset: f32,
		min_dihedral: f32,
		no_joint: bool,
	},
	FittedRectangle {
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
	},
	ClippedFittedRectangle {
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
		left: f32,
		right: f32,
		bottom: f32,
		top: f32,
	},
	ClippedFittedRectangularStrip {
		inset: f32,
		min_dihedral: f32,
		no_joint: bool,
	},
	RectangularNTube {
		inset: f32,
		min_dihedral: f32,
		no_joint: bool,
		omit_faces: Vec<usize>,
	},
	ApproximatedCircle {
		center: Vec3,
		radius: f32,
		segments: u32,
		clip: Option<f32>,
	},
	ArcSweep {
		radius: f32,
		height: f32,
		sweep_degrees: f32,
		start_yaw_deg: f32,
	},
	ClippedArcSweep {
		radius: f32,
		height: f32,
		sweep_degrees: f32,
		start_yaw_deg: f32,
	},
	QuadPanel {
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
		t_a0: f32,
		t_a1: f32,
		t_b0: f32,
		t_b1: f32,
		min_dihedral: f32,
		no_joint: bool,
	},
	PanelComplex {
		mesh: String,
		min_dihedral: f32,
		no_joint: bool,
	},
	QuadPanelComplex {
		mesh: String,
		min_dihedral: f32,
		no_joint: bool,
	},
	RuledPitch {
		min_dihedral: f32,
		no_joint: bool,
	},
	Polyline,
	NoisyRectangularWall {
		distance: f32,
		step_len: StepLenRange,
		allowed_angles: AllowedAngles,
		path_noise: NoiseParams,
	},
	WizardsTower {
		noise: f32,
	},
	StackedRings {
		floor_count: u32,
		floor_height: f32,
		radius: f32,
	},
	Bedroom {
		/// Cell size along X / Y / Z (AABB from origin to `extent`).
		extent: Vec3,
		/// Unit noise for layout fitting.
		noise: f32,
		spaciousness: f32,
		occupancy: f32,
		/// When true, punch a south (−Z) passage for entry clearance.
		door: bool,
	},
	/// Side-by-side gallery of CommonBedroom variants (passage boxes as gizmos).
	BedroomExamples,
	ResidentialBathroom {
		extent: Vec3,
		seed: i32,
		door: bool,
	},
	ResidentialHalfBathroom {
		extent: Vec3,
		seed: i32,
		door: bool,
	},
	/// Side-by-side gallery of full + half residential bathrooms (passage boxes as gizmos).
	ResidentialBathroomExamples,
	KitchenExamples,
	DiningRoomExamples,
	LivingRoomExamples,
	SittingRoomExamples,
	StudyExamples,
	CommercialStall {
		extent: Vec3,
		seed: i32,
	},
	CommercialStallStrip {
		extent: Vec3,
		seed: i32,
	},
	BitesStall {
		extent: Vec3,
		seed: i32,
		door_side: BitesDoorSide,
	},
	BitesSitdownStall {
		extent: Vec3,
		seed: i32,
		door_side: BitesDoorSide,
	},
	/// Side-by-side gallery of bites + sit-down variants (passage boxes as gizmos).
	BitesExamples,
	MiniMart {
		extent: Vec3,
		seed: i32,
		door_side: BitesDoorSide,
	},
	/// Side-by-side gallery of MiniMart variants (passage boxes as gizmos).
	MiniMartExamples,
	PartsStall {
		extent: Vec3,
		seed: i32,
		door_side: BitesDoorSide,
	},
	/// Side-by-side gallery of Parts variants (passage boxes as gizmos).
	PartsExamples,
	KnickKnackStall {
		extent: Vec3,
		seed: i32,
		door_side: BitesDoorSide,
	},
	/// Side-by-side gallery of KnickKnack variants (passage boxes as gizmos).
	KnickKnackExamples,
	PublicRestroom {
		extent: Vec3,
		seed: i32,
		door_side: BitesDoorSide,
	},
	/// Side-by-side gallery of PublicRestroom variants (passage boxes as gizmos).
	PublicRestroomExamples,
	LesHallesFloorPlan {
		/// Confines size (XZ centered at origin; Y from 0).
		extent: Vec3,
		seed: i32,
		ceiling: bool,
		/// Inbound openings (`--opening`). Empty ⇒ demo requests all shaft slots.
		openings: Vec<PreviewOpening>,
	},
	/// Side-by-side gallery of `LesHallesFloorPlan` (corner vs mid-side shafts).
	LesHallesFloorPlanExamples,
	LesHallesFullStorey {
		/// Confines size (XZ centered at origin; Y from 0).
		extent: Vec3,
		seed: i32,
		ceiling: bool,
		/// Inbound openings (`--opening`). Empty ⇒ demo requests all shaft slots.
		openings: Vec<PreviewOpening>,
	},
	LesHallesLivableFullStorey {
		/// Confines size (XZ centered at origin; Y from 0).
		extent: Vec3,
		seed: i32,
		ceiling: bool,
		/// Inbound openings (`--opening`). Empty ⇒ demo requests all shaft slots.
		openings: Vec<PreviewOpening>,
	},
	/// Side-by-side gallery of `LesHallesLivableFullStorey` (lengthwise RLA bays).
	LesHallesLivableFullStoreyExamples,
	/// Commercial-below / livable-above Les Halles monotower stack.
	MixedUseLesHallesMonotower {
		/// Confines size (XZ centered at origin; Y from 0). Tall ⇒ several storeys.
		extent: Vec3,
		seed: i32,
		/// Inbound openings (`--opening`). Empty ⇒ monotower samples shaft slots.
		openings: Vec<PreviewOpening>,
	},
	/// Flattened Les Halles development (storey / stairwell / roof hosts).
	MixedUseLesHallesDevelopment {
		/// Confines size (XZ centered at origin; Y from 0). Tall ⇒ several storeys.
		extent: Vec3,
		seed: i32,
		/// Inbound openings (`--opening`). Empty ⇒ monotower samples shaft slots.
		openings: Vec<PreviewOpening>,
	},
	IApartmentFloorPlan {
		/// Confines size (XZ centered at origin; Y from 0).
		extent: Vec3,
		seed: i32,
		ceiling: bool,
		/// Inbound openings (`--opening`). Empty ⇒ demo boundary shaft requests.
		openings: Vec<PreviewOpening>,
	},
	/// Side-by-side gallery of `IApartmentFloorPlan::fit_to_confines` variants.
	IApartmentFloorPlanExamples,
	IApartmentFullStorey {
		/// Confines size (XZ centered at origin; Y from 0).
		extent: Vec3,
		seed: i32,
		ceiling: bool,
		/// Inbound openings (`--opening`). Empty ⇒ demo boundary shaft requests.
		openings: Vec<PreviewOpening>,
	},
	/// Side-by-side gallery of `IApartmentFullStorey` (LivableApartments packs).
	IApartmentFullStoreyExamples,
	/// Side-by-side gallery of standalone [`LivableApartments`] packs.
	LivableApartmentsExamples,
	/// Side-by-side gallery of standalone [`LivableApartment`] layouts.
	LivableApartmentExamples,
	/// Side-by-side gallery of standalone [`RectangularLivableArea`] fits.
	LivableRectanglesExamples,
	/// HallsToShafts on a rectangular host (gizmo boxes for halls / openings / residuals).
	HallsToShafts {
		extent: Vec3,
		seed: i32,
		/// `None` ⇒ sample 2–4 m; `Some` ⇒ fixed clear width.
		hall_width: Option<f32>,
		openings: Vec<PreviewOpening>,
	},
}

impl Default for PreviewSubject {
	fn default() -> Self {
		Self::None
	}
}

#[derive(Resource, Clone, Debug)]
pub struct PreviewConfig {
	/// When true, draw face Text3d stroke gizmos for [`LabelNode`]s.
	pub label_text: bool,
	pub subject: PreviewSubject,
	pub transform: Transform,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { label_text: true, subject: PreviewSubject::None, transform: Transform::IDENTITY }
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		match self.subject {
			PreviewSubject::None => "preview: (none — `/show …`)".into(),
			PreviewSubject::Linear => "preview: rough-stonework linear".into(),
			PreviewSubject::Arc90 => "preview: rough-stonework arc-90".into(),
			PreviewSubject::Arc180 => "preview: rough-stonework arc-180".into(),
			PreviewSubject::Slice90 => "preview: rough-stonework slice-90".into(),
			PreviewSubject::Pitch {
				rise,
				run,
				length,
				tile_width,
				left,
				right,
			} => {
				format!(
					"preview: pitch (rise={rise:.2} run={run:.2} len={length:?} tile={tile_width:.2} left={left:?} right={right:?})"
				)
			}
			PreviewSubject::TessellatedTriangle { a, b, c } => {
				format!("preview: tessellated-triangle (a={a:?} b={b:?} c={c:?})")
			}
			PreviewSubject::TessellatedTriangle3d { a, b, c } => {
				format!("preview: tessellated-triangle-3d (a={a:?} b={b:?} c={c:?})")
			}
			PreviewSubject::ClippedTessellatedTriangle {
				a,
				b,
				c,
				ref clip,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-tessellated-triangle (a={a:?} b={b:?} c={c:?} clip={clip:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::ClippedQuadPanel {
				a0,
				a1,
				b0,
				b1,
				ref clip,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-quad-panel (a0={a0:?} a1={a1:?} b0={b0:?} b1={b1:?} clip={clip:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::ClippedRuledStrip {
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-ruled-strip (min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::Tube {
				min_dihedral,
				no_joint,
				no_floor,
				no_ceiling,
				no_left,
				no_right,
			} => format!(
				"preview: tube (min_dihedral={min_dihedral:.3} no_joint={no_joint} no_floor={no_floor} no_ceiling={no_ceiling} no_left={no_left} no_right={no_right})"
			),
			PreviewSubject::ConnectingHall => "preview: connecting-hall (one kink)".into(),
			PreviewSubject::ConnectingStairwell { case, tread_fill, kind } => {
				format!("preview: connecting-stairwell ({case:?} {kind:?} fill={tread_fill:.2})")
			}
			PreviewSubject::ConnectingStairwellExamples { kind } => {
				format!("preview: connecting-stairwell-examples ({kind:?} gallery)")
			}
			PreviewSubject::ArcFloor {
				radius,
				storey_height,
				floor,
				ceiling,
				ref openings,
			} => format!(
				"preview: arc-floor (r={radius:.1} h={storey_height:.1} floor={floor} ceil={ceiling} openings={})",
				openings.len()
			),
			PreviewSubject::ArcTower {
				radius,
				floor_count,
				storey_height,
				floor_hole,
				no_base_floor,
				no_ceiling,
			} => format!(
				"preview: arc-tower (r={radius:.1} floors={floor_count} h={storey_height:.1} hole={floor_hole:.2} no_base={no_base_floor} no_ceil={no_ceiling})"
			),
			PreviewSubject::ConnectingShells => {
				"preview: connecting-shells (arc-tower + hall + trazaloid)".into()
			}
			PreviewSubject::Trazaloid {
				footprint_x,
				footprint_z,
				ridge_x,
				ridge_z,
				lower_height,
				upper_height,
				band_vertical_offset,
				waist_horizontal_offset,
				ref openings,
				floor,
				no_ceiling,
				face_post_count,
			} => format!(
				"preview: trazaloid (foot={footprint_x:.1}x{footprint_z:.1} ridge={ridge_x:.1}x{ridge_z:.1} h={lower_height:.1}+{upper_height:.1} gap={band_vertical_offset:.2} inset={waist_horizontal_offset:.2} openings={} floor={floor} ceil=!{no_ceiling} posts={face_post_count})",
				openings.len()
			),
			PreviewSubject::PitchedRectangularRoof {
				footprint_x,
				footprint_z,
				ridge_height,
				eave_height,
				ridge_inset,
				gables,
				no_walls,
				no_hips,
				ref openings,
			} => format!(
				"preview: pitched-rectangular-roof (foot={footprint_x:.1}x{footprint_z:.1} ridge_h={ridge_height:.1} eave_h={eave_height:.1} inset={ridge_inset:.1} gables={gables} walls={} hips={} openings={})",
				!no_walls,
				!no_hips,
				openings.len()
			),
			PreviewSubject::RectangularPitchedRoofComplex {
				ref preset,
				overhang_fixed,
				overhang_ratio,
				end_cap_gable,
				run_up,
				skylight,
				..
			} => format!(
				"preview: rectangular-pitched-roof-complex (preset={preset} overhang={} end={} run_up={run_up:.2} skylight={skylight})",
				overhang_ratio
					.map(|r| format!("ratio={r:.2}"))
					.unwrap_or_else(|| format!("fixed={overhang_fixed:.2}")),
				if end_cap_gable { "gable" } else { "hip" }
			),
			PreviewSubject::RectFloor {
				footprint_x,
				footprint_z,
				storey_height,
				ref openings,
				floor,
				ceiling,
			} => format!(
				"preview: rect-floor (foot={footprint_x:.1}x{footprint_z:.1} h={storey_height:.1} openings={} floor={floor} ceil={ceiling})",
				openings.len()
			),
			PreviewSubject::RoundedRectFloor {
				footprint_x,
				footprint_z,
				storey_height,
				corner_radius,
				corner_segments,
				ref openings,
				floor,
				ceiling,
			} => format!(
				"preview: rounded-rect-floor (foot={footprint_x:.1}x{footprint_z:.1} h={storey_height:.1} r={corner_radius:.2} segs={corner_segments} openings={} floor={floor} ceil={ceiling})",
				openings.len()
			),
			PreviewSubject::IFloor {
				central_x,
				central_z,
				storey_height,
				ref openings,
				floor,
				ceiling,
				..
			} => format!(
				"preview: i-floor (central={central_x:.1}x{central_z:.1} h={storey_height:.1} openings={} floor={floor} ceil={ceiling})",
				openings.len()
			),
			PreviewSubject::RectRingFloor {
				outer_x,
				outer_z,
				inner_x,
				inner_z,
				storey_height,
				ref openings,
				floor,
				ceiling,
				..
			} => format!(
				"preview: rect-ring-floor (outer={outer_x:.1}x{outer_z:.1} inner={inner_x:.1}x{inner_z:.1} h={storey_height:.1} openings={} floor={floor} ceil={ceiling})",
				openings.len()
			),
			PreviewSubject::CircRingFloor {
				outer_radius,
				inner_radius,
				storey_height,
				ref openings,
				floor,
				ceiling,
			} => format!(
				"preview: circ-ring-floor (R={outer_radius:.1} r={inner_radius:.1} h={storey_height:.1} openings={} floor={floor} ceil={ceiling})",
				openings.len()
			),
			PreviewSubject::Rectangle {
				origin,
				edge,
				height,
				thickness,
				roll,
			} => {
				format!(
					"preview: rectangle (origin={origin:?} edge={edge:?} height={height:.3} thickness={thickness:.3} roll={roll:.3})"
				)
			}
			PreviewSubject::ClippedRectangle {
				origin,
				edge,
				height,
				thickness,
				roll,
				left,
				right,
				bottom,
				top,
			} => format!(
				"preview: clipped-rectangle (origin={origin:?} edge={edge:?} height={height:.3} thickness={thickness:.3} roll={roll:.3} inset=[{left:.2},{right:.2},{bottom:.2},{top:.2}])"
			),
			PreviewSubject::ClippedRectangularStrip {
				inset,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-rectangular-strip (inset={inset:.2} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::FittedRectangle { a0, a1, b0, b1 } => {
				format!("preview: fitted-rectangle (a0={a0:?} a1={a1:?} b0={b0:?} b1={b1:?})")
			}
			PreviewSubject::ClippedFittedRectangle {
				a0,
				a1,
				b0,
				b1,
				left,
				right,
				bottom,
				top,
			} => format!(
				"preview: clipped-fitted-rectangle (a0={a0:?} a1={a1:?} b0={b0:?} b1={b1:?} inset=[{left:.2},{right:.2},{bottom:.2},{top:.2}])"
			),
			PreviewSubject::ClippedFittedRectangularStrip {
				inset,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-fitted-rectangular-strip (inset={inset:.2} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::RectangularNTube {
				inset,
				min_dihedral,
				no_joint,
				ref omit_faces,
			} => format!(
				"preview: rectangular-n-tube (inset={inset:.2} min_dihedral={min_dihedral:.3} no_joint={no_joint} omit_faces={omit_faces:?})"
			),
			PreviewSubject::ApproximatedCircle {
				center,
				radius,
				segments,
				clip,
			} => format!(
				"preview: approximated-circle (c={center:?} r={radius:.2} n={segments} clip={clip:?})"
			),
			PreviewSubject::ArcSweep {
				radius,
				height,
				sweep_degrees,
				start_yaw_deg,
			} => format!(
				"preview: arc-sweep (r={radius:.2} h={height:.2} sweep={sweep_degrees:.1} yaw0={start_yaw_deg:.1})"
			),
			PreviewSubject::ClippedArcSweep {
				radius,
				height,
				sweep_degrees,
				start_yaw_deg,
			} => format!(
				"preview: clipped-arc-sweep (r={radius:.2} h={height:.2} sweep={sweep_degrees:.1} yaw0={start_yaw_deg:.1})"
			),
			PreviewSubject::QuadPanel {
				a0,
				a1,
				b0,
				b1,
				t_a0,
				t_a1,
				t_b0,
				t_b1,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: quad-panel (a0={a0:?} a1={a1:?} b0={b0:?} b1={b1:?} t=[{t_a0:.2},{t_a1:.2},{t_b0:.2},{t_b1:.2}] min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::PanelComplex {
				ref mesh,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: panel-complex (mesh={mesh:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::QuadPanelComplex {
				ref mesh,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: quad-panel-complex (mesh={mesh:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::RuledPitch {
				min_dihedral,
				no_joint,
			} => format!(
				"preview: ruled-pitch (min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::Polyline => "preview: partition polyline (L)".into(),
			PreviewSubject::NoisyRectangularWall {
				distance,
				step_len,
				allowed_angles,
				path_noise,
			} => format!(
				"preview: noisy-rectangular-wall (d={distance:.1} step=[{:.2},{:.2}] ang=({:.2},{:.2},{:.2}) seed={})",
				step_len.min, step_len.max,
				allowed_angles.x, allowed_angles.y, allowed_angles.z, path_noise.seed
			),
			PreviewSubject::WizardsTower { noise } => {
				format!("preview: wizards-tower (noise={noise:.2})")
			}
			PreviewSubject::StackedRings {
				floor_count,
				floor_height,
				radius,
			} => format!(
				"preview: stacked-rings (n={floor_count} h={floor_height:.2} r={radius:.2})"
			),
			PreviewSubject::Bedroom {
				extent,
				noise,
				spaciousness,
				occupancy,
				door,
			} => {
				format!(
					"preview: bedroom (extent={:.2},{:.2},{:.2} noise={noise:.2} space={spaciousness:.2} occ={occupancy:.2} door={door})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::BedroomExamples => "preview: bedroom-examples (gallery)".into(),
			PreviewSubject::ResidentialBathroom { extent, seed, door } => {
				format!(
					"preview: residential-bathroom (extent={:.2},{:.2},{:.2} seed={seed} door={door})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::ResidentialHalfBathroom { extent, seed, door } => {
				format!(
					"preview: residential-half-bathroom (extent={:.2},{:.2},{:.2} seed={seed} door={door})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::ResidentialBathroomExamples => {
				"preview: residential-bathroom-examples (gallery)".into()
			}
			PreviewSubject::KitchenExamples => "preview: kitchen-examples (gallery)".into(),
			PreviewSubject::DiningRoomExamples => "preview: dining-room-examples (gallery)".into(),
			PreviewSubject::LivingRoomExamples => "preview: living-room-examples (gallery)".into(),
			PreviewSubject::SittingRoomExamples => "preview: sitting-room-examples (gallery)".into(),
			PreviewSubject::StudyExamples => "preview: study-examples (gallery)".into(),
			PreviewSubject::CommercialStall { extent, seed } => {
				format!(
					"preview: commercial-stall (extent={:.2},{:.2},{:.2} seed={seed})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::CommercialStallStrip { extent, seed } => {
				format!(
					"preview: commercial-stall-strip (extent={:.2},{:.2},{:.2} seed={seed})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::BitesStall {
				extent,
				seed,
				door_side,
			} => {
				format!(
					"preview: bites-stall (extent={:.2},{:.2},{:.2} seed={seed} door-side={door_side:?})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::BitesSitdownStall {
				extent,
				seed,
				door_side,
			} => {
				format!(
					"preview: bites-sitdown-stall (extent={:.2},{:.2},{:.2} seed={seed} door-side={door_side:?})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::BitesExamples => "preview: bites-examples (gallery)".into(),
			PreviewSubject::MiniMart {
				extent,
				seed,
				door_side,
			} => {
				format!(
					"preview: mini-mart (extent={:.2},{:.2},{:.2} seed={seed} door-side={door_side:?})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::MiniMartExamples => "preview: mini-mart-examples (gallery)".into(),
			PreviewSubject::PartsStall {
				extent,
				seed,
				door_side,
			} => {
				format!(
					"preview: parts-stall (extent={:.2},{:.2},{:.2} seed={seed} door-side={door_side:?})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::PartsExamples => "preview: parts-examples (gallery)".into(),
			PreviewSubject::KnickKnackStall {
				extent,
				seed,
				door_side,
			} => {
				format!(
					"preview: knick-knack-stall (extent={:.2},{:.2},{:.2} seed={seed} door-side={door_side:?})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::KnickKnackExamples => "preview: knick-knack-examples (gallery)".into(),
			PreviewSubject::PublicRestroom {
				extent,
				seed,
				door_side,
			} => {
				format!(
					"preview: public-restroom (extent={:.2},{:.2},{:.2} seed={seed} door-side={door_side:?})",
					extent.x, extent.y, extent.z
				)
			}
			PreviewSubject::PublicRestroomExamples => {
				"preview: public-restroom-examples (gallery)".into()
			}
			PreviewSubject::LesHallesFloorPlan {
				extent,
				seed,
				ceiling,
				ref openings,
			} => {
				format!(
					"preview: les-halles-floor-plan (extent={:.1},{:.1},{:.1} seed={seed} ceiling={ceiling} openings={})",
					extent.x,
					extent.y,
					extent.z,
					openings.len()
				)
			}
			PreviewSubject::LesHallesFullStorey {
				extent,
				seed,
				ceiling,
				ref openings,
			} => {
				format!(
					"preview: les-halles-full-storey (extent={:.1},{:.1},{:.1} seed={seed} ceiling={ceiling} openings={})",
					extent.x,
					extent.y,
					extent.z,
					openings.len()
				)
			}
			PreviewSubject::LesHallesLivableFullStorey {
				extent,
				seed,
				ceiling,
				ref openings,
			} => {
				format!(
					"preview: les-halles-livable-full-storey (extent={:.1},{:.1},{:.1} seed={seed} ceiling={ceiling} openings={})",
					extent.x,
					extent.y,
					extent.z,
					openings.len()
				)
			}
			PreviewSubject::LesHallesFloorPlanExamples => {
				"preview: les-halles-floor-plan-examples (gallery)".into()
			}
			PreviewSubject::LesHallesLivableFullStoreyExamples => {
				"preview: les-halles-livable-full-storey-examples (gallery)".into()
			}
			PreviewSubject::MixedUseLesHallesMonotower {
				extent,
				seed,
				ref openings,
			} => {
				format!(
					"preview: mixed-use-les-halles-monotower (extent={:.1},{:.1},{:.1} seed={seed} openings={})",
					extent.x,
					extent.y,
					extent.z,
					openings.len()
				)
			}
			PreviewSubject::MixedUseLesHallesDevelopment {
				extent,
				seed,
				ref openings,
			} => {
				format!(
					"preview: mixed-use-les-halles-development (extent={:.1},{:.1},{:.1} seed={seed} openings={})",
					extent.x,
					extent.y,
					extent.z,
					openings.len()
				)
			}
			PreviewSubject::IApartmentFloorPlan {
				extent,
				seed,
				ceiling,
				ref openings,
			} => {
				format!(
					"preview: i-apartment-floor-plan (extent={:.1},{:.1},{:.1} seed={seed} ceiling={ceiling} openings={})",
					extent.x,
					extent.y,
					extent.z,
					openings.len()
				)
			}
			PreviewSubject::IApartmentFloorPlanExamples => {
				"preview: i-apartment-floor-plan-examples (gallery)".into()
			}
			PreviewSubject::IApartmentFullStoreyExamples => {
				"preview: i-apartment-full-storey-examples (gallery)".into()
			}
			PreviewSubject::LivableApartmentsExamples => {
				"preview: livable-apartments-examples (gallery)".into()
			}
			PreviewSubject::LivableApartmentExamples => {
				"preview: livable-apartment-examples (gallery)".into()
			}
			PreviewSubject::LivableRectanglesExamples => {
				"preview: livable-rectangles-examples (gallery)".into()
			}
			PreviewSubject::IApartmentFullStorey {
				extent,
				seed,
				ceiling,
				ref openings,
			} => {
				format!(
					"preview: i-apartment-full-storey (extent={:.1},{:.1},{:.1} seed={seed} ceiling={ceiling} openings={})",
					extent.x,
					extent.y,
					extent.z,
					openings.len()
				)
			}
			PreviewSubject::HallsToShafts {
				extent,
				seed,
				hall_width,
				ref openings,
			} => {
				let width = hall_width
					.map(|w| format!("width={w:.2}"))
					.unwrap_or_else(|| "width=noise".into());
				format!(
					"preview: halls-to-shafts (extent={:.1},{:.1},{:.1} seed={seed} {width} openings={})",
					extent.x,
					extent.y,
					extent.z,
					openings.len()
				)
			}
		}
	}
}

/// Authored preview payload kept across LOD flips (stable noise / geometry).
#[derive(Resource, Default)]
pub struct CachedPreview {
	key: Option<(PreviewSubject, Transform)>,
	wizards_tower: Option<WizardsTower>,
	stacked_rings: Option<StackedRings>,
	bedroom: Option<CommonBedroom>,
	noisy_wall: Option<NoisyRectangularWall>,
	les_halles_floor_plan: Option<LesHallesFloorPlan>,
	les_halles_floor_plan_examples: Vec<LesHallesFloorPlanExampleCell>,
	les_halles_full_storey: Option<LesHallesFullStorey>,
	les_halles_livable_full_storey: Option<LesHallesLivableFullStorey>,
	les_halles_livable_full_storey_examples: Vec<LesHallesLivableFullStoreyExampleCell>,
	mixed_use_les_halles_monotower: Option<MixedUseLesHallesMonotower>,
	mixed_use_les_halles_development: Option<MixedUseLesHallesDevelopment>,
	i_apartment_floor_plan: Option<IApartmentFloorPlan>,
	i_apartment_floor_plan_examples: Vec<IApartmentFloorPlanExampleCell>,
	i_apartment_full_storey: Option<IApartmentFullStorey>,
	i_apartment_full_storey_examples: Vec<IApartmentFullStoreyExampleCell>,
	livable_apartments_examples: Vec<LivableApartmentsExampleCell>,
	livable_apartment_examples: Vec<LivableApartmentExampleCell>,
	livable_rectangles_examples: Vec<LivableRectangleExampleCell>,
	halls_to_shafts: Option<HallsToShaftsPreview>,
	commercial_stall: Option<CommercialStall>,
	commercial_stall_strip: Option<CommercialStallStrip>,
	bites_stall: Option<BitesStall>,
	bites_sitdown_stall: Option<BitesSitdownStall>,
	mini_mart: Option<MiniMart>,
	parts_stall: Option<PartsStall>,
	knick_knack_stall: Option<KnickKnackStall>,
	public_restroom: Option<PublicRestroom>,
	/// Passage AABBs in the active stall-demo preview local space.
	bites_passages: Vec<(Aabb3d, Vec3)>,
	bites_examples: Vec<BitesExampleCell>,
	mini_mart_examples: Vec<MiniMartExampleCell>,
	parts_examples: Vec<PartsExampleCell>,
	knick_knack_examples: Vec<KnickKnackExampleCell>,
	public_restroom_examples: Vec<PublicRestroomExampleCell>,
	bedroom_examples: Vec<BedroomExampleCell>,
	residential_bathroom: Option<ResidentialBathroom>,
	residential_half_bathroom: Option<ResidentialHalfBathroom>,
	residential_bathroom_examples: Vec<ResidentialBathroomExampleCell>,
	kitchen_examples: Vec<GalleryExampleCell<Kitchen>>,
	dining_room_examples: Vec<GalleryExampleCell<DiningRoom>>,
	living_room_examples: Vec<GalleryExampleCell<LivingRoom>>,
	sitting_room_examples: Vec<GalleryExampleCell<SittingRoom>>,
	study_examples: Vec<GalleryExampleCell<Study>>,
}

/// One cell in [`PreviewSubject::BedroomExamples`].
#[derive(Clone)]
struct BedroomExampleCell {
	offset: Vec3,
	room: CommonBedroom,
}

/// One cell in [`PreviewSubject::ResidentialBathroomExamples`].
#[derive(Clone)]
enum ResidentialBathroomExampleCell {
	Full { offset: Vec3, room: ResidentialBathroom },
	Half { offset: Vec3, room: ResidentialHalfBathroom },
}

/// One cell in livable-quarters example galleries.
#[derive(Clone)]
struct GalleryExampleCell<T> {
	offset: Vec3,
	room: T,
}

/// Cached [`HallsToShaftsFit`] result for the gizmo-only playground demo.
#[derive(Clone)]
struct HallsToShaftsPreview {
	fit: HallsToShaftsFit,
	regions: FillableRegions,
	host: Aabb3d,
}

/// One cell in [`PreviewSubject::IApartmentFloorPlanExamples`].
#[derive(Clone)]
struct IApartmentFloorPlanExampleCell {
	offset: Vec3,
	plan: IApartmentFloorPlan,
}

/// One cell in [`PreviewSubject::IApartmentFullStoreyExamples`].
#[derive(Clone)]
struct IApartmentFullStoreyExampleCell {
	offset: Vec3,
	storey: IApartmentFullStorey,
}

/// One cell in [`PreviewSubject::LesHallesFloorPlanExamples`].
#[derive(Clone)]
struct LesHallesFloorPlanExampleCell {
	offset: Vec3,
	plan: LesHallesFloorPlan,
}

/// One cell in [`PreviewSubject::LesHallesLivableFullStoreyExamples`].
#[derive(Clone)]
struct LesHallesLivableFullStoreyExampleCell {
	offset: Vec3,
	storey: LesHallesLivableFullStorey,
}

/// One cell in [`PreviewSubject::LivableApartmentsExamples`].
#[derive(Clone)]
struct LivableApartmentsExampleCell {
	offset: Vec3,
	block: LivableApartments,
}

/// One cell in [`PreviewSubject::LivableApartmentExamples`].
#[derive(Clone)]
struct LivableApartmentExampleCell {
	offset: Vec3,
	apartment: LivableApartment,
}

/// One cell in [`PreviewSubject::LivableRectanglesExamples`].
#[derive(Clone)]
struct LivableRectangleExampleCell {
	offset: Vec3,
	area: RectangularLivableArea,
}

/// One cell in [`PreviewSubject::PartsExamples`].
#[derive(Clone)]
struct PartsExampleCell {
	offset: Vec3,
	stall: PartsStall,
}

/// One cell in [`PreviewSubject::KnickKnackExamples`].
#[derive(Clone)]
struct KnickKnackExampleCell {
	offset: Vec3,
	stall: KnickKnackStall,
}

/// One cell in [`PreviewSubject::PublicRestroomExamples`].
#[derive(Clone)]
struct PublicRestroomExampleCell {
	offset: Vec3,
	stall: PublicRestroom,
}

/// One cell in [`PreviewSubject::MiniMartExamples`].
#[derive(Clone)]
struct MiniMartExampleCell {
	offset: Vec3,
	stall: MiniMart,
}

/// One cell in [`PreviewSubject::BitesExamples`].
#[derive(Clone)]
enum BitesExampleCell {
	Stall { offset: Vec3, stall: BitesStall },
	Sitdown { offset: Vec3, stall: BitesSitdownStall },
}

impl CachedPreview {
	fn rebuild_if_needed(&mut self, config: &PreviewConfig) {
		let key = (config.subject.clone(), config.transform);
		if self.key.as_ref() == Some(&key) {
			return;
		}
		self.key = Some(key);
		self.wizards_tower = None;
		self.stacked_rings = None;
		self.bedroom = None;
		self.noisy_wall = None;
		self.les_halles_floor_plan = None;
		self.les_halles_floor_plan_examples.clear();
		self.les_halles_full_storey = None;
		self.les_halles_livable_full_storey = None;
		self.les_halles_livable_full_storey_examples.clear();
		self.mixed_use_les_halles_monotower = None;
		self.mixed_use_les_halles_development = None;
		self.i_apartment_floor_plan = None;
		self.i_apartment_floor_plan_examples.clear();
		self.i_apartment_full_storey = None;
		self.i_apartment_full_storey_examples.clear();
		self.livable_apartments_examples.clear();
		self.livable_apartment_examples.clear();
		self.livable_rectangles_examples.clear();
		self.halls_to_shafts = None;
		self.commercial_stall = None;
		self.commercial_stall_strip = None;
		self.bites_stall = None;
		self.bites_sitdown_stall = None;
		self.mini_mart = None;
		self.parts_stall = None;
		self.knick_knack_stall = None;
		self.public_restroom = None;
		self.bites_passages.clear();
		self.bites_examples.clear();
		self.mini_mart_examples.clear();
		self.parts_examples.clear();
		self.knick_knack_examples.clear();
		self.public_restroom_examples.clear();
		self.bedroom_examples.clear();
		self.residential_bathroom = None;
		self.residential_half_bathroom = None;
		self.residential_bathroom_examples.clear();
		self.kitchen_examples.clear();
		self.dining_room_examples.clear();
		self.living_room_examples.clear();
		self.sitting_room_examples.clear();
		self.study_examples.clear();
		match &config.subject {
			PreviewSubject::WizardsTower { noise } => {
				let footprint = CellConstraints::cell_owned(Aabb3d::from_min_max(
					Vec3::new(-4.0, 0.0, -4.0),
					Vec3::new(4.0, 3.0, 4.0),
				));
				self.wizards_tower = Some(WizardsTower::new(&footprint, *noise));
			}
			PreviewSubject::StackedRings { floor_count, floor_height, radius } => {
				self.stacked_rings = Some(StackedRings::new(*floor_count, *floor_height, *radius));
			}
			PreviewSubject::Bedroom { extent, noise, spaciousness, occupancy, door } => {
				let confines = demo_common_bedroom_confines(*extent, *door);
				self.bites_passages = passage_aabbs_at(&confines, Vec3::ZERO);
				let seed =
					NoiseParams { seed: (*noise * 1_000_000.0) as i32, ..NoiseParams::default() };
				match CommonBedroom::fit_with_fill(
					&confines,
					seed,
					buildings::CommonBedroomParameterized::with_fill(*spaciousness, *occupancy),
				) {
					Ok((room, _)) => self.bedroom = Some(room),
					Err(err) => bevy::log::error!("common-bedroom fit failed: {err}"),
				}
			}
			PreviewSubject::BedroomExamples => {
				let (cells, passages) = build_bedroom_examples();
				self.bedroom_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::ResidentialBathroom { extent, seed, door } => {
				let confines = demo_common_bedroom_confines(*extent, *door);
				self.bites_passages = passage_aabbs_at(&confines, Vec3::ZERO);
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match ResidentialBathroom::fit_to_confines(&confines, noise) {
					Ok((room, _)) => self.residential_bathroom = Some(room),
					Err(err) => bevy::log::error!("residential-bathroom fit failed: {err}"),
				}
			}
			PreviewSubject::ResidentialHalfBathroom { extent, seed, door } => {
				let confines = demo_common_bedroom_confines(*extent, *door);
				self.bites_passages = passage_aabbs_at(&confines, Vec3::ZERO);
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match ResidentialHalfBathroom::fit_to_confines(&confines, noise) {
					Ok((room, _)) => self.residential_half_bathroom = Some(room),
					Err(err) => bevy::log::error!("residential-half-bathroom fit failed: {err}"),
				}
			}
			PreviewSubject::ResidentialBathroomExamples => {
				let (cells, passages) = build_residential_bathroom_examples();
				self.residential_bathroom_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::KitchenExamples => {
				let (cells, passages) = build_kitchen_examples();
				self.kitchen_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::DiningRoomExamples => {
				let (cells, passages) = build_dining_room_examples();
				self.dining_room_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::LivingRoomExamples => {
				let (cells, passages) = build_living_room_examples();
				self.living_room_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::SittingRoomExamples => {
				let (cells, passages) = build_sitting_room_examples();
				self.sitting_room_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::StudyExamples => {
				let (cells, passages) = build_study_examples();
				self.study_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::NoisyRectangularWall {
				distance,
				step_len,
				allowed_angles,
				path_noise,
			} => {
				self.noisy_wall = Some(NoisyRectangularWall::new(NoisyRectangularWallParams {
					distance: *distance,
					step_len: *step_len,
					allowed_angles: *allowed_angles,
					path_noise: *path_noise,
					must_assign: vec![MustAssignPortal::at(0.5, Portal::Window)],
					optional_portals: (0, 0),
					..NoisyRectangularWallParams::default()
				}));
			}
			PreviewSubject::CommercialStall { extent, seed } => {
				let confines = Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, *extent));
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match CommercialStall::fit_to_confines(&confines, noise) {
					Ok((stall, _)) => self.commercial_stall = Some(stall),
					Err(err) => bevy::log::error!("commercial-stall fit failed: {err}"),
				}
			}
			PreviewSubject::CommercialStallStrip { extent, seed } => {
				let confines = demo_commercial_stall_strip_confines(*extent, *seed);
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match CommercialStallStrip::fit_to_confines(&confines, noise) {
					Ok((strip, _)) => self.commercial_stall_strip = Some(strip),
					Err(err) => bevy::log::error!("commercial-stall-strip fit failed: {err}"),
				}
			}
			PreviewSubject::BitesStall { extent, seed, door_side } => {
				let confines = demo_bites_stall_confines(*extent, *door_side);
				self.bites_passages = passage_aabbs_at(&confines, Vec3::ZERO);
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match BitesStall::fit_to_confines(&confines, noise) {
					Ok((stall, _)) => self.bites_stall = Some(stall),
					Err(err) => bevy::log::error!("bites-stall fit failed: {err}"),
				}
			}
			PreviewSubject::BitesSitdownStall { extent, seed, door_side } => {
				let confines = demo_bites_stall_confines(*extent, *door_side);
				self.bites_passages = passage_aabbs_at(&confines, Vec3::ZERO);
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match BitesSitdownStall::fit_to_confines(&confines, noise) {
					Ok((stall, _)) => self.bites_sitdown_stall = Some(stall),
					Err(err) => bevy::log::error!("bites-sitdown-stall fit failed: {err}"),
				}
			}
			PreviewSubject::BitesExamples => {
				let (cells, passages) = build_bites_examples();
				self.bites_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::MiniMart { extent, seed, door_side } => {
				let confines = demo_bites_stall_confines(*extent, *door_side);
				self.bites_passages = passage_aabbs_at(&confines, Vec3::ZERO);
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match MiniMart::fit_to_confines(&confines, noise) {
					Ok((stall, _)) => self.mini_mart = Some(stall),
					Err(err) => bevy::log::error!("mini-mart fit failed: {err}"),
				}
			}
			PreviewSubject::MiniMartExamples => {
				let (cells, passages) = build_mini_mart_examples();
				self.mini_mart_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::PartsStall { extent, seed, door_side } => {
				let confines = demo_bites_stall_confines(*extent, *door_side);
				self.bites_passages = passage_aabbs_at(&confines, Vec3::ZERO);
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match PartsStall::fit_to_confines(&confines, noise) {
					Ok((stall, _)) => self.parts_stall = Some(stall),
					Err(err) => bevy::log::error!("parts-stall fit failed: {err}"),
				}
			}
			PreviewSubject::PartsExamples => {
				let (cells, passages) = build_parts_examples();
				self.parts_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::KnickKnackStall { extent, seed, door_side } => {
				let confines = demo_bites_stall_confines(*extent, *door_side);
				self.bites_passages = passage_aabbs_at(&confines, Vec3::ZERO);
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match KnickKnackStall::fit_to_confines(&confines, noise) {
					Ok((stall, _)) => self.knick_knack_stall = Some(stall),
					Err(err) => bevy::log::error!("knick-knack-stall fit failed: {err}"),
				}
			}
			PreviewSubject::KnickKnackExamples => {
				let (cells, passages) = build_knick_knack_examples();
				self.knick_knack_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::PublicRestroom { extent, seed, door_side } => {
				let confines = demo_bites_stall_confines(*extent, *door_side);
				self.bites_passages = passage_aabbs_at(&confines, Vec3::ZERO);
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match PublicRestroom::fit_to_confines(&confines, noise) {
					Ok((stall, _)) => self.public_restroom = Some(stall),
					Err(err) => bevy::log::error!("public-restroom fit failed: {err}"),
				}
			}
			PreviewSubject::PublicRestroomExamples => {
				let (cells, passages) = build_public_restroom_examples();
				self.public_restroom_examples = cells;
				self.bites_passages = passages;
			}
			PreviewSubject::LesHallesFloorPlan { extent, seed, ceiling, openings } => {
				match fit_les_halles_floor_plan(*extent, *seed, *ceiling, openings) {
					Ok(plan) => self.les_halles_floor_plan = Some(plan),
					Err(err) => {
						bevy::log::error!("les-halles-floor-plan fit failed: {err}");
					}
				}
			}
			PreviewSubject::LesHallesFloorPlanExamples => {
				self.les_halles_floor_plan_examples = build_les_halles_floor_plan_examples();
			}
			PreviewSubject::LesHallesFullStorey { extent, seed, ceiling, openings } => {
				match fit_les_halles_floor_plan(*extent, *seed, *ceiling, openings) {
					Ok(plan) => {
						let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
						match LesHallesFullStorey::from_floor_plan(plan, noise) {
							Ok((storey, _)) => self.les_halles_full_storey = Some(storey),
							Err(err) => {
								bevy::log::error!("les-halles-full-storey fill failed: {err}");
							}
						}
					}
					Err(err) => {
						bevy::log::error!("les-halles-full-storey fit failed: {err}");
					}
				}
			}
			PreviewSubject::LesHallesLivableFullStorey { extent, seed, ceiling, openings } => {
				match fit_les_halles_livable_floor_plan(*extent, *seed, *ceiling, openings) {
					Ok(plan) => {
						let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
						match LesHallesLivableFullStorey::from_floor_plan(plan, noise) {
							Ok((storey, _)) => self.les_halles_livable_full_storey = Some(storey),
							Err(err) => {
								bevy::log::error!(
									"les-halles-livable-full-storey fill failed: {err}"
								);
							}
						}
					}
					Err(err) => {
						bevy::log::error!("les-halles-livable-full-storey fit failed: {err}");
					}
				}
			}
			PreviewSubject::LesHallesLivableFullStoreyExamples => {
				self.les_halles_livable_full_storey_examples =
					build_les_halles_livable_full_storey_examples();
			}
			PreviewSubject::MixedUseLesHallesMonotower { extent, seed, openings } => {
				match fit_mixed_use_les_halles_monotower(*extent, *seed, openings) {
					Ok(tower) => self.mixed_use_les_halles_monotower = Some(tower),
					Err(err) => {
						bevy::log::error!("mixed-use-les-halles-monotower fit failed: {err}");
					}
				}
			}
			PreviewSubject::MixedUseLesHallesDevelopment { extent, seed, openings } => {
				match fit_mixed_use_les_halles_development(*extent, *seed, openings) {
					Ok(dev) => self.mixed_use_les_halles_development = Some(dev),
					Err(err) => {
						bevy::log::error!("mixed-use-les-halles-development fit failed: {err}");
					}
				}
			}
			PreviewSubject::IApartmentFloorPlan { extent, seed, ceiling, openings } => {
				match fit_i_apartment_floor_plan(*extent, *seed, *ceiling, openings) {
					Ok(plan) => self.i_apartment_floor_plan = Some(plan),
					Err(err) => {
						bevy::log::error!("i-apartment-floor-plan fit failed: {err}");
					}
				}
			}
			PreviewSubject::IApartmentFloorPlanExamples => {
				self.i_apartment_floor_plan_examples = build_i_apartment_floor_plan_examples();
			}
			PreviewSubject::IApartmentFullStoreyExamples => {
				self.i_apartment_full_storey_examples = build_i_apartment_full_storey_examples();
			}
			PreviewSubject::LivableApartmentsExamples => {
				self.livable_apartments_examples = build_livable_apartments_examples();
			}
			PreviewSubject::LivableApartmentExamples => {
				self.livable_apartment_examples = build_livable_apartment_examples();
			}
			PreviewSubject::LivableRectanglesExamples => {
				self.livable_rectangles_examples = build_livable_rectangles_examples();
			}
			PreviewSubject::IApartmentFullStorey { extent, seed, ceiling, openings } => {
				match fit_i_apartment_floor_plan(*extent, *seed, *ceiling, openings) {
					Ok(plan) => {
						let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
						match IApartmentFullStorey::from_floor_plan(plan, noise) {
							Ok((storey, _)) => self.i_apartment_full_storey = Some(storey),
							Err(err) => {
								bevy::log::error!("i-apartment-full-storey fill failed: {err}");
							}
						}
					}
					Err(err) => {
						bevy::log::error!("i-apartment-full-storey fit failed: {err}");
					}
				}
			}
			PreviewSubject::HallsToShafts { extent, seed, hall_width, openings } => {
				match fit_halls_to_shafts(*extent, *seed, *hall_width, openings) {
					Ok(preview) => self.halls_to_shafts = Some(preview),
					Err(err) => {
						bevy::log::error!("halls-to-shafts fit failed: {err}");
					}
				}
			}
			_ => {}
		}
	}

	fn label_nodes(&self) -> Vec<LabelNode> {
		use lod::scene::LodSceneLevel;
		if let Some(stall) = self.commercial_stall.as_ref() {
			return stall.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(strip) = self.commercial_stall_strip.as_ref() {
			return strip.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(stall) = self.bites_stall.as_ref() {
			return stall.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(stall) = self.bites_sitdown_stall.as_ref() {
			return stall.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(stall) = self.mini_mart.as_ref() {
			return stall.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(stall) = self.parts_stall.as_ref() {
			return stall.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(stall) = self.knick_knack_stall.as_ref() {
			return stall.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(stall) = self.public_restroom.as_ref() {
			return stall.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(room) = self.bedroom.as_ref() {
			return room.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(room) = self.residential_bathroom.as_ref() {
			return room.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(room) = self.residential_half_bathroom.as_ref() {
			return room.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if !self.bedroom_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.bedroom_examples {
				out.extend(
					cell.room.label_nodes_for_level(LodSceneLevel::High).flatten().into_iter().map(
						|mut label| {
							label.placement.translation += cell.offset;
							label
						},
					),
				);
			}
			return out;
		}
		if !self.residential_bathroom_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.residential_bathroom_examples {
				let (offset, labels) = match cell {
					ResidentialBathroomExampleCell::Full { offset, room } => {
						(*offset, room.label_nodes_for_level(LodSceneLevel::High).flatten())
					}
					ResidentialBathroomExampleCell::Half { offset, room } => {
						(*offset, room.label_nodes_for_level(LodSceneLevel::High).flatten())
					}
				};
				out.extend(labels.into_iter().map(|mut label| {
					label.placement.translation += offset;
					label
				}));
			}
			return out;
		}
		if let Some(labels) = gallery_example_labels(&self.kitchen_examples) {
			return labels;
		}
		if let Some(labels) = gallery_example_labels(&self.dining_room_examples) {
			return labels;
		}
		if let Some(labels) = gallery_example_labels(&self.living_room_examples) {
			return labels;
		}
		if let Some(labels) = gallery_example_labels(&self.sitting_room_examples) {
			return labels;
		}
		if let Some(labels) = gallery_example_labels(&self.study_examples) {
			return labels;
		}
		if !self.bites_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.bites_examples {
				let (offset, labels) = match cell {
					BitesExampleCell::Stall { offset, stall } => {
						(*offset, stall.label_nodes_for_level(LodSceneLevel::High).flatten())
					}
					BitesExampleCell::Sitdown { offset, stall } => {
						(*offset, stall.label_nodes_for_level(LodSceneLevel::High).flatten())
					}
				};
				out.extend(labels.into_iter().map(|mut label| {
					label.placement.translation += offset;
					label
				}));
			}
			return out;
		}
		if !self.mini_mart_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.mini_mart_examples {
				out.extend(
					cell.stall
						.label_nodes_for_level(LodSceneLevel::High)
						.flatten()
						.into_iter()
						.map(|mut label| {
							label.placement.translation += cell.offset;
							label
						}),
				);
			}
			return out;
		}
		if !self.parts_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.parts_examples {
				out.extend(
					cell.stall
						.label_nodes_for_level(LodSceneLevel::High)
						.flatten()
						.into_iter()
						.map(|mut label| {
							label.placement.translation += cell.offset;
							label
						}),
				);
			}
			return out;
		}
		if !self.knick_knack_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.knick_knack_examples {
				out.extend(
					cell.stall
						.label_nodes_for_level(LodSceneLevel::High)
						.flatten()
						.into_iter()
						.map(|mut label| {
							label.placement.translation += cell.offset;
							label
						}),
				);
			}
			return out;
		}
		if !self.public_restroom_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.public_restroom_examples {
				out.extend(
					cell.stall
						.label_nodes_for_level(LodSceneLevel::High)
						.flatten()
						.into_iter()
						.map(|mut label| {
							label.placement.translation += cell.offset;
							label
						}),
				);
			}
			return out;
		}
		if let Some(plan) = self.les_halles_floor_plan.as_ref() {
			return plan.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if !self.les_halles_floor_plan_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.les_halles_floor_plan_examples {
				out.extend(
					cell.plan.label_nodes_for_level(LodSceneLevel::High).flatten().into_iter().map(
						|mut label| {
							label.placement.translation += cell.offset;
							label
						},
					),
				);
			}
			return out;
		}
		if let Some(storey) = self.les_halles_full_storey.as_ref() {
			return storey.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(storey) = self.les_halles_livable_full_storey.as_ref() {
			return storey.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if let Some(tower) = self.mixed_use_les_halles_monotower.as_ref() {
			return tower.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if !self.les_halles_livable_full_storey_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.les_halles_livable_full_storey_examples {
				out.extend(
					cell.storey
						.label_nodes_for_level(LodSceneLevel::High)
						.flatten()
						.into_iter()
						.map(|mut label| {
							label.placement.translation += cell.offset;
							label
						}),
				);
			}
			return out;
		}
		if !self.i_apartment_floor_plan_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.i_apartment_floor_plan_examples {
				out.extend(
					cell.plan.label_nodes_for_level(LodSceneLevel::High).flatten().into_iter().map(
						|mut label| {
							label.placement.translation += cell.offset;
							label
						},
					),
				);
			}
			return out;
		}
		if let Some(plan) = self.i_apartment_floor_plan.as_ref() {
			return plan.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if !self.i_apartment_full_storey_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.i_apartment_full_storey_examples {
				out.extend(
					cell.storey
						.label_nodes_for_level(LodSceneLevel::High)
						.flatten()
						.into_iter()
						.map(|mut label| {
							label.placement.translation += cell.offset;
							label
						}),
				);
			}
			return out;
		}
		if let Some(storey) = self.i_apartment_full_storey.as_ref() {
			return storey.label_nodes_for_level(LodSceneLevel::High).flatten();
		}
		if !self.livable_apartments_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.livable_apartments_examples {
				out.extend(
					cell.block
						.label_nodes_for_level(LodSceneLevel::High)
						.flatten()
						.into_iter()
						.map(|mut label| {
							label.placement.translation += cell.offset;
							label
						}),
				);
			}
			return out;
		}
		if !self.livable_apartment_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.livable_apartment_examples {
				out.extend(
					cell.apartment
						.label_nodes_for_level(LodSceneLevel::High)
						.flatten()
						.into_iter()
						.map(|mut label| {
							label.placement.translation += cell.offset;
							label
						}),
				);
			}
			return out;
		}
		if !self.livable_rectangles_examples.is_empty() {
			let mut out = Vec::new();
			for cell in &self.livable_rectangles_examples {
				out.extend(
					cell.area.label_nodes_for_level(LodSceneLevel::High).flatten().into_iter().map(
						|mut label| {
							label.placement.translation += cell.offset;
							label
						},
					),
				);
			}
			return out;
		}
		Vec::new()
	}
}

fn gallery_example_labels<T: BuildingComponents>(
	cells: &[GalleryExampleCell<T>],
) -> Option<Vec<LabelNode>> {
	use lod::scene::LodSceneLevel;
	if cells.is_empty() {
		return None;
	}
	let mut out = Vec::new();
	for cell in cells {
		out.extend(cell.room.label_nodes_for_level(LodSceneLevel::High).flatten().into_iter().map(
			|mut label| {
				label.placement.translation += cell.offset;
				label
			},
		));
	}
	Some(out)
}

fn passage_aabbs_at(confines: &Confines, offset: Vec3) -> Vec<(Aabb3d, Vec3)> {
	confines
		.openings
		.iter()
		.filter(|(_, o)| matches!(o.label, OpeningLabel::Passage))
		.map(|(_, o)| (o.bounds, offset))
		.collect()
}

const STALL_GALLERY_GAP: f32 = 2.5;

/// World offset for cell `index` in a row-major gallery of `cols` columns.
fn gallery_grid_offset(
	extent_at: impl Fn(usize) -> Vec3,
	len: usize,
	index: usize,
	cols: usize,
	gap: f32,
) -> Vec3 {
	let col = index % cols;
	let row = index / cols;
	let mut x = 0.0;
	for c in 0..col {
		x += extent_at(row * cols + c).x + gap;
	}
	let mut z = 0.0;
	for r in 0..row {
		let mut row_depth = 0.0_f32;
		for c in 0..cols {
			let idx = r * cols + c;
			if idx < len {
				row_depth = row_depth.max(extent_at(idx).z);
			}
		}
		z += row_depth + gap;
	}
	Vec3::new(x, 0.0, z)
}

/// Fit each `(extent, seed, door_side)` cell; collect successes + passage gizmos.
fn build_fit_gallery<T>(
	label: &str,
	specs: &[(Vec3, i32, BitesDoorSide)],
	cols: usize,
	mut fit: impl FnMut(&Confines, NoiseParams) -> Result<T, FitError>,
) -> (Vec<(Vec3, T)>, Vec<(Aabb3d, Vec3)>) {
	let gap = STALL_GALLERY_GAP;
	let mut cells = Vec::new();
	let mut passages = Vec::new();
	for (i, (extent, seed, door_side)) in specs.iter().enumerate() {
		let offset = gallery_grid_offset(|j| specs[j].0, specs.len(), i, cols, gap);
		let confines = demo_bites_stall_confines(*extent, *door_side);
		passages.extend(passage_aabbs_at(&confines, offset));
		let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
		match fit(&confines, noise) {
			Ok(stall) => cells.push((offset, stall)),
			Err(err) => {
				bevy::log::error!("{label} ({extent:?} seed={seed}) failed: {err}")
			}
		}
	}
	(cells, passages)
}

fn bites_examples_specs() -> Vec<(bool, Vec3, i32, BitesDoorSide)> {
	// (sitdown?, extent, seed, door_side)
	vec![
		// Row 0 — BitesStall
		(false, Vec3::new(12.0, 3.2, 8.0), 1337, BitesDoorSide::South),
		(false, Vec3::new(10.0, 3.2, 6.0), 42, BitesDoorSide::South),
		(false, Vec3::new(5.5, 3.2, 9.0), 7, BitesDoorSide::South),
		(false, Vec3::new(6.0, 3.2, 14.0), 99, BitesDoorSide::East),
		(false, Vec3::new(8.0, 3.2, 22.0), 3, BitesDoorSide::South),
		// Row 1 — BitesSitdownStall
		(true, Vec3::new(12.0, 3.2, 8.0), 1337, BitesDoorSide::South),
		(true, Vec3::new(14.0, 3.2, 5.0), 42, BitesDoorSide::South),
		(true, Vec3::new(10.0, 3.2, 22.0), 11, BitesDoorSide::South),
		(true, Vec3::new(6.0, 3.2, 14.0), 55, BitesDoorSide::East),
		(true, Vec3::new(12.0, 3.2, 8.0), 42, BitesDoorSide::North),
	]
}

fn mini_mart_examples_specs() -> Vec<(Vec3, i32, BitesDoorSide)> {
	vec![
		(Vec3::new(14.0, 3.2, 12.0), 11, BitesDoorSide::South),
		(Vec3::new(16.0, 3.2, 10.0), 42, BitesDoorSide::South),
		(Vec3::new(12.0, 3.2, 14.0), 7, BitesDoorSide::South),
		(Vec3::new(10.0, 3.2, 16.0), 99, BitesDoorSide::East),
		(Vec3::new(18.0, 3.2, 12.0), 3, BitesDoorSide::North),
		(Vec3::new(14.0, 3.2, 12.0), 21, BitesDoorSide::South),
	]
}

fn parts_examples_specs() -> Vec<(Vec3, i32, BitesDoorSide)> {
	vec![
		(Vec3::new(10.0, 3.2, 8.0), 3, BitesDoorSide::South),
		(Vec3::new(12.0, 3.2, 7.0), 11, BitesDoorSide::South),
		(Vec3::new(8.0, 3.2, 10.0), 42, BitesDoorSide::East),
		(Vec3::new(14.0, 3.2, 8.0), 7, BitesDoorSide::South),
		(Vec3::new(10.0, 3.2, 9.0), 21, BitesDoorSide::North),
		(Vec3::new(11.0, 3.2, 8.0), 55, BitesDoorSide::South),
	]
}

fn build_parts_examples() -> (Vec<PartsExampleCell>, Vec<(Aabb3d, Vec3)>) {
	let (cells, passages) =
		build_fit_gallery("parts-examples", &parts_examples_specs(), 3, |confines, noise| {
			PartsStall::fit_to_confines(confines, noise).map(|(s, _)| s)
		});
	(
		cells
			.into_iter()
			.map(|(offset, stall)| PartsExampleCell { offset, stall })
			.collect(),
		passages,
	)
}

/// `(extent, seed, spaciousness, occupancy, door, bed_against_wall)`.
fn bedroom_examples_specs() -> Vec<(Vec3, i32, f32, f32, bool, bool)> {
	vec![
		// Row 0 — compact cells, roomier default spaciousness
		(Vec3::new(6.5, 3.0, 6.5), 7, 1.2, 0.55, true, true),
		(Vec3::new(7.0, 3.0, 6.0), 11, 1.25, 0.65, true, true),
		(Vec3::new(6.0, 3.0, 7.0), 21, 1.35, 0.5, true, false),
		// Row 1 — mid-size
		(Vec3::new(8.0, 3.0, 8.0), 42, 1.25, 0.55, true, true),
		(Vec3::new(9.0, 3.2, 7.0), 55, 1.3, 0.5, true, false),
		(Vec3::new(7.5, 3.0, 9.0), 99, 1.2, 0.6, false, true),
		// Row 2 — large bedrooms (how fill scales up)
		(Vec3::new(11.0, 3.2, 10.0), 3, 1.35, 0.5, true, true),
		(Vec3::new(12.0, 3.2, 12.0), 17, 1.4, 0.45, true, true),
		(Vec3::new(14.0, 3.2, 11.0), 33, 1.45, 0.55, true, false),
	]
}

fn build_bedroom_examples() -> (Vec<BedroomExampleCell>, Vec<(Aabb3d, Vec3)>) {
	let specs = bedroom_examples_specs();
	let cols = 3;
	let gap = STALL_GALLERY_GAP;
	let mut cells = Vec::new();
	let mut passages = Vec::new();
	for (i, (extent, seed, spaciousness, occupancy, door, bed_against_wall)) in
		specs.iter().enumerate()
	{
		let offset = gallery_grid_offset(|j| specs[j].0, specs.len(), i, cols, gap);
		let confines = demo_common_bedroom_confines(*extent, *door);
		passages.extend(passage_aabbs_at(&confines, offset));
		let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
		let mut params =
			buildings::CommonBedroomParameterized::with_fill(*spaciousness, *occupancy);
		params.bed_against_wall = *bed_against_wall;
		match CommonBedroom::fit_with_fill(&confines, noise, params) {
			Ok((room, _)) => cells.push(BedroomExampleCell { offset, room }),
			Err(err) => {
				bevy::log::error!("bedroom-examples ({extent:?} seed={seed}) failed: {err}")
			}
		}
	}
	(cells, passages)
}

type LivableQuartersExampleSpec = (Vec3, i32, f32, f32, bool);

fn build_livable_quarters_examples<T, P>(
	label: &str,
	specs: &[LivableQuartersExampleSpec],
	cols: usize,
	params: impl Fn(usize, f32, f32) -> P,
	fit: impl Fn(&Confines, NoiseParams, P) -> Result<(T, FillableRegions), FitError>,
) -> (Vec<GalleryExampleCell<T>>, Vec<(Aabb3d, Vec3)>) {
	let gap = STALL_GALLERY_GAP;
	let mut cells = Vec::new();
	let mut passages = Vec::new();
	for (i, (extent, seed, spaciousness, occupancy, door)) in specs.iter().enumerate() {
		let offset = gallery_grid_offset(|j| specs[j].0, specs.len(), i, cols, gap);
		let confines = demo_common_bedroom_confines(*extent, *door);
		passages.extend(passage_aabbs_at(&confines, offset));
		let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
		match fit(&confines, noise, params(i, *spaciousness, *occupancy)) {
			Ok((room, _)) => cells.push(GalleryExampleCell { offset, room }),
			Err(err) => bevy::log::error!("{label} ({extent:?} seed={seed}) failed: {err}"),
		}
	}
	(cells, passages)
}

fn kitchen_examples_specs() -> Vec<LivableQuartersExampleSpec> {
	vec![
		(Vec3::new(4.0, 2.8, 3.0), 7, 1.1, 0.4, true),
		(Vec3::new(5.0, 2.8, 3.5), 11, 1.2, 0.35, true),
		(Vec3::new(4.5, 2.8, 4.0), 21, 1.25, 0.45, true),
		(Vec3::new(6.0, 3.0, 4.0), 42, 1.3, 0.38, true),
		(Vec3::new(5.5, 3.0, 5.0), 55, 1.35, 0.42, true),
		(Vec3::new(7.0, 3.0, 5.5), 99, 1.4, 0.35, true),
	]
}

fn build_kitchen_examples() -> (Vec<GalleryExampleCell<Kitchen>>, Vec<(Aabb3d, Vec3)>) {
	use buildings::KitchenCounterLayout;
	// Force a mix of counter subtypes so galleries show galley / L / peninsula.
	let layouts = [
		KitchenCounterLayout::Galley,
		KitchenCounterLayout::LShape,
		KitchenCounterLayout::Peninsula,
		KitchenCounterLayout::LShape,
		KitchenCounterLayout::Peninsula,
		KitchenCounterLayout::Galley,
	];
	build_livable_quarters_examples(
		"kitchen-examples",
		&kitchen_examples_specs(),
		3,
		|i, spaciousness, occupancy| {
			buildings::KitchenParameterized::with_fill(spaciousness, occupancy)
				.with_layout(layouts[i % layouts.len()])
		},
		Kitchen::fit_with_fill,
	)
}

fn dining_room_examples_specs() -> Vec<LivableQuartersExampleSpec> {
	vec![
		// Compact / near-square
		(Vec3::new(4.0, 2.8, 3.0), 7, 1.15, 0.45, true),
		(Vec3::new(5.0, 2.8, 3.5), 11, 1.2, 0.4, true),
		(Vec3::new(4.5, 2.8, 4.5), 21, 1.25, 0.5, true),
		// Longer / thinner halls
		(Vec3::new(8.0, 3.0, 3.2), 42, 1.2, 0.42, true),
		(Vec3::new(9.5, 3.0, 3.0), 55, 1.25, 0.4, true),
		(Vec3::new(11.0, 3.0, 3.4), 99, 1.3, 0.38, true),
	]
}

fn build_dining_room_examples() -> (Vec<GalleryExampleCell<DiningRoom>>, Vec<(Aabb3d, Vec3)>) {
	build_livable_quarters_examples(
		"dining-room-examples",
		&dining_room_examples_specs(),
		3,
		|_, s, o| buildings::DiningRoomParameterized::with_fill(s, o),
		DiningRoom::fit_with_fill,
	)
}

fn living_room_examples_specs() -> Vec<LivableQuartersExampleSpec> {
	vec![
		(Vec3::new(5.0, 2.8, 4.0), 7, 1.1, 0.35, true),
		(Vec3::new(6.0, 2.8, 4.5), 11, 1.2, 0.4, true),
		(Vec3::new(5.5, 2.8, 5.0), 21, 1.25, 0.38, true),
		(Vec3::new(7.0, 3.0, 5.0), 42, 1.3, 0.42, true),
		(Vec3::new(8.0, 3.0, 6.0), 55, 1.35, 0.35, true),
		(Vec3::new(9.0, 3.0, 7.0), 99, 1.4, 0.4, true),
	]
}

fn build_living_room_examples() -> (Vec<GalleryExampleCell<LivingRoom>>, Vec<(Aabb3d, Vec3)>) {
	build_livable_quarters_examples(
		"living-room-examples",
		&living_room_examples_specs(),
		3,
		|_, s, o| buildings::LivingRoomParameterized::with_fill(s, o),
		LivingRoom::fit_with_fill,
	)
}

fn sitting_room_examples_specs() -> Vec<LivableQuartersExampleSpec> {
	vec![
		(Vec3::new(4.0, 2.8, 3.5), 7, 1.1, 0.4, true),
		(Vec3::new(5.0, 2.8, 4.0), 11, 1.2, 0.38, true),
		(Vec3::new(4.5, 2.8, 4.5), 21, 1.25, 0.42, true),
		(Vec3::new(6.0, 3.0, 4.5), 42, 1.3, 0.35, true),
		(Vec3::new(5.5, 3.0, 5.0), 55, 1.35, 0.4, true),
		(Vec3::new(7.0, 3.0, 5.5), 99, 1.4, 0.38, true),
	]
}

fn build_sitting_room_examples() -> (Vec<GalleryExampleCell<SittingRoom>>, Vec<(Aabb3d, Vec3)>) {
	build_livable_quarters_examples(
		"sitting-room-examples",
		&sitting_room_examples_specs(),
		3,
		|_, s, o| buildings::SittingRoomParameterized::with_fill(s, o),
		SittingRoom::fit_with_fill,
	)
}

fn study_examples_specs() -> Vec<LivableQuartersExampleSpec> {
	vec![
		(Vec3::new(4.0, 2.8, 3.0), 7, 1.1, 0.42, true),
		(Vec3::new(4.5, 2.8, 3.5), 11, 1.2, 0.38, true),
		(Vec3::new(5.0, 2.8, 4.0), 21, 1.25, 0.45, true),
		(Vec3::new(5.5, 3.0, 4.5), 42, 1.3, 0.4, true),
		(Vec3::new(6.0, 3.0, 5.0), 55, 1.35, 0.35, true),
		(Vec3::new(7.0, 3.0, 5.5), 99, 1.4, 0.42, true),
	]
}

fn build_study_examples() -> (Vec<GalleryExampleCell<Study>>, Vec<(Aabb3d, Vec3)>) {
	build_livable_quarters_examples(
		"study-examples",
		&study_examples_specs(),
		3,
		|_, s, o| buildings::StudyParameterized::with_fill(s, o),
		Study::fit_with_fill,
	)
}

/// `(half?, extent, seed, door)`.
fn residential_bathroom_examples_specs() -> Vec<(bool, Vec3, i32, bool)> {
	vec![
		// Row 0 — full bathrooms
		(false, Vec3::new(3.0, 2.8, 2.2), 3, true),
		(false, Vec3::new(3.5, 2.8, 2.5), 11, true),
		(false, Vec3::new(3.2, 2.8, 2.8), 21, true),
		// Row 1 — half bathrooms
		(true, Vec3::new(1.8, 2.8, 1.5), 7, true),
		(true, Vec3::new(2.0, 2.8, 1.6), 42, true),
		(true, Vec3::new(1.9, 2.8, 1.7), 55, true),
	]
}

fn build_residential_bathroom_examples(
) -> (Vec<ResidentialBathroomExampleCell>, Vec<(Aabb3d, Vec3)>) {
	let gap = STALL_GALLERY_GAP;
	let specs = residential_bathroom_examples_specs();
	let cols = 3usize;
	let mut cells = Vec::new();
	let mut passages = Vec::new();
	for (i, (half, extent, seed, door)) in specs.iter().enumerate() {
		let offset = gallery_grid_offset(|j| specs[j].1, specs.len(), i, cols, gap);
		let confines = demo_common_bedroom_confines(*extent, *door);
		passages.extend(passage_aabbs_at(&confines, offset));
		let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
		if *half {
			match ResidentialHalfBathroom::fit_to_confines(&confines, noise) {
				Ok((room, _)) => cells.push(ResidentialBathroomExampleCell::Half { offset, room }),
				Err(err) => bevy::log::error!(
					"residential-bathroom-examples half ({extent:?} seed={seed}) failed: {err}"
				),
			}
		} else {
			match ResidentialBathroom::fit_to_confines(&confines, noise) {
				Ok((room, _)) => cells.push(ResidentialBathroomExampleCell::Full { offset, room }),
				Err(err) => bevy::log::error!(
					"residential-bathroom-examples full ({extent:?} seed={seed}) failed: {err}"
				),
			}
		}
	}
	(cells, passages)
}

fn knick_knack_examples_specs() -> Vec<(Vec3, i32, BitesDoorSide)> {
	vec![
		(Vec3::new(10.0, 3.2, 8.0), 3, BitesDoorSide::South),
		(Vec3::new(12.0, 3.2, 7.0), 11, BitesDoorSide::South),
		(Vec3::new(8.0, 3.2, 10.0), 42, BitesDoorSide::East),
		(Vec3::new(14.0, 3.2, 8.0), 7, BitesDoorSide::South),
		(Vec3::new(10.0, 3.2, 9.0), 21, BitesDoorSide::North),
		(Vec3::new(11.0, 3.2, 8.0), 55, BitesDoorSide::West),
	]
}

fn build_knick_knack_examples() -> (Vec<KnickKnackExampleCell>, Vec<(Aabb3d, Vec3)>) {
	let (cells, passages) = build_fit_gallery(
		"knick-knack-examples",
		&knick_knack_examples_specs(),
		3,
		|confines, noise| KnickKnackStall::fit_to_confines(confines, noise).map(|(s, _)| s),
	);
	(
		cells
			.into_iter()
			.map(|(offset, stall)| KnickKnackExampleCell { offset, stall })
			.collect(),
		passages,
	)
}

fn public_restroom_examples_specs() -> Vec<(Vec3, i32, BitesDoorSide)> {
	vec![
		(Vec3::new(10.0, 3.2, 8.0), 3, BitesDoorSide::South),
		(Vec3::new(12.0, 3.2, 7.0), 11, BitesDoorSide::South),
		(Vec3::new(8.0, 3.2, 10.0), 42, BitesDoorSide::East),
		(Vec3::new(14.0, 3.2, 8.0), 7, BitesDoorSide::South),
		(Vec3::new(10.0, 3.2, 9.0), 21, BitesDoorSide::North),
		(Vec3::new(11.0, 3.2, 8.0), 55, BitesDoorSide::West),
	]
}

fn build_public_restroom_examples() -> (Vec<PublicRestroomExampleCell>, Vec<(Aabb3d, Vec3)>) {
	let (cells, passages) = build_fit_gallery(
		"public-restroom-examples",
		&public_restroom_examples_specs(),
		3,
		|confines, noise| PublicRestroom::fit_to_confines(confines, noise).map(|(s, _)| s),
	);
	(
		cells
			.into_iter()
			.map(|(offset, stall)| PublicRestroomExampleCell { offset, stall })
			.collect(),
		passages,
	)
}

fn build_mini_mart_examples() -> (Vec<MiniMartExampleCell>, Vec<(Aabb3d, Vec3)>) {
	let (cells, passages) = build_fit_gallery(
		"mini-mart-examples",
		&mini_mart_examples_specs(),
		3,
		|confines, noise| MiniMart::fit_to_confines(confines, noise).map(|(s, _)| s),
	);
	(
		cells
			.into_iter()
			.map(|(offset, stall)| MiniMartExampleCell { offset, stall })
			.collect(),
		passages,
	)
}

fn build_bites_examples() -> (Vec<BitesExampleCell>, Vec<(Aabb3d, Vec3)>) {
	let gap = STALL_GALLERY_GAP;
	let specs = bites_examples_specs();
	let cols = 5usize;
	let mut cells = Vec::new();
	let mut passages = Vec::new();
	for (i, (sitdown, extent, seed, door_side)) in specs.iter().enumerate() {
		let offset = gallery_grid_offset(|j| specs[j].1, specs.len(), i, cols, gap);
		let confines = demo_bites_stall_confines(*extent, *door_side);
		passages.extend(passage_aabbs_at(&confines, offset));
		let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
		if *sitdown {
			match BitesSitdownStall::fit_to_confines(&confines, noise) {
				Ok((stall, _)) => cells.push(BitesExampleCell::Sitdown { offset, stall }),
				Err(err) => bevy::log::error!(
					"bites-examples sitdown ({extent:?} seed={seed}) failed: {err}"
				),
			}
		} else {
			match BitesStall::fit_to_confines(&confines, noise) {
				Ok((stall, _)) => cells.push(BitesExampleCell::Stall { offset, stall }),
				Err(err) => {
					bevy::log::error!("bites-examples stall ({extent:?} seed={seed}) failed: {err}")
				}
			}
		}
	}
	(cells, passages)
}

fn les_halles_confines_bounds(extent: Vec3) -> Aabb3d {
	let hx = extent.x.max(1e-4) * 0.5;
	let hz = extent.z.max(1e-4) * 0.5;
	let h = extent.y.max(1e-4);
	Aabb3d::from_min_max(Vec3::new(-hx, 0.0, -hz), Vec3::new(hx, h, hz))
}

/// Demo common bedroom confines; optional south (−Z) passage for entry clearance.
fn demo_common_bedroom_confines(extent: Vec3, door: bool) -> Confines {
	let extent = extent.max(Vec3::splat(1e-4));
	let mut openings = Openings::new();
	if door {
		let door_w = (extent.x * 0.3).clamp(0.8, 1.2);
		let cx = extent.x * 0.5;
		let door_h = (extent.y * 0.72).clamp(2.0, extent.y.max(2.0));
		openings.insert(
			OpeningId::new("demo_bedroom_door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(cx - door_w * 0.5, 0.0, -0.2),
				Vec3::new(cx + door_w * 0.5, door_h, 0.2),
			)),
		);
	}
	Confines::new(Aabb3d::from_min_max(Vec3::ZERO, extent), 0.0, openings)
}

/// Demo bites stall with long Passage(s) on the chosen façade.
fn demo_bites_stall_confines(extent: Vec3, door_side: BitesDoorSide) -> Confines {
	let extent = extent.max(Vec3::splat(1e-4));
	let door_h = (extent.y * 0.72).clamp(2.0, extent.y.max(2.0));
	let mut openings = Openings::new();
	let along = match door_side {
		BitesDoorSide::South | BitesDoorSide::North => extent.x,
		BitesDoorSide::East | BitesDoorSide::West => extent.z,
	};
	let band = 0.25_f32;
	let mk = |a0: f32, a1: f32| -> Aabb3d {
		match door_side {
			BitesDoorSide::South => {
				Aabb3d::from_min_max(Vec3::new(a0, 0.0, -band), Vec3::new(a1, door_h, band))
			}
			BitesDoorSide::North => Aabb3d::from_min_max(
				Vec3::new(a0, 0.0, extent.z - band),
				Vec3::new(a1, door_h, extent.z + band),
			),
			BitesDoorSide::East => Aabb3d::from_min_max(
				Vec3::new(extent.x - band, 0.0, a0),
				Vec3::new(extent.x + band, door_h, a1),
			),
			BitesDoorSide::West => {
				Aabb3d::from_min_max(Vec3::new(-band, 0.0, a0), Vec3::new(band, door_h, a1))
			}
		}
	};
	if along >= 6.0 {
		openings.insert(
			OpeningId::new("demo_bites_door_a"),
			Opening::passage(mk(0.4, (along * 0.42).max(2.5))),
		);
		openings.insert(
			OpeningId::new("demo_bites_door_b"),
			Opening::passage(mk(along * 0.58, (along - 0.4).max(along * 0.58 + 2.5))),
		);
	} else {
		openings.insert(
			OpeningId::new("demo_bites_door"),
			Opening::passage(mk(0.3, (along - 0.3).max(2.2))),
		);
	}
	Confines::new(Aabb3d::from_min_max(Vec3::ZERO, extent), 0.0, openings)
}

/// Demo strip confines with one Passage per ~preferred bay on the −Z façade.
fn demo_commercial_stall_strip_confines(extent: Vec3, seed: i32) -> Confines {
	let extent = extent.max(Vec3::splat(1e-4));
	let along_x = extent.x >= extent.z;
	let along = if along_x { extent.x } else { extent.z };
	let bay = (4.5 + ((seed.rem_euclid(17) as f32) * 0.12)).clamp(3.5, 8.0);
	let n = ((along / bay).floor() as usize).max(1);
	let cell = along / n as f32;
	let mut openings = Openings::new();
	for i in 0..n {
		let mid = (i as f32 + 0.5) * cell;
		let half_w = (1.1_f32).min(cell * 0.35);
		let door_h = (extent.y * 0.72).clamp(2.0, extent.y.max(2.0));
		let bounds = if along_x {
			Aabb3d::from_min_max(
				Vec3::new(mid - half_w, 0.0, -0.25),
				Vec3::new(mid + half_w, door_h, 0.25),
			)
		} else {
			Aabb3d::from_min_max(
				Vec3::new(-0.25, 0.0, mid - half_w),
				Vec3::new(0.25, door_h, mid + half_w),
			)
		};
		openings.insert(OpeningId::new(format!("demo_door_{i}")), Opening::passage(bounds));
	}
	Confines::new(Aabb3d::from_min_max(Vec3::ZERO, extent), 0.0, openings)
}

fn fit_mixed_use_les_halles_monotower(
	extent: Vec3,
	seed: i32,
	openings: &[PreviewOpening],
) -> Result<MixedUseLesHallesMonotower, buildings::FitError> {
	let bounds = les_halles_confines_bounds(extent);
	let inbound =
		if openings.is_empty() { Openings::new() } else { openings_from_preview(openings) };
	let confines = Confines::new(bounds, 0.0, inbound);
	let noise = NoiseParams { seed, ..NoiseParams::default() };
	MixedUseLesHallesMonotower::fit_to_confines(&confines, noise).map(|(tower, _)| tower)
}

fn fit_mixed_use_les_halles_development(
	extent: Vec3,
	seed: i32,
	openings: &[PreviewOpening],
) -> Result<MixedUseLesHallesDevelopment, buildings::FitError> {
	let bounds = les_halles_confines_bounds(extent);
	let inbound =
		if openings.is_empty() { Openings::new() } else { openings_from_preview(openings) };
	let confines = Confines::new(bounds, 0.0, inbound);
	let noise = NoiseParams { seed, ..NoiseParams::default() };
	MixedUseLesHallesDevelopment::fit_to_confines(&confines, noise).map(|(dev, _)| dev)
}

fn fit_les_halles_floor_plan(
	extent: Vec3,
	seed: i32,
	ceiling: bool,
	openings: &[PreviewOpening],
) -> Result<LesHallesFloorPlan, buildings::FitError> {
	fit_les_halles_floor_plan_sampled(extent, seed, ceiling, openings, false, None)
}

fn fit_les_halles_floor_plan_sampled(
	extent: Vec3,
	seed: i32,
	ceiling: bool,
	openings: &[PreviewOpening],
	livable: bool,
	force_placement: Option<LesHallesShaftPlacement>,
) -> Result<LesHallesFloorPlan, buildings::FitError> {
	let bounds = les_halles_confines_bounds(extent);
	let empty = Confines::from_bounds(bounds);
	let noise = NoiseParams { seed, ..NoiseParams::default() };
	let mut params = if livable {
		LesHallesParameterized::sample_livable(&empty, noise)?
	} else {
		LesHallesParameterized::sample(&empty, noise)?
	};
	if let Some(placement) = force_placement {
		params.shaft_placement = placement;
	}
	let inbound = if openings.is_empty() {
		// Demo default: request all placement slots so shafts remain visible.
		LesHallesFloorPlan::shaft_requests_for_all_slots(&params, &empty)
	} else {
		openings_from_preview(openings)
	};
	let confines = Confines::new(bounds, 0.0, inbound);
	let ceiling = if ceiling { RectRingFloorSlab::Solid } else { RectRingFloorSlab::None };
	LesHallesFloorPlan::from_parameterized_with_ceiling(params, &confines, ceiling)
		.map(|(plan, _)| plan)
}

const LES_HALLES_FLOOR_PLAN_GALLERY_COLS: usize = 3;
const LES_HALLES_FLOOR_PLAN_GALLERY_GAP: f32 = 14.0;

/// `(extent, seed, livable_sample, forced shaft placement)`.
fn les_halles_floor_plan_examples_specs() -> Vec<(Vec3, i32, bool, Option<LesHallesShaftPlacement>)>
{
	vec![
		// Commercial sampling, natural shaft placement.
		(Vec3::new(48.0, 4.0, 36.0), 1337, false, None),
		(Vec3::new(48.0, 4.0, 36.0), 7, false, None),
		(Vec3::new(56.0, 4.0, 40.0), 42, false, None),
		(Vec3::new(40.0, 4.0, 40.0), 3, false, None),
		// Same footprint, forced corner vs mid-side (strip topology contrast).
		(Vec3::new(72.0, 4.0, 54.0), 11, true, Some(LesHallesShaftPlacement::Corners)),
		(Vec3::new(72.0, 4.0, 54.0), 11, true, Some(LesHallesShaftPlacement::MidSides)),
		(Vec3::new(64.0, 4.0, 64.0), 19, true, Some(LesHallesShaftPlacement::Corners)),
		(Vec3::new(64.0, 4.0, 64.0), 19, true, Some(LesHallesShaftPlacement::MidSides)),
		// Extra livable natural samples.
		(Vec3::new(80.0, 4.0, 60.0), 5, true, None),
		(Vec3::new(88.0, 4.0, 56.0), 23, true, None),
		(Vec3::new(72.0, 4.0, 48.0), 31, true, None),
		(Vec3::new(60.0, 4.0, 60.0), 17, true, None),
	]
}

fn build_les_halles_floor_plan_examples() -> Vec<LesHallesFloorPlanExampleCell> {
	let specs = les_halles_floor_plan_examples_specs();
	let mut fitted = Vec::new();
	for (extent, seed, livable, placement) in &specs {
		match fit_les_halles_floor_plan_sampled(*extent, *seed, false, &[], *livable, *placement) {
			Ok(plan) => fitted.push(plan),
			Err(err) => {
				bevy::log::error!(
					"les-halles-floor-plan-examples ({extent:?} seed={seed} livable={livable} placement={placement:?}) failed: {err}"
				);
			}
		}
	}
	let layout: Vec<(Vec3, Vec3)> = fitted.iter().map(les_halles_plan_footprint_aabb).collect();
	fitted
		.into_iter()
		.enumerate()
		.map(|(i, plan)| {
			let (local_min, _) = layout[i];
			let cell_origin = gallery_grid_offset(
				|j| layout[j].1,
				layout.len(),
				i,
				LES_HALLES_FLOOR_PLAN_GALLERY_COLS,
				LES_HALLES_FLOOR_PLAN_GALLERY_GAP,
			);
			let offset = cell_origin - local_min;
			LesHallesFloorPlanExampleCell { offset, plan }
		})
		.collect()
}

fn fit_les_halles_livable_floor_plan(
	extent: Vec3,
	seed: i32,
	ceiling: bool,
	openings: &[PreviewOpening],
) -> Result<LesHallesFloorPlan, buildings::FitError> {
	let bounds = les_halles_confines_bounds(extent);
	let empty = Confines::from_bounds(bounds);
	let noise = NoiseParams { seed, ..NoiseParams::default() };
	let params = LesHallesParameterized::sample_livable(&empty, noise)?;
	let inbound = if openings.is_empty() {
		LesHallesFloorPlan::shaft_requests_for_all_slots(&params, &empty)
	} else {
		openings_from_preview(openings)
	};
	let confines = Confines::new(bounds, 0.0, inbound);
	let ceiling = if ceiling { RectRingFloorSlab::Solid } else { RectRingFloorSlab::None };
	LesHallesFloorPlan::from_parameterized_with_ceiling(params, &confines, ceiling)
		.map(|(plan, _)| plan)
}

const LES_HALLES_LIVABLE_GALLERY_COLS: usize = 3;
const LES_HALLES_LIVABLE_GALLERY_GAP: f32 = 16.0;

fn les_halles_livable_full_storey_examples_specs() -> Vec<(Vec3, i32)> {
	vec![
		(Vec3::new(72.0, 4.0, 54.0), 1337),
		(Vec3::new(72.0, 4.0, 54.0), 7),
		(Vec3::new(80.0, 4.0, 60.0), 42),
		(Vec3::new(64.0, 4.0, 64.0), 3),
		(Vec3::new(88.0, 4.0, 56.0), 19),
		(Vec3::new(72.0, 4.0, 48.0), 11),
	]
}

fn les_halles_plan_footprint_aabb(plan: &LesHallesFloorPlan) -> (Vec3, Vec3) {
	let hx = plan.outer.x * 0.5;
	let hz = plan.outer.y * 0.5;
	let min = Vec3::new(plan.center_xz.x - hx, 0.0, plan.center_xz.z - hz);
	let extent =
		Vec3::new(plan.outer.x.max(1.0), plan.storey_height.max(1.0), plan.outer.y.max(1.0));
	(min, extent)
}

fn build_les_halles_livable_full_storey_examples() -> Vec<LesHallesLivableFullStoreyExampleCell> {
	let specs = les_halles_livable_full_storey_examples_specs();
	let mut fitted = Vec::new();
	for (extent, seed) in &specs {
		match fit_les_halles_livable_floor_plan(*extent, *seed, false, &[]) {
			Ok(plan) => {
				let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
				match LesHallesLivableFullStorey::from_floor_plan(plan, noise) {
					Ok((storey, _)) => fitted.push(storey),
					Err(err) => {
						bevy::log::error!(
							"les-halles-livable-full-storey-examples ({extent:?} seed={seed}) fill failed: {err}"
						);
					}
				}
			}
			Err(err) => {
				bevy::log::error!(
					"les-halles-livable-full-storey-examples ({extent:?} seed={seed}) fit failed: {err}"
				);
			}
		}
	}
	let layout: Vec<(Vec3, Vec3)> =
		fitted.iter().map(|s| les_halles_plan_footprint_aabb(&s.floor_plan)).collect();
	fitted
		.into_iter()
		.enumerate()
		.map(|(i, storey)| {
			let (local_min, _) = layout[i];
			let cell_origin = gallery_grid_offset(
				|j| layout[j].1,
				layout.len(),
				i,
				LES_HALLES_LIVABLE_GALLERY_COLS,
				LES_HALLES_LIVABLE_GALLERY_GAP,
			);
			let offset = cell_origin - local_min;
			LesHallesLivableFullStoreyExampleCell { offset, storey }
		})
		.collect()
}

fn fit_i_apartment_floor_plan(
	extent: Vec3,
	seed: i32,
	ceiling: bool,
	openings: &[PreviewOpening],
) -> Result<IApartmentFloorPlan, buildings::FitError> {
	use buildings::IFloorSlab;
	let bounds = les_halles_confines_bounds(extent);
	let empty = Confines::from_bounds(bounds);
	let noise = NoiseParams { seed, ..NoiseParams::default() };
	let params = IApartmentParameterized::sample(&empty, noise)?;
	let inbound = if openings.is_empty() {
		// Demo default: one shaft request per primary rect (9-pocket mapped).
		IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty)
	} else {
		openings_from_preview(openings)
	};
	let confines = Confines::new(bounds, 0.0, inbound);
	let ceiling = if ceiling { IFloorSlab::Solid } else { IFloorSlab::None };
	IApartmentFloorPlan::from_parameterized_with_ceiling(params, &confines, ceiling)
		.map(|(plan, _)| plan)
}

fn fit_halls_to_shafts(
	extent: Vec3,
	seed: i32,
	hall_width: Option<f32>,
	openings: &[PreviewOpening],
) -> Result<HallsToShaftsPreview, buildings::FitError> {
	let host = les_halles_confines_bounds(extent);
	let inbound = if openings.is_empty() {
		openings_from_preview(&crate::commands::show::halls_to_shafts::default_demo_openings(
			extent,
		))
	} else {
		openings_from_preview(openings)
	};
	let confines = Confines::new(host, 0.0, inbound);
	let noise = NoiseParams { seed, ..NoiseParams::default() };
	let (fit, regions) = HallsToShaftsFit::from_confines_with(
		&confines,
		noise,
		HallsToShaftsOptions { hall_width },
	)?;
	Ok(HallsToShaftsPreview { fit, regions, host })
}

/// Curated (extent, seed) cells: same Fit path as production; seed/aspect vary I / T / L.
fn i_apartment_floor_plan_examples_specs() -> Vec<(Vec3, i32)> {
	vec![
		(Vec3::new(44.0, 3.5, 36.0), 3),  // I
		(Vec3::new(44.0, 3.5, 36.0), 0),  // I
		(Vec3::new(44.0, 3.5, 36.0), 4),  // I
		(Vec3::new(44.0, 3.5, 36.0), 9),  // I
		(Vec3::new(36.0, 3.5, 44.0), 1),  // I, tall aspect
		(Vec3::new(52.0, 3.5, 32.0), 2),  // I, wide
		(Vec3::new(44.0, 3.5, 36.0), 18), // T
		(Vec3::new(44.0, 3.5, 36.0), 19), // L
		(Vec3::new(40.0, 3.5, 40.0), 28), // L, square
	]
}

const I_APARTMENT_GALLERY_COLS: usize = 3;
const I_APARTMENT_GALLERY_GAP: f32 = 14.0;

/// Plan-space AABB of primary rects (`min_x/z` may be negative / asymmetric).
fn i_apartment_plan_footprint_aabb(plan: &IApartmentFloorPlan) -> (Vec3, Vec3) {
	let mut min_x = f32::INFINITY;
	let mut max_x = f32::NEG_INFINITY;
	let mut min_z = f32::INFINITY;
	let mut max_z = f32::NEG_INFINITY;
	for rect in &plan.primary_rects {
		min_x = min_x.min(rect.min_x);
		max_x = max_x.max(rect.max_x);
		min_z = min_z.min(rect.min_z);
		max_z = max_z.max(rect.max_z);
	}
	let extent =
		Vec3::new((max_x - min_x).max(1.0), plan.storey_height.max(1.0), (max_z - min_z).max(1.0));
	let min = Vec3::new(min_x, 0.0, min_z);
	(min, extent)
}

fn build_i_apartment_floor_plan_examples() -> Vec<IApartmentFloorPlanExampleCell> {
	let specs = i_apartment_floor_plan_examples_specs();
	let mut fitted = Vec::new();
	for (extent, seed) in &specs {
		let bounds = les_halles_confines_bounds(*extent);
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
		match IApartmentParameterized::sample(&empty, noise).and_then(|params| {
			let inbound = IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty);
			let confines = Confines::new(bounds, 0.0, inbound);
			IApartmentFloorPlan::from_parameterized(params, &confines).map(|(plan, _)| plan)
		}) {
			Ok(plan) => fitted.push(plan),
			Err(err) => {
				bevy::log::error!(
					"i-apartment-floor-plan-examples ({extent:?} seed={seed}) failed: {err}"
				);
			}
		}
	}
	let layout: Vec<(Vec3, Vec3)> = fitted.iter().map(i_apartment_plan_footprint_aabb).collect();
	fitted
		.into_iter()
		.enumerate()
		.map(|(i, plan)| {
			let (local_min, _) = layout[i];
			let cell_origin = gallery_grid_offset(
				|j| layout[j].1,
				layout.len(),
				i,
				I_APARTMENT_GALLERY_COLS,
				I_APARTMENT_GALLERY_GAP,
			);
			// Map plan-space AABB min → gallery cell origin (handles asymmetric L/Z).
			let offset = cell_origin - local_min;
			IApartmentFloorPlanExampleCell { offset, plan }
		})
		.collect()
}

fn build_i_apartment_full_storey_examples() -> Vec<IApartmentFullStoreyExampleCell> {
	let specs = i_apartment_floor_plan_examples_specs();
	let mut fitted = Vec::new();
	for (extent, seed) in &specs {
		let bounds = les_halles_confines_bounds(*extent);
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
		match IApartmentParameterized::sample(&empty, noise).and_then(|params| {
			let inbound = IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty);
			let confines = Confines::new(bounds, 0.0, inbound);
			IApartmentFloorPlan::from_parameterized(params, &confines)
				.and_then(|(plan, _)| IApartmentFullStorey::from_floor_plan(plan, noise))
				.map(|(storey, _)| storey)
		}) {
			Ok(storey) => fitted.push(storey),
			Err(err) => {
				bevy::log::error!(
					"i-apartment-full-storey-examples ({extent:?} seed={seed}) failed: {err}"
				);
			}
		}
	}
	let layout: Vec<(Vec3, Vec3)> =
		fitted.iter().map(|s| i_apartment_plan_footprint_aabb(&s.floor_plan)).collect();
	fitted
		.into_iter()
		.enumerate()
		.map(|(i, storey)| {
			let (local_min, _) = layout[i];
			let cell_origin = gallery_grid_offset(
				|j| layout[j].1,
				layout.len(),
				i,
				I_APARTMENT_GALLERY_COLS,
				I_APARTMENT_GALLERY_GAP,
			);
			let offset = cell_origin - local_min;
			IApartmentFullStoreyExampleCell { offset, storey }
		})
		.collect()
}

const LIVABLE_APARTMENTS_GALLERY_COLS: usize = 3;
const LIVABLE_APARTMENTS_GALLERY_GAP: f32 = 8.0;

/// Curated `(extent, seed, hall_width, targets)` hosts for standalone [`LivableApartments`].
/// Larger catalog targets encourage multi-cell (L / non-rect) apartment groups.
fn livable_apartments_examples_specs() -> Vec<(Vec3, i32, Option<f32>, Option<Vec<f32>>)> {
	vec![
		(Vec3::new(24.0, 3.5, 18.0), 1337, Some(2.5), Some(vec![55.0, 48.0, 40.0, 30.0])),
		(Vec3::new(24.0, 3.5, 18.0), 0, None, None),
		(Vec3::new(30.0, 3.5, 22.0), 3, Some(3.0), Some(vec![55.0, 48.0, 40.0, 30.0])),
		(Vec3::new(20.0, 3.5, 16.0), 7, Some(2.0), None),
		(Vec3::new(28.0, 3.5, 20.0), 11, None, Some(vec![60.0, 50.0, 42.0, 35.0, 28.0])),
		(Vec3::new(36.0, 3.5, 24.0), 19, Some(3.5), None),
		(Vec3::new(22.0, 3.5, 22.0), 42, Some(2.5), Some(vec![55.0, 48.0, 40.0])),
		(Vec3::new(32.0, 3.5, 18.0), 55, None, None),
		(Vec3::new(26.0, 3.5, 26.0), 77, Some(2.8), Some(vec![58.0, 50.0, 42.0, 32.0])),
	]
}

fn livable_apartments_host_footprint(block: &LivableApartments) -> (Vec3, Vec3) {
	let min = Vec3::from(block.confines.bounds.min);
	let max = Vec3::from(block.confines.bounds.max);
	(min, max - min)
}

fn build_livable_apartments_examples() -> Vec<LivableApartmentsExampleCell> {
	let specs = livable_apartments_examples_specs();
	let mut fitted = Vec::new();
	for (extent, seed, hall_width, targets) in &specs {
		let host = les_halles_confines_bounds(*extent);
		let inbound = openings_from_preview(
			&crate::commands::show::halls_to_shafts::default_demo_openings(*extent),
		);
		let confines = Confines::new(host, 0.0, inbound);
		let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
		match LivableApartments::from_confines_with(
			&confines,
			noise,
			LivableApartmentsOptions { hall_width: *hall_width, targets: targets.clone() },
		) {
			Ok((block, _)) => {
				let multi = block.apartments.iter().filter(|a| a.cells.len() >= 2).count();
				if multi == 0 {
					bevy::log::warn!(
						"livable-apartments-examples ({extent:?} seed={seed}): no multi-cell groups"
					);
				}
				fitted.push(block);
			}
			Err(err) => {
				bevy::log::error!(
					"livable-apartments-examples ({extent:?} seed={seed}) failed: {err}"
				);
			}
		}
	}
	let layout: Vec<(Vec3, Vec3)> = fitted.iter().map(livable_apartments_host_footprint).collect();
	fitted
		.into_iter()
		.enumerate()
		.map(|(i, block)| {
			let (local_min, _) = layout[i];
			let cell_origin = gallery_grid_offset(
				|j| layout[j].1,
				layout.len(),
				i,
				LIVABLE_APARTMENTS_GALLERY_COLS,
				LIVABLE_APARTMENTS_GALLERY_GAP,
			);
			let offset = cell_origin - local_min;
			LivableApartmentsExampleCell { offset, block }
		})
		.collect()
}

const LIVABLE_APARTMENT_GALLERY_COLS: usize = 3;
const LIVABLE_APARTMENT_GALLERY_GAP: f32 = 8.0;

#[derive(Clone, Copy)]
enum LivableApartmentExampleShape {
	/// Single rectangular host with a south hall door.
	Rect { extent: Vec3 },
	/// L: stem along +X, bar along +Z (door on stem south).
	LStem { stem: Vec2, bar: Vec2, height: f32 },
	/// T: crossbar along +X, stem along +Z from mid (door on stem south tip).
	TBar { bar: Vec2, stem: Vec2, height: f32 },
}

/// Curated `(shape, seed)` hosts — small stress cases through larger well-formed plans.
fn livable_apartment_examples_specs() -> Vec<(LivableApartmentExampleShape, i32)> {
	vec![
		// Small / tight
		(LivableApartmentExampleShape::Rect { extent: Vec3::new(8.0, 3.0, 6.5) }, 7),
		(
			LivableApartmentExampleShape::LStem {
				stem: Vec2::new(8.0, 4.5),
				bar: Vec2::new(4.5, 7.0),
				height: 3.0,
			},
			11,
		),
		(LivableApartmentExampleShape::Rect { extent: Vec3::new(12.0, 3.0, 9.0) }, 21),
		// Medium
		(
			LivableApartmentExampleShape::TBar {
				bar: Vec2::new(14.0, 4.5),
				stem: Vec2::new(5.0, 7.0),
				height: 3.0,
			},
			3,
		),
		(
			LivableApartmentExampleShape::LStem {
				stem: Vec2::new(12.0, 6.0),
				bar: Vec2::new(6.5, 10.0),
				height: 3.0,
			},
			42,
		),
		(LivableApartmentExampleShape::Rect { extent: Vec3::new(18.0, 3.2, 14.0) }, 55),
		// Larger / well-formed
		(LivableApartmentExampleShape::Rect { extent: Vec3::new(22.0, 3.2, 16.0) }, 77),
		(
			LivableApartmentExampleShape::LStem {
				stem: Vec2::new(16.0, 7.0),
				bar: Vec2::new(8.0, 14.0),
				height: 3.2,
			},
			99,
		),
		(
			LivableApartmentExampleShape::TBar {
				bar: Vec2::new(20.0, 6.0),
				stem: Vec2::new(7.0, 12.0),
				height: 3.2,
			},
			1337,
		),
		(LivableApartmentExampleShape::Rect { extent: Vec3::new(28.0, 3.4, 20.0) }, 19),
		(
			LivableApartmentExampleShape::LStem {
				stem: Vec2::new(20.0, 9.0),
				bar: Vec2::new(10.0, 16.0),
				height: 3.4,
			},
			201,
		),
		(
			LivableApartmentExampleShape::TBar {
				bar: Vec2::new(26.0, 7.0),
				stem: Vec2::new(8.0, 14.0),
				height: 3.4,
			},
			404,
		),
	]
}

fn livable_apartment_host_footprint(apt: &LivableApartment) -> (Vec3, Vec3) {
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	for part in apt.cells.iter() {
		let pmin = Vec3::from(part.confines.bounds.min);
		let pmax = Vec3::from(part.confines.bounds.max);
		min = min.min(pmin);
		max = max.max(pmax);
	}
	if !min.is_finite() {
		return (Vec3::ZERO, Vec3::splat(1.0));
	}
	(min, max - min)
}

fn demo_multi_confines_with_south_door(parts: &[(Vec3, Vec3)], door_part: usize) -> MultiConfines {
	let mut regions = Vec::new();
	for (i, (min, max)) in parts.iter().enumerate() {
		let mut openings = Openings::new();
		if i == door_part {
			let w = ((max.x - min.x) * 0.3).clamp(0.8, 1.2);
			let cx = 0.5 * (min.x + max.x);
			let door_h = ((max.y - min.y) * 0.72).clamp(2.0, (max.y - min.y).max(2.0));
			openings.insert(
				OpeningId::new("demo_apt_door"),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(cx - w * 0.5, min.y, min.z - 0.2),
					Vec3::new(cx + w * 0.5, min.y + door_h, min.z + 0.2),
				)),
			);
		}
		regions.push(FillRegion::new(
			SpaceKind::InternalSpace,
			Confines::new(Aabb3d::from_min_max(*min, *max), 0.0, openings),
		));
	}
	MultiConfines::new(regions)
}

fn livable_apartment_example_multi(shape: LivableApartmentExampleShape) -> MultiConfines {
	match shape {
		LivableApartmentExampleShape::Rect { extent } => {
			MultiConfines::from(demo_common_bedroom_confines(extent, true))
		}
		LivableApartmentExampleShape::LStem { stem, bar, height } => {
			// Stem: [0,stem.x] × [0,stem.y]; bar: [0,bar.x] × [stem.y, stem.y+bar.y] (L).
			demo_multi_confines_with_south_door(
				&[
					(Vec3::ZERO, Vec3::new(stem.x, height, stem.y)),
					(Vec3::new(0.0, 0.0, stem.y), Vec3::new(bar.x, height, stem.y + bar.y)),
				],
				0,
			)
		}
		LivableApartmentExampleShape::TBar { bar, stem, height } => {
			// Crossbar along X at z=stem.y..stem.y+bar.y; stem centered under it.
			let stem_x0 = (bar.x - stem.x) * 0.5;
			demo_multi_confines_with_south_door(
				&[
					(Vec3::new(stem_x0, 0.0, 0.0), Vec3::new(stem_x0 + stem.x, height, stem.y)),
					(Vec3::new(0.0, 0.0, stem.y), Vec3::new(bar.x, height, stem.y + bar.y)),
				],
				0,
			)
		}
	}
}

fn build_livable_apartment_examples() -> Vec<LivableApartmentExampleCell> {
	let specs = livable_apartment_examples_specs();
	let mut fitted = Vec::new();
	for (shape, seed) in &specs {
		let multi = livable_apartment_example_multi(*shape);
		let noise = NoiseParams { seed: *seed, ..NoiseParams::default() };
		match LivableApartment::from_multi(0, &multi, noise) {
			Ok((apt, _)) => fitted.push(apt),
			Err(err) => {
				bevy::log::error!("livable-apartment-examples (seed={seed}) failed: {err}");
			}
		}
	}
	let layout: Vec<(Vec3, Vec3)> = fitted.iter().map(livable_apartment_host_footprint).collect();
	fitted
		.into_iter()
		.enumerate()
		.map(|(i, apartment)| {
			let (local_min, _) = layout[i];
			let cell_origin = gallery_grid_offset(
				|j| layout[j].1,
				layout.len(),
				i,
				LIVABLE_APARTMENT_GALLERY_COLS,
				LIVABLE_APARTMENT_GALLERY_GAP,
			);
			let offset = cell_origin - local_min;
			LivableApartmentExampleCell { offset, apartment }
		})
		.collect()
}

fn draw_livable_apartment_gizmos(gizmos: &mut Gizmos, apt: &LivableApartment, tf: Transform) {
	use buildings::usage_areas::livable_apartment::ApartmentRoom;
	let host = Color::srgb(0.72, 0.72, 0.78);
	for part in apt.cells.iter() {
		gizmos.aabb_3d(part.confines.bounds, tf, host);
	}
	let y0 = apt
		.cells
		.parts
		.first()
		.map(|p| Vec3::from(p.confines.bounds.min).y)
		.unwrap_or(0.0);
	let y1 = apt
		.cells
		.parts
		.first()
		.map(|p| Vec3::from(p.confines.bounds.max).y)
		.unwrap_or(3.0);
	// Max-rect hosts (decomposition).
	let max_c = Color::srgb(0.55, 0.65, 0.95);
	for (ri, r) in apt.max_rects.iter().enumerate() {
		let color = if ri % 2 == 0 { max_c } else { Color::srgb(0.45, 0.55, 0.9) };
		gizmos.aabb_3d(
			Aabb3d::from_min_max(Vec3::new(r.min.x, y0, r.min.y), Vec3::new(r.max.x, y1, r.max.y)),
			tf,
			color,
		);
	}
	// Walkway / open hall bands.
	let walk = Color::srgb(0.95, 0.85, 0.25);
	for (wi, band) in apt.walkways.iter().enumerate() {
		let color = if wi % 2 == 0 { walk } else { Color::srgb(0.9, 0.7, 0.2) };
		gizmos.aabb_3d(
			Aabb3d::from_min_max(
				Vec3::new(band.min.x, y0, band.min.y),
				Vec3::new(band.max.x, y1, band.max.y),
			),
			tf,
			color,
		);
	}
	for room in &apt.rooms {
		let (bounds, color) = match room {
			ApartmentRoom::Entryway { confines, .. } => {
				(confines.bounds, Color::srgb(0.25, 0.88, 0.92))
			}
			ApartmentRoom::OpenHall { confines, .. } => {
				(confines.bounds, Color::srgb(0.95, 0.9, 0.35))
			}
			ApartmentRoom::HouseholdCloset { confines, .. } => {
				(confines.bounds, Color::srgb(0.55, 0.55, 0.6))
			}
			ApartmentRoom::Bedroom(r) => {
				(label_bounds(&r.room_type), Color::srgb(0.35, 0.55, 0.95))
			}
			ApartmentRoom::Living(r) => (label_bounds(&r.room_type), Color::srgb(0.95, 0.55, 0.25)),
			ApartmentRoom::Kitchen(r) => (label_bounds(&r.room_type), Color::srgb(0.35, 0.9, 0.45)),
			ApartmentRoom::Dining(r) => (label_bounds(&r.room_type), Color::srgb(0.45, 0.85, 0.35)),
			ApartmentRoom::Bathroom(r) => {
				(label_bounds(&r.room_type), Color::srgb(0.55, 0.75, 0.95))
			}
			ApartmentRoom::HalfBath(r) => {
				(label_bounds(&r.room_type), Color::srgb(0.65, 0.8, 0.95))
			}
			ApartmentRoom::Sitting(r) => {
				(label_bounds(&r.room_type), Color::srgb(0.95, 0.65, 0.35))
			}
			ApartmentRoom::Study(r) => (label_bounds(&r.room_type), Color::srgb(0.75, 0.45, 0.85)),
		};
		gizmos.aabb_3d(bounds, tf, color);
	}
}

const LIVABLE_RECT_GALLERY_COLS: usize = 4;
const LIVABLE_RECT_GALLERY_GAP: f32 = 6.0;

#[derive(Clone, Copy)]
struct LivableRectangleExampleSpec {
	extent: Vec2,
	height: f32,
	ports: &'static [(CardinalFace, f32)],
	strategy: RectLivableStrategy,
	seed: i32,
	program: &'static [RectQuarterKind],
}

fn livable_rectangles_examples_specs() -> Vec<LivableRectangleExampleSpec> {
	use CardinalFace::*;
	use RectLivableStrategy::*;
	use RectQuarterKind::*;
	vec![
		// Small, 1 south door, SingleClosed
		LivableRectangleExampleSpec {
			extent: Vec2::new(4.0, 5.0),
			height: 3.0,
			ports: &[(South, 0.5)],
			strategy: SingleClosed,
			seed: 1,
			program: &[Bedroom],
		},
		// Small, CaseAttempt
		LivableRectangleExampleSpec {
			extent: Vec2::new(5.0, 6.0),
			height: 3.0,
			ports: &[(South, 0.5)],
			strategy: CaseAttempt,
			seed: 2,
			program: &[Eating, Living, Bathroom],
		},
		// Medium AllOpen, 1 port
		LivableRectangleExampleSpec {
			extent: Vec2::new(8.0, 10.0),
			height: 3.0,
			ports: &[(South, 0.5)],
			strategy: AllOpen,
			seed: 3,
			program: &[Eating, Living],
		},
		// Opposite ports + SpineHall
		LivableRectangleExampleSpec {
			extent: Vec2::new(8.0, 10.0),
			height: 3.0,
			ports: &[(South, 0.5), (North, 0.5)],
			strategy: SpineHall,
			seed: 4,
			program: &[Eating, Living, Bedroom, Bathroom],
		},
		// Adjacent (L) ports
		LivableRectangleExampleSpec {
			extent: Vec2::new(9.0, 8.0),
			height: 3.0,
			ports: &[(South, 0.35), (East, 0.5)],
			strategy: CaseAttempt,
			seed: 5,
			program: &[Eating, Living, Bedroom],
		},
		// 3 ports
		LivableRectangleExampleSpec {
			extent: Vec2::new(10.0, 10.0),
			height: 3.2,
			ports: &[(South, 0.5), (North, 0.35), (West, 0.5)],
			strategy: SpineHall,
			seed: 6,
			program: &[Eating, Living, Bedroom, Bathroom],
		},
		// Large Guillotine
		LivableRectangleExampleSpec {
			extent: Vec2::new(12.0, 14.0),
			height: 3.2,
			ports: &[(South, 0.5)],
			strategy: GuillotineSplit,
			seed: 7,
			program: &[Eating, Living, Bedroom, Bathroom],
		},
		// Large CaseAttempt, 2 opposite
		LivableRectangleExampleSpec {
			extent: Vec2::new(12.0, 16.0),
			height: 3.2,
			ports: &[(South, 0.4), (North, 0.6)],
			strategy: CaseAttempt,
			seed: 8,
			program: &[Eating, Living, Bedroom, Bedroom, Bathroom],
		},
		// Compact 3×4 SingleClosed
		LivableRectangleExampleSpec {
			extent: Vec2::new(3.5, 4.5),
			height: 3.0,
			ports: &[(South, 0.5)],
			strategy: SingleClosed,
			seed: 9,
			program: &[Bathroom],
		},
		// Medium AllOpen adjacent ports
		LivableRectangleExampleSpec {
			extent: Vec2::new(7.0, 7.0),
			height: 3.0,
			ports: &[(South, 0.5), (West, 0.5)],
			strategy: AllOpen,
			seed: 10,
			program: &[Eating, Living, Sitting],
		},
	]
}

fn build_livable_rectangles_examples() -> Vec<LivableRectangleExampleCell> {
	let specs = livable_rectangles_examples_specs();
	let mut fitted = Vec::new();
	for (si, spec) in specs.iter().enumerate() {
		let host = bevy_math::bounding::Aabb2d { min: Vec2::ZERO, max: spec.extent };
		let openings = passages_on_faces(host, 0.0, spec.height, spec.ports);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(spec.extent.x, spec.height, spec.extent.y)),
			0.0,
			openings,
		);
		let params = RectangularLivableAreaParameterized {
			strategy: spec.strategy,
			min_hall: 1.0,
			closed_max_area: 36.0,
		};
		let noise = NoiseParams { seed: spec.seed, ..NoiseParams::default() };
		match RectangularLivableArea::fit_with_params(&confines, noise, params, spec.program) {
			Ok((area, _)) => fitted.push(area),
			Err(err) => {
				bevy::log::error!(
					"livable-rectangles-examples[{si}] (seed={}) failed: {err}",
					spec.seed
				);
			}
		}
	}
	let layout: Vec<(Vec3, Vec3)> = fitted
		.iter()
		.map(|a| {
			let fp = a.confines.footprint();
			let h = Vec3::from(a.confines.bounds.max).y - Vec3::from(a.confines.bounds.min).y;
			(Vec3::ZERO, Vec3::new(fp.x, h, fp.y))
		})
		.collect();
	fitted
		.into_iter()
		.enumerate()
		.map(|(i, area)| {
			let cell_origin = gallery_grid_offset(
				|j| layout[j].1,
				layout.len(),
				i,
				LIVABLE_RECT_GALLERY_COLS,
				LIVABLE_RECT_GALLERY_GAP,
			);
			LivableRectangleExampleCell { offset: cell_origin, area }
		})
		.collect()
}

fn draw_livable_rectangle_gizmos(
	gizmos: &mut Gizmos,
	area: &RectangularLivableArea,
	tf: Transform,
) {
	let y0 = Vec3::from(area.confines.bounds.min).y;
	let y1 = Vec3::from(area.confines.bounds.max).y;
	gizmos.aabb_3d(area.confines.bounds, tf, Color::srgb(0.7, 0.7, 0.78));
	// Passage AABBs
	for (_, o) in area.confines.openings.iter() {
		if matches!(o.label, OpeningLabel::Passage) {
			gizmos.aabb_3d(o.bounds, tf, Color::srgb(0.95, 0.35, 0.35));
		}
	}
	// Hall bands
	for (wi, band) in area.walkways.iter().enumerate() {
		let color =
			if wi % 2 == 0 { Color::srgb(0.95, 0.85, 0.25) } else { Color::srgb(0.9, 0.7, 0.2) };
		gizmos.aabb_3d(
			Aabb3d::from_min_max(
				Vec3::new(band.min.x, y0, band.min.y),
				Vec3::new(band.max.x, y1, band.max.y),
			),
			tf,
			color,
		);
	}
	for room in &area.rooms {
		let (bounds, color) = match room {
			RectAreaRoom::OpenBand { confines, .. } => {
				(confines.bounds, Color::srgb(0.95, 0.9, 0.35))
			}
			RectAreaRoom::HouseholdCloset { confines, .. } => {
				(confines.bounds, Color::srgb(0.55, 0.55, 0.6))
			}
			RectAreaRoom::Bedroom(r) => (label_bounds(&r.room_type), Color::srgb(0.35, 0.55, 0.95)),
			RectAreaRoom::Living(r) => (label_bounds(&r.room_type), Color::srgb(0.95, 0.55, 0.25)),
			RectAreaRoom::Eating(r) => (label_bounds(&r.room_type), Color::srgb(0.4, 0.88, 0.4)),
			RectAreaRoom::Kitchen(r) => (label_bounds(&r.room_type), Color::srgb(0.35, 0.9, 0.45)),
			RectAreaRoom::Dining(r) => (label_bounds(&r.room_type), Color::srgb(0.45, 0.85, 0.35)),
			RectAreaRoom::Bathroom(r) => {
				(label_bounds(&r.room_type), Color::srgb(0.55, 0.75, 0.95))
			}
			RectAreaRoom::HalfBath(r) => (label_bounds(&r.room_type), Color::srgb(0.65, 0.8, 0.95)),
			RectAreaRoom::Sitting(r) => (label_bounds(&r.room_type), Color::srgb(0.95, 0.65, 0.35)),
			RectAreaRoom::Study(r) => (label_bounds(&r.room_type), Color::srgb(0.75, 0.45, 0.85)),
		};
		gizmos.aabb_3d(bounds, tf, color);
	}
}

fn label_bounds(label: &building_components::LabelNode) -> Aabb3d {
	let c = label.placement.translation;
	let e = label.placement.scale.abs() * 0.5;
	Aabb3d::from_min_max(c - e, c + e)
}

fn draw_livable_apartments_block_gizmos(
	gizmos: &mut Gizmos,
	block: &LivableApartments,
	tf: Transform,
	cyan: Color,
	amber: Color,
	magenta: Color,
) {
	let lime = Color::srgb(0.35, 0.95, 0.4);
	let host_color = Color::srgb(0.75, 0.75, 0.8);

	gizmos.aabb_3d(block.confines.bounds, tf, host_color);

	for (hi, band) in block.halls.hall_bands.iter().enumerate() {
		let y0 = Vec3::from(block.confines.bounds.min).y;
		let y1 = Vec3::from(block.confines.bounds.max).y;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(band.min.x, y0, band.min.y),
			Vec3::new(band.max.x, y1, band.max.y),
		);
		let color = if hi % 2 == 0 { lime } else { Color::srgb(0.2, 0.8, 0.55) };
		gizmos.aabb_3d(bounds, tf, color);
	}

	for (ai, apt) in block.apartments.iter().enumerate() {
		// Distinct hue per apartment so multi-cell (non-rect) groups read clearly.
		let t = (ai as f32 * 0.17) % 1.0;
		let color = Color::srgb(0.25 + 0.55 * t, 0.55 + 0.35 * (1.0 - t), 0.85);
		for part in apt.cells.iter() {
			gizmos.aabb_3d(part.confines.bounds, tf, color);
		}
	}

	let mut passage_i = 0usize;
	let mut shaft_i = 0usize;
	for (_id, opening) in block.confines.openings.iter() {
		match opening.label {
			OpeningLabel::Shaft => {
				let color = if shaft_i % 2 == 0 { magenta } else { Color::srgb(0.75, 0.35, 1.0) };
				gizmos.aabb_3d(opening.bounds, tf, color);
				shaft_i += 1;
			}
			OpeningLabel::Passage => {
				let color = if passage_i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds, tf, color);
				passage_i += 1;
			}
			_ => {}
		}
	}
}

fn draw_i_apartment_primary_rect_gizmos(
	gizmos: &mut Gizmos,
	plan: &IApartmentFloorPlan,
	tf: Transform,
) {
	let y0 = plan.center_xz.y;
	let y1 = y0 + plan.storey_height;
	for (i, rect) in plan.primary_rects.iter().enumerate() {
		let color =
			if i % 2 == 0 { Color::srgb(0.25, 0.55, 0.95) } else { Color::srgb(0.35, 0.8, 0.55) };
		let bounds = Aabb3d::from_min_max(
			Vec3::new(rect.min_x, y0, rect.min_z),
			Vec3::new(rect.max_x, y1, rect.max_z),
		);
		gizmos.aabb_3d(bounds, tf, color);
	}
}

/// Spawn preview when the subject changes. LOD flips update host levels in-place
/// ([`lod::LodRefreshCorePlugin`] + domain refresh-pass systems).
pub fn present_preview_lod(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	viewer: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, With<LodViewer>)>,
	mut cache: ResMut<CachedPreview>,
	roots: Query<Entity, With<PreviewRoot>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut last_subject: Local<Option<(PreviewSubject, Transform)>>,
) {
	let subject_key = (config.subject.clone(), config.transform);
	let subject_changed = last_subject.as_ref() != Some(&subject_key);
	let has_root = roots.iter().next().is_some();

	if matches!(config.subject, PreviewSubject::None) {
		if subject_changed || has_root {
			for entity in &roots {
				commands.entity(entity).despawn();
			}
			*last_subject = Some(subject_key);
			cache.key = None;
			cache.wizards_tower = None;
			cache.stacked_rings = None;
			cache.bedroom = None;
			cache.noisy_wall = None;
		}
		return;
	}

	cache.rebuild_if_needed(&config);

	if !subject_changed && has_root {
		return;
	}

	for entity in &roots {
		commands.entity(entity).despawn();
	}
	*last_subject = Some(subject_key);

	let Ok((viewer_entity, pose, viewer_bounds)) = viewer.single() else {
		return;
	};
	let driver_bounds = viewer_bounds
		.map(|b| b.0)
		.unwrap_or_else(|| point_bounds(pose.current.translation));
	let lod_ref: LodRef = pose.as_lod_ref(viewer_entity, &driver_bounds);

	let transform = config.transform;
	match &config.subject {
		PreviewSubject::None => {}
		PreviewSubject::Linear => {
			spawn_preview(&mut commands, transform, RoughStoneworkLinear.host(&lod_ref));
		}
		PreviewSubject::Arc90 => {
			spawn_preview(&mut commands, transform, RoughStonework90.host(&lod_ref));
		}
		PreviewSubject::Arc180 => {
			spawn_preview(&mut commands, transform, RoughStonework180.host(&lod_ref));
		}
		PreviewSubject::Slice90 => {
			spawn_preview(&mut commands, transform, RoughStoneworkSlice90.host(&lod_ref));
		}
		PreviewSubject::Pitch { rise, run, length, tile_width, left, right } => {
			let mut pitch = Pitch::new(*rise, *run, *tile_width);
			if let Some(len) = length {
				pitch = pitch.with_length(*len);
			}
			if let Some(base) = left {
				pitch = pitch.with_left(*base);
			}
			if let Some(base) = right {
				pitch = pitch.with_right(*base);
			}
			let roof = RoofNode::shepherds_thatch(RoofGeometry::pitch(pitch), Placement::IDENTITY);
			spawn_preview(&mut commands, transform, roof.host(&lod_ref));
		}
		PreviewSubject::TessellatedTriangle { a, b, c } => {
			let panel = PanelNode::rough_stone(
				PanelGeometry::tessellated_triangle(TessellatedTriangle::new(*a, *b, *c)),
				Placement::IDENTITY,
			);
			spawn_preview(&mut commands, transform, panel.host(&lod_ref));
		}
		PreviewSubject::TessellatedTriangle3d { a, b, c } => {
			let panel = TessellatedTrianglePanel::rough_stone(*a, *b, *c);
			spawn_building_preview(&mut commands, transform, &panel, &lod_ref);
		}
		PreviewSubject::ClippedTessellatedTriangle { a, b, c, clip, min_dihedral, no_joint } => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let complex = ClippedTessellatedTriangle::rough_stone(*a, *b, *c, clip.clone())
				.with_joint_policy(policy)
				.into_complex();
			spawn_building_preview(&mut commands, transform, &complex, &lod_ref);
		}
		PreviewSubject::ClippedQuadPanel { a0, a1, b0, b1, clip, min_dihedral, no_joint } => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let complex = ClippedQuadPanel::rough_stone(*a0, *a1, *b0, *b1, clip.clone())
				.with_joint_policy(policy)
				.into_complex();
			spawn_building_preview(&mut commands, transform, &complex, &lod_ref);
		}
		PreviewSubject::ClippedRuledStrip { min_dihedral, no_joint } => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let rail_a = [
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(0.0, 0.0, 2.0),
				Vec3::new(0.0, 0.0, 4.0),
				Vec3::new(0.0, 0.0, 6.0),
			];
			let rail_b = [
				Vec3::new(2.5, 0.8, 0.0),
				Vec3::new(2.2, 1.0, 2.0),
				Vec3::new(2.8, 0.7, 4.0),
				Vec3::new(2.4, 1.1, 6.0),
			];
			let mid_clip = vec![
				Vec3::new(0.5, 0.2, 2.4),
				Vec3::new(1.8, 0.5, 2.4),
				Vec3::new(1.8, 0.5, 3.4),
				Vec3::new(0.5, 0.2, 3.4),
			];
			let strip = ClippedRuledStrip::from_lines(
				building_components::panels::PanelStyle::ShepherdsThatch,
				rail_a,
				rail_b,
				[None, Some(mid_clip), None],
			)
			.with_joint_policy(policy);
			spawn_building_preview(&mut commands, transform, &strip, &lod_ref);
		}
		PreviewSubject::Tube {
			min_dihedral,
			no_joint,
			no_floor,
			no_ceiling,
			no_left,
			no_right,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let mut faces = TubeFaces::ALL;
			if *no_floor {
				faces = faces.without_floor();
			}
			if *no_ceiling {
				faces = faces.without_ceiling();
			}
			if *no_left {
				faces = faces.without_left();
			}
			if *no_right {
				faces = faces.without_right();
			}
			// Level start, then plan bend + pitch; slight roll on the kink station.
			let nodes = [
				TubeCrossSectionNode::new(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, 2.2, 1.0, 1.0),
				TubeCrossSectionNode::new(Vec3::new(0.0, 0.0, 3.0), 1.2, 1.2, 2.2, 1.0, 1.0),
				TubeCrossSectionNode::new(Vec3::new(2.0, 0.5, 6.0), 1.3, 1.1, 2.4, 1.1, 0.9)
					.with_roll(0.15),
				TubeCrossSectionNode::new(Vec3::new(5.0, 1.0, 8.0), 1.2, 1.2, 2.2, 1.0, 1.0),
			];
			// Opening on the left wall of the middle bay (between stations 1–2).
			let left_clip = vec![
				Vec3::new(-1.0, 0.4, 3.6),
				Vec3::new(-1.0, 1.6, 3.6),
				Vec3::new(-0.2, 1.6, 5.0),
				Vec3::new(-0.2, 0.4, 5.0),
			];
			let tube = Tube::from_nodes_with_clips(
				building_components::panels::PanelStyle::RoughStonework,
				nodes,
				std::iter::empty(),
				std::iter::empty(),
				[None, Some(left_clip), None],
				std::iter::empty(),
			)
			.with_joint_policy(policy)
			.with_faces(faces);
			spawn_building_preview(&mut commands, transform, &tube, &lod_ref);
		}
		PreviewSubject::ConnectingHall => {
			let (end_a, end_b) = connecting_hall_demo_endpoints();
			let hall = ConnectingHall::rough_stone(end_a, end_b);
			spawn_building_preview(&mut commands, transform, &hall, &lod_ref);
		}
		PreviewSubject::ConnectingStairwell { case, tread_fill, kind } => {
			spawn_connecting_stairwell(
				&mut commands,
				transform,
				*case,
				*tread_fill,
				*kind,
				&lod_ref,
			);
		}
		PreviewSubject::ConnectingStairwellExamples { kind } => {
			for cell in crate::commands::show::connecting_stairwell::pathological_gallery(*kind) {
				let tf = transform * Transform::from_translation(cell.offset);
				for well in cell.stairwells {
					spawn_building_preview(&mut commands, tf, &well, &lod_ref);
				}
			}
		}
		PreviewSubject::ArcFloor { radius, storey_height, floor, ceiling, openings } => {
			let floor_shell = ArcFloor::new(ArcFloorParams {
				center_xz: Vec3::ZERO,
				radius: *radius,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { ArcFloorSlab::Solid } else { ArcFloorSlab::None },
				ceiling: if *ceiling { ArcFloorSlab::Solid } else { ArcFloorSlab::None },
				..ArcFloorParams::default()
			});
			spawn_building_preview(&mut commands, transform, &floor_shell, &lod_ref);
		}
		PreviewSubject::ArcTower {
			radius,
			floor_count,
			storey_height,
			floor_hole,
			no_base_floor,
			no_ceiling,
		} => {
			let mut openings = Openings::new();
			for (id, t, label) in [
				("door", 0.0, OpeningLabel::Passage),       // +X
				("window_w", 0.25, OpeningLabel::Aperture), // −Z
				("window_e", 0.5, OpeningLabel::Aperture),  // −X
				("window_n", 0.75, OpeningLabel::Aperture), // +Z
			] {
				let (id, opening) =
					ArcFloor::plan_opening_at_t(id, label, Vec3::ZERO, *radius, *storey_height, t);
				openings.insert(id, opening);
			}
			let tower = ArcTower::new(ArcTowerParams {
				center_xz: Vec3::ZERO,
				radius: *radius,
				floor_count: *floor_count,
				storey_height: *storey_height,
				openings,
				base_floor: if *no_base_floor { ArcFloorSlab::None } else { ArcFloorSlab::Solid },
				intermediate_floors: ArcFloorSlab::Solid,
				top_ceiling: if *no_ceiling { ArcFloorSlab::None } else { ArcFloorSlab::Solid },
				intermediate_floor_hole: *floor_hole,
				..ArcTowerParams::default()
			});
			spawn_building_preview(&mut commands, transform, &tower, &lod_ref);
		}
		PreviewSubject::ConnectingShells => {
			let demo = ConnectingShells::new();
			spawn_building_preview(&mut commands, transform, &demo, &lod_ref);
		}
		PreviewSubject::Trazaloid {
			footprint_x,
			footprint_z,
			ridge_x,
			ridge_z,
			lower_height,
			upper_height,
			band_vertical_offset,
			waist_horizontal_offset,
			openings,
			floor,
			no_ceiling,
			face_post_count,
		} => {
			let shell = Trazaloid::new(TrazaloidParams {
				footprint: Vec2::new(*footprint_x, *footprint_z),
				ridge: Vec2::new(*ridge_x, *ridge_z),
				lower_height: *lower_height,
				upper_height: *upper_height,
				band_vertical_offset: *band_vertical_offset,
				waist_horizontal_offset: *waist_horizontal_offset,
				openings: openings_from_preview(openings),
				floor: if *floor { TrazaloidSlab::Solid } else { TrazaloidSlab::None },
				ceiling: if *no_ceiling { TrazaloidSlab::None } else { TrazaloidSlab::Solid },
				face_post_count: *face_post_count,
				..TrazaloidParams::default()
			});
			spawn_building_preview(&mut commands, transform, &shell, &lod_ref);
		}
		PreviewSubject::PitchedRectangularRoof {
			footprint_x,
			footprint_z,
			ridge_height,
			eave_height,
			ridge_inset,
			gables,
			no_walls,
			no_hips,
			openings,
		} => {
			let shell = pitched_roof_from_preview(
				*footprint_x,
				*footprint_z,
				*ridge_height,
				*eave_height,
				*ridge_inset,
				*gables,
				*no_walls,
				*no_hips,
				openings,
			);
			spawn_building_preview(&mut commands, transform, &shell, &lod_ref);
		}
		PreviewSubject::RectFloor {
			footprint_x,
			footprint_z,
			storey_height,
			openings,
			floor,
			ceiling,
		} => {
			let shell = RectFloor::new(RectFloorParams {
				center_xz: Vec3::ZERO,
				footprint: Vec2::new(*footprint_x, *footprint_z),
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { RectFloorSlab::Solid } else { RectFloorSlab::None },
				ceiling: if *ceiling { RectFloorSlab::Solid } else { RectFloorSlab::None },
				..RectFloorParams::default()
			});
			spawn_building_preview(&mut commands, transform, &shell, &lod_ref);
		}
		PreviewSubject::RoundedRectFloor {
			footprint_x,
			footprint_z,
			storey_height,
			corner_radius,
			corner_segments,
			openings,
			floor,
			ceiling,
		} => {
			let shell = RoundedRectFloor::new(RoundedRectFloorParams {
				center_xz: Vec3::ZERO,
				footprint: Vec2::new(*footprint_x, *footprint_z),
				storey_height: *storey_height,
				corner_radius: *corner_radius,
				corner_segments: *corner_segments,
				openings: openings_from_preview(openings),
				floor: if *floor {
					RoundedRectFloorSlab::Solid
				} else {
					RoundedRectFloorSlab::None
				},
				ceiling: if *ceiling {
					RoundedRectFloorSlab::Solid
				} else {
					RoundedRectFloorSlab::None
				},
				..RoundedRectFloorParams::default()
			});
			spawn_building_preview(&mut commands, transform, &shell, &lod_ref);
		}
		PreviewSubject::IFloor {
			central_x,
			central_z,
			storey_height,
			top_left,
			top_right,
			bottom_left,
			bottom_right,
			openings,
			floor,
			ceiling,
		} => {
			let shell = IFloor::new(IFloorParams {
				center_xz: Vec3::ZERO,
				top_left_length: *top_left,
				top_right_length: *top_right,
				central_rectangle: Vec2::new(*central_x, *central_z),
				bottom_left_length: *bottom_left,
				bottom_right_length: *bottom_right,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { IFloorSlab::Solid } else { IFloorSlab::None },
				ceiling: if *ceiling { IFloorSlab::Solid } else { IFloorSlab::None },
				..IFloorParams::default()
			});
			spawn_building_preview(&mut commands, transform, &shell, &lod_ref);
		}
		PreviewSubject::RectRingFloor {
			outer_x,
			outer_z,
			inner_x,
			inner_z,
			storey_height,
			openings,
			floor,
			ceiling,
		} => {
			let shell = RectRingFloor::new(RectRingFloorParams {
				center_xz: Vec3::ZERO,
				outer: Vec2::new(*outer_x, *outer_z),
				inner: Vec2::new(*inner_x, *inner_z),
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { RectRingFloorSlab::Solid } else { RectRingFloorSlab::None },
				ceiling: if *ceiling { RectRingFloorSlab::Solid } else { RectRingFloorSlab::None },
				..RectRingFloorParams::default()
			});
			spawn_building_preview(&mut commands, transform, &shell, &lod_ref);
		}
		PreviewSubject::CircRingFloor {
			outer_radius,
			inner_radius,
			storey_height,
			openings,
			floor,
			ceiling,
		} => {
			let shell = CircRingFloor::new(CircRingFloorParams {
				center_xz: Vec3::ZERO,
				outer_radius: *outer_radius,
				inner_radius: *inner_radius,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { CircRingFloorSlab::Solid } else { CircRingFloorSlab::None },
				ceiling: if *ceiling { CircRingFloorSlab::Solid } else { CircRingFloorSlab::None },
				..CircRingFloorParams::default()
			});
			spawn_building_preview(&mut commands, transform, &shell, &lod_ref);
		}
		PreviewSubject::RectangularPitchedRoofComplex {
			preset,
			overhang_fixed,
			overhang_ratio,
			end_cap_gable,
			gable_ridge,
			gable_eave,
			run_up,
			skylight,
		} => {
			let params = build_roof_complex_params(
				preset,
				*overhang_fixed,
				*overhang_ratio,
				*end_cap_gable,
				*gable_ridge,
				*gable_eave,
				*run_up,
				*skylight,
			);
			let shell = RectangularPitchedRoofComplex::new(params);
			spawn_building_preview(&mut commands, transform, &shell, &lod_ref);
		}
		PreviewSubject::Rectangle { origin, edge, height, thickness, roll } => {
			let rect = Rectangle::rough_stone(*origin, *edge, *height, *thickness, *roll);
			let corners = [rect.oriented.a0, rect.oriented.a1, rect.oriented.b0, rect.oriented.b1];
			spawn_building_preview(&mut commands, transform, &rect, &lod_ref);
			spawn_rectangle_debug_balls(
				&mut commands,
				&mut meshes,
				&mut materials,
				transform,
				corners,
			);
		}
		PreviewSubject::ClippedRectangle {
			origin,
			edge,
			height,
			thickness,
			roll,
			left,
			right,
			bottom,
			top,
		} => {
			let rect = ClippedRectangle::rough_stone(
				*origin,
				*edge,
				*height,
				*thickness,
				*roll,
				RectInset::new(*left, *right, *bottom, *top),
			);
			spawn_building_preview(&mut commands, transform, &rect, &lod_ref);
		}
		PreviewSubject::ClippedRectangularStrip { inset, min_dihedral, no_joint } => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			// Plan-turn fold so bay creases exceed the default dihedral threshold.
			let nodes = [
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 0.0), 2.5, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 2.0), 2.5, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(2.0, 0.0, 2.0), 2.5, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(2.0, 0.0, 4.0), 2.5, 0.75, 0.0),
			];
			let strip = ClippedRectangularStrip::from_nodes(
				building_components::panels::PanelStyle::RoughStonework,
				nodes,
				[None, Some(RectInset::uniform(*inset)), None],
			)
			.with_joint_policy(policy);
			spawn_building_preview(&mut commands, transform, &strip, &lod_ref);
		}
		PreviewSubject::FittedRectangle { a0, a1, b0, b1 } => {
			let rect = FittedRectangle::rough_stone(*a0, *a1, *b0, *b1);
			let corners = [rect.fitted.a0, rect.fitted.a1, rect.fitted.b0, rect.fitted.b1];
			spawn_building_preview(&mut commands, transform, &rect, &lod_ref);
			spawn_rectangle_debug_balls(
				&mut commands,
				&mut meshes,
				&mut materials,
				transform,
				corners,
			);
		}
		PreviewSubject::ClippedFittedRectangle { a0, a1, b0, b1, left, right, bottom, top } => {
			let rect = ClippedFittedRectangle::rough_stone(
				*a0,
				*a1,
				*b0,
				*b1,
				RectInset::new(*left, *right, *bottom, *top),
			);
			spawn_building_preview(&mut commands, transform, &rect, &lod_ref);
		}
		PreviewSubject::ClippedFittedRectangularStrip { inset, min_dihedral, no_joint } => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			// Folded two-rail strip (a along path, b offset; mid bay tips up).
			let rail_a = [
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(0.0, 0.0, 2.0),
				Vec3::new(0.0, 0.0, 4.0),
				Vec3::new(0.0, 0.0, 6.0),
			];
			let rail_b = [
				Vec3::new(2.5, 0.0, 0.0),
				Vec3::new(2.5, 0.0, 2.0),
				Vec3::new(2.5, 1.2, 4.0),
				Vec3::new(2.5, 1.2, 6.0),
			];
			let strip = ClippedFittedRectangularStrip::from_lines(
				building_components::panels::PanelStyle::RoughStonework,
				rail_a,
				rail_b,
				[None, Some(RectInset::uniform(*inset)), None],
			)
			.with_joint_policy(policy);
			spawn_building_preview(&mut commands, transform, &strip, &lod_ref);
		}
		PreviewSubject::RectangularNTube { inset, min_dihedral, no_joint, omit_faces } => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let t = DEFAULT_PANEL_THICKNESS;
			// Floor at y=0, ceiling at y=2 (not centered on the ground plane).
			let station = |z: f32| {
				RectangularNTubeStation::new([
					RectangularNTubeCorner::new(Vec3::new(-1.0, 0.0, z), t),
					RectangularNTubeCorner::new(Vec3::new(1.0, 0.0, z), t),
					RectangularNTubeCorner::new(Vec3::new(1.0, 2.0, z), t),
					RectangularNTubeCorner::new(Vec3::new(-1.0, 2.0, z), t),
				])
			};
			let tube = RectangularNTube::from_stations_with_insets(
				building_components::panels::PanelStyle::RoughStonework,
				[station(0.0), station(2.0), station(4.0), station(6.0)],
				[
					vec![None, None, None],
					vec![None, Some(RectInset::uniform(*inset)), None],
					vec![None, None, None],
					vec![None, None, None],
				],
			)
			.without_face_edges(omit_faces.iter().copied())
			.with_joint_policy(policy);
			spawn_building_preview(&mut commands, transform, &tube, &lod_ref);
		}
		PreviewSubject::ApproximatedCircle { center, radius, segments, clip } => {
			let disk = ApproximatedCircle::rough_stone(*center, *radius, *segments, *clip);
			spawn_building_preview(&mut commands, transform, &disk, &lod_ref);
		}
		PreviewSubject::ArcSweep { radius, height, sweep_degrees, start_yaw_deg } => {
			let sweep = ArcSweep::rough_stone(
				Vec3::ZERO,
				*radius,
				*height,
				*sweep_degrees,
				start_yaw_deg.to_radians(),
			);
			spawn_building_preview(&mut commands, transform, &sweep, &lod_ref);
		}
		PreviewSubject::ClippedArcSweep { radius, height, sweep_degrees, start_yaw_deg } => {
			let sweep = ClippedArcSweep::rough_stone(
				Vec3::ZERO,
				*radius,
				*height,
				*sweep_degrees,
				start_yaw_deg.to_radians(),
				[(0.2, 0.35), (0.6, 0.72)],
			);
			spawn_building_preview(&mut commands, transform, &sweep, &lod_ref);
		}
		PreviewSubject::QuadPanel {
			a0,
			a1,
			b0,
			b1,
			t_a0,
			t_a1,
			t_b0,
			t_b1,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let complex = QuadPanel::rough_stone(
				PanelPoint::new(*a0, *t_a0),
				PanelPoint::new(*a1, *t_a1),
				PanelPoint::new(*b0, *t_b0),
				PanelPoint::new(*b1, *t_b1),
			)
			.with_joint_policy(policy)
			.into_complex();
			spawn_building_preview(&mut commands, transform, &complex, &lod_ref);
		}
		PreviewSubject::PanelComplex { mesh, min_dihedral, no_joint } => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			match mesh.parse::<PanelComplex>() {
				Ok(complex) => {
					let complex = complex.with_joint_policy(policy);
					spawn_building_preview(&mut commands, transform, &complex, &lod_ref);
				}
				Err(e) => {
					warn!("panel-complex parse failed: {e}");
				}
			}
		}
		PreviewSubject::QuadPanelComplex { mesh, min_dihedral, no_joint } => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			match mesh.parse::<QuadPanelComplex>() {
				Ok(quads) => {
					let complex = quads.with_joint_policy(policy).into_complex();
					spawn_building_preview(&mut commands, transform, &complex, &lod_ref);
				}
				Err(e) => {
					warn!("quad-panel-complex parse failed: {e}");
				}
			}
		}
		PreviewSubject::RuledPitch { min_dihedral, no_joint } => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			// Funky 5+5: eave snakes on the ground; ridge wanders higher with a lag,
			// so rafters twist and bays pick up visible crease dihedrals.
			let eave = [
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(0.6, 0.15, 1.4),
				Vec3::new(-0.3, 0.0, 2.8),
				Vec3::new(0.9, 0.25, 4.1),
				Vec3::new(0.2, 0.0, 5.6),
			];
			let ridge = [
				Vec3::new(1.8, 1.6, 0.4),
				Vec3::new(2.6, 2.1, 1.1),
				Vec3::new(1.4, 1.4, 2.5),
				Vec3::new(2.9, 2.4, 3.6),
				Vec3::new(2.1, 1.7, 5.2),
			];
			let complex = RuledPitch::shepherds_thatch()
				.with_lines(eave, ridge)
				.with_joint_policy(policy)
				.into_complex();
			spawn_building_preview(&mut commands, transform, &complex, &lod_ref);
		}
		PreviewSubject::Polyline => {
			let node = PartitionNode::rough_stone(
				Partition::Polyline(
					building_components::partitions::PolylinePartition::new([
						Vec3::new(0.0, 0.0, 0.0),
						Vec3::new(4.0, 0.0, 0.0),
						Vec3::new(4.0, 0.0, 4.0),
					])
					.with_wall_scale(3.0, 1.0),
				),
				Placement::IDENTITY,
			);
			spawn_preview(&mut commands, transform, node.host(&lod_ref));
		}
		PreviewSubject::NoisyRectangularWall { .. } => {
			if let Some(wall) = cache.noisy_wall.as_ref() {
				spawn_building_preview(&mut commands, transform, wall, &lod_ref);
			}
		}
		PreviewSubject::WizardsTower { .. } => {
			if let Some(tower) = cache.wizards_tower.clone() {
				commands
					.spawn_scene((
						tower.scene_with_lod(&lod_ref),
						bsn! {
							template_value(transform)
							Visibility::default()
						},
					))
					.insert(PreviewRoot)
					.insert(tower);
			}
		}
		PreviewSubject::StackedRings { .. } => {
			if let Some(rings) = cache.stacked_rings.as_ref() {
				spawn_building_preview(&mut commands, transform, rings, &lod_ref);
			}
		}
		PreviewSubject::Bedroom { .. } => {
			if let Some(bedroom) = cache.bedroom.as_ref() {
				spawn_building_preview(&mut commands, transform, bedroom, &lod_ref);
			}
		}
		PreviewSubject::BedroomExamples => {
			for cell in &cache.bedroom_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.room, &lod_ref);
			}
		}
		PreviewSubject::ResidentialBathroom { .. } => {
			if let Some(room) = cache.residential_bathroom.as_ref() {
				spawn_building_preview(&mut commands, transform, room, &lod_ref);
			}
		}
		PreviewSubject::ResidentialHalfBathroom { .. } => {
			if let Some(room) = cache.residential_half_bathroom.as_ref() {
				spawn_building_preview(&mut commands, transform, room, &lod_ref);
			}
		}
		PreviewSubject::ResidentialBathroomExamples => {
			for cell in &cache.residential_bathroom_examples {
				let tf = transform
					* Transform::from_translation(match cell {
						ResidentialBathroomExampleCell::Full { offset, .. }
						| ResidentialBathroomExampleCell::Half { offset, .. } => *offset,
					});
				match cell {
					ResidentialBathroomExampleCell::Full { room, .. } => {
						spawn_building_preview(&mut commands, tf, room, &lod_ref);
					}
					ResidentialBathroomExampleCell::Half { room, .. } => {
						spawn_building_preview(&mut commands, tf, room, &lod_ref);
					}
				}
			}
		}
		PreviewSubject::KitchenExamples => {
			spawn_livable_quarters_gallery(
				&mut commands,
				transform,
				&cache.kitchen_examples,
				&lod_ref,
			);
		}
		PreviewSubject::DiningRoomExamples => {
			spawn_livable_quarters_gallery(
				&mut commands,
				transform,
				&cache.dining_room_examples,
				&lod_ref,
			);
		}
		PreviewSubject::LivingRoomExamples => {
			spawn_livable_quarters_gallery(
				&mut commands,
				transform,
				&cache.living_room_examples,
				&lod_ref,
			);
		}
		PreviewSubject::SittingRoomExamples => {
			spawn_livable_quarters_gallery(
				&mut commands,
				transform,
				&cache.sitting_room_examples,
				&lod_ref,
			);
		}
		PreviewSubject::StudyExamples => {
			spawn_livable_quarters_gallery(
				&mut commands,
				transform,
				&cache.study_examples,
				&lod_ref,
			);
		}
		PreviewSubject::CommercialStall { .. } => {
			if let Some(stall) = cache.commercial_stall.as_ref() {
				spawn_building_preview(&mut commands, transform, stall, &lod_ref);
			}
		}
		PreviewSubject::CommercialStallStrip { .. } => {
			if let Some(strip) = cache.commercial_stall_strip.as_ref() {
				spawn_building_preview(&mut commands, transform, strip, &lod_ref);
			}
		}
		PreviewSubject::BitesStall { .. } => {
			if let Some(stall) = cache.bites_stall.as_ref() {
				spawn_building_preview(&mut commands, transform, stall, &lod_ref);
			}
		}
		PreviewSubject::BitesSitdownStall { .. } => {
			if let Some(stall) = cache.bites_sitdown_stall.as_ref() {
				spawn_building_preview(&mut commands, transform, stall, &lod_ref);
			}
		}
		PreviewSubject::MiniMart { .. } => {
			if let Some(stall) = cache.mini_mart.as_ref() {
				spawn_building_preview(&mut commands, transform, stall, &lod_ref);
			}
		}
		PreviewSubject::MiniMartExamples => {
			for cell in &cache.mini_mart_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.stall, &lod_ref);
			}
		}
		PreviewSubject::PartsStall { .. } => {
			if let Some(stall) = cache.parts_stall.as_ref() {
				spawn_building_preview(&mut commands, transform, stall, &lod_ref);
			}
		}
		PreviewSubject::PartsExamples => {
			for cell in &cache.parts_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.stall, &lod_ref);
			}
		}
		PreviewSubject::KnickKnackStall { .. } => {
			if let Some(stall) = cache.knick_knack_stall.as_ref() {
				spawn_building_preview(&mut commands, transform, stall, &lod_ref);
			}
		}
		PreviewSubject::KnickKnackExamples => {
			for cell in &cache.knick_knack_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.stall, &lod_ref);
			}
		}
		PreviewSubject::PublicRestroom { .. } => {
			if let Some(stall) = cache.public_restroom.as_ref() {
				spawn_building_preview(&mut commands, transform, stall, &lod_ref);
			}
		}
		PreviewSubject::PublicRestroomExamples => {
			for cell in &cache.public_restroom_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.stall, &lod_ref);
			}
		}
		PreviewSubject::BitesExamples => {
			for cell in &cache.bites_examples {
				let local = match cell {
					BitesExampleCell::Stall { offset, .. }
					| BitesExampleCell::Sitdown { offset, .. } => Transform::from_translation(*offset),
				};
				let tf = transform * local;
				match cell {
					BitesExampleCell::Stall { stall, .. } => {
						spawn_building_preview(&mut commands, tf, stall, &lod_ref);
					}
					BitesExampleCell::Sitdown { stall, .. } => {
						spawn_building_preview(&mut commands, tf, stall, &lod_ref);
					}
				}
			}
		}
		PreviewSubject::LesHallesFloorPlan { .. } => {
			if let Some(plan) = cache.les_halles_floor_plan.as_ref() {
				spawn_building_preview(&mut commands, transform, plan, &lod_ref);
			}
		}
		PreviewSubject::LesHallesFloorPlanExamples => {
			for cell in &cache.les_halles_floor_plan_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.plan, &lod_ref);
			}
		}
		PreviewSubject::LesHallesFullStorey { .. } => {
			if let Some(storey) = cache.les_halles_full_storey.as_ref() {
				spawn_building_preview(&mut commands, transform, storey, &lod_ref);
			}
		}
		PreviewSubject::LesHallesLivableFullStorey { .. } => {
			if let Some(storey) = cache.les_halles_livable_full_storey.as_ref() {
				spawn_building_preview(&mut commands, transform, storey, &lod_ref);
			}
		}
		PreviewSubject::MixedUseLesHallesMonotower { .. } => {
			if let Some(tower) = cache.mixed_use_les_halles_monotower.as_ref() {
				spawn_building_preview(&mut commands, transform, tower, &lod_ref);
			}
		}
		PreviewSubject::MixedUseLesHallesDevelopment { .. } => {
			if let Some(dev) = cache.mixed_use_les_halles_development.as_ref() {
				for floor in &dev.tower.floors {
					spawn_building_preview(&mut commands, transform, floor, &lod_ref);
				}
				for stairwell in &dev.stairwells {
					spawn_building_preview(&mut commands, transform, stairwell, &lod_ref);
				}
				spawn_building_preview(&mut commands, transform, &dev.roof, &lod_ref);
			}
		}
		PreviewSubject::LesHallesLivableFullStoreyExamples => {
			for cell in &cache.les_halles_livable_full_storey_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.storey, &lod_ref);
			}
		}
		PreviewSubject::IApartmentFloorPlan { .. } => {
			if let Some(plan) = cache.i_apartment_floor_plan.as_ref() {
				spawn_building_preview(&mut commands, transform, plan, &lod_ref);
			}
		}
		PreviewSubject::IApartmentFloorPlanExamples => {
			for cell in &cache.i_apartment_floor_plan_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.plan, &lod_ref);
			}
		}
		PreviewSubject::IApartmentFullStorey { .. } => {
			if let Some(storey) = cache.i_apartment_full_storey.as_ref() {
				spawn_building_preview(&mut commands, transform, storey, &lod_ref);
			}
		}
		PreviewSubject::IApartmentFullStoreyExamples => {
			for cell in &cache.i_apartment_full_storey_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.storey, &lod_ref);
			}
		}
		PreviewSubject::LivableApartmentsExamples => {
			for cell in &cache.livable_apartments_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.block, &lod_ref);
			}
		}
		PreviewSubject::LivableApartmentExamples => {
			for cell in &cache.livable_apartment_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.apartment, &lod_ref);
			}
		}
		PreviewSubject::LivableRectanglesExamples => {
			for cell in &cache.livable_rectangles_examples {
				let tf = transform * Transform::from_translation(cell.offset);
				spawn_building_preview(&mut commands, tf, &cell.area, &lod_ref);
			}
		}
		PreviewSubject::HallsToShafts { .. } => {
			// Gizmo-only preview (no BuildingComponents on HallsToShafts).
		}
	}
}

fn spawn_livable_quarters_gallery<T>(
	commands: &mut Commands,
	transform: Transform,
	cells: &[GalleryExampleCell<T>],
	lod_ref: &lod::lod_ref::LodRef<'_>,
) where
	T: BuildingComponents + Clone + Send + Sync + 'static,
{
	for cell in cells {
		let tf = transform * Transform::from_translation(cell.offset);
		spawn_building_preview(commands, tf, &cell.room, lod_ref);
	}
}

fn spawn_connecting_stairwell(
	commands: &mut Commands,
	transform: Transform,
	case: crate::commands::show::connecting_stairwell::StairwellCase,
	tread_fill: f32,
	kind: crate::commands::show::connecting_stairwell::StairwellFit,
	lod_ref: &lod::lod_ref::LodRef<'_>,
) {
	for well in
		crate::commands::show::connecting_stairwell::preview_stairwells(case, tread_fill, kind)
	{
		spawn_building_preview(commands, transform, &well, lod_ref);
	}
}

fn spawn_building_preview<T>(
	commands: &mut Commands,
	transform: Transform,
	building: &T,
	lod_ref: &lod::lod_ref::LodRef<'_>,
) where
	T: BuildingComponents + Clone + Send + Sync + 'static,
{
	use building_components::{
		append_flattened_component_scenes, scene_children, spawn_building_components_for,
		ComponentsOnly,
	};
	use lod::scene::LodScene;
	if building.structural_lod().is_some() {
		let host = ComponentsOnly::posed(building.clone(), transform);
		let bounds = host.scene_bounds();
		for entity in
			spawn_building_components_for(commands, building, transform, bounds, Some(lod_ref))
		{
			commands.entity(entity).insert(PreviewRoot);
		}
	} else {
		let mut children: Vec<Box<dyn bevy::scene::Scene>> = Vec::new();
		append_flattened_component_scenes(
			building,
			lod_ref,
			lod::scene::LodSceneLevel::High,
			&mut children,
		);
		spawn_preview(commands, transform, scene_children(children));
	}
}

fn spawn_preview(commands: &mut Commands, transform: Transform, scene: impl bevy::scene::Scene) {
	commands
		.spawn_scene((
			scene,
			bsn! {
				template_value(transform)
				Visibility::default()
			},
		))
		.insert(PreviewRoot);
}

/// Demo openings: south facing +Z, east facing −X (mild kink near the origin).
pub fn connecting_hall_demo_endpoints() -> (MappedOpening, MappedOpening) {
	let end_a = MappedOpening::new(
		MappedOpeningQuad::new(
			Vec3::new(-1.2, 0.0, -4.0),
			Vec3::new(1.2, 0.0, -4.0),
			Vec3::new(-1.0, 2.4, -4.0),
			Vec3::new(1.0, 2.4, -4.0),
		),
		Vec2::Y,
	);
	// Looking along −X: left = −Z, right = +Z.
	let end_b = MappedOpening::new(
		MappedOpeningQuad::new(
			Vec3::new(4.0, 0.5, -1.2),
			Vec3::new(4.0, 0.5, 1.2),
			Vec3::new(4.0, 2.6, -1.0),
			Vec3::new(4.0, 2.6, 1.0),
		),
		-Vec2::X,
	);
	(end_a, end_b)
}

/// Wireframe plan AABBs (+ mapped contact quads) for `--opening` previews.
///
/// Color key:
/// - cyan / amber: authored plan [`Aabb3d`] voids (accepted)
/// - red: authored voids the model intentionally dropped
/// - lime: mapped outward opening quads (what connectors consume)
/// - orange arrows: mapped XZ orientation
/// - magenta: Les Halles fitted shaft volumes after inbound mapping
pub fn draw_opening_plan_gizmos(
	mut gizmos: Gizmos,
	config: Res<PreviewConfig>,
	cache: Res<CachedPreview>,
) {
	let tf = config.transform;
	let map = |p: Vec3| tf.transform_point(p);
	let cyan = Color::srgb(0.25, 0.95, 1.0);
	let amber = Color::srgb(1.0, 0.75, 0.2);
	let lime = Color::srgb(0.35, 0.95, 0.35);
	let orange = Color::srgb(1.0, 0.55, 0.15);
	let magenta = Color::srgb(0.95, 0.25, 0.85);

	match &config.subject {
		PreviewSubject::ArcFloor { radius, storey_height, floor, ceiling, openings } => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let floor_shell = ArcFloor::new(ArcFloorParams {
				center_xz: Vec3::ZERO,
				radius: *radius,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { ArcFloorSlab::Solid } else { ArcFloorSlab::None },
				ceiling: if *ceiling { ArcFloorSlab::Solid } else { ArcFloorSlab::None },
				..ArcFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &floor_shell, lime, orange);
		}
		PreviewSubject::Trazaloid {
			footprint_x,
			footprint_z,
			ridge_x,
			ridge_z,
			lower_height,
			upper_height,
			band_vertical_offset,
			waist_horizontal_offset,
			openings,
			floor,
			no_ceiling,
			face_post_count,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = Trazaloid::new(TrazaloidParams {
				footprint: Vec2::new(*footprint_x, *footprint_z),
				ridge: Vec2::new(*ridge_x, *ridge_z),
				lower_height: *lower_height,
				upper_height: *upper_height,
				band_vertical_offset: *band_vertical_offset,
				waist_horizontal_offset: *waist_horizontal_offset,
				openings: openings_from_preview(openings),
				floor: if *floor { TrazaloidSlab::Solid } else { TrazaloidSlab::None },
				ceiling: if *no_ceiling { TrazaloidSlab::None } else { TrazaloidSlab::Solid },
				face_post_count: *face_post_count,
				..TrazaloidParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::RectFloor {
			footprint_x,
			footprint_z,
			storey_height,
			openings,
			floor,
			ceiling,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = RectFloor::new(RectFloorParams {
				center_xz: Vec3::ZERO,
				footprint: Vec2::new(*footprint_x, *footprint_z),
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { RectFloorSlab::Solid } else { RectFloorSlab::None },
				ceiling: if *ceiling { RectFloorSlab::Solid } else { RectFloorSlab::None },
				..RectFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::RoundedRectFloor {
			footprint_x,
			footprint_z,
			storey_height,
			corner_radius,
			corner_segments,
			openings,
			floor,
			ceiling,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = RoundedRectFloor::new(RoundedRectFloorParams {
				center_xz: Vec3::ZERO,
				footprint: Vec2::new(*footprint_x, *footprint_z),
				storey_height: *storey_height,
				corner_radius: *corner_radius,
				corner_segments: *corner_segments,
				openings: openings_from_preview(openings),
				floor: if *floor {
					RoundedRectFloorSlab::Solid
				} else {
					RoundedRectFloorSlab::None
				},
				ceiling: if *ceiling {
					RoundedRectFloorSlab::Solid
				} else {
					RoundedRectFloorSlab::None
				},
				..RoundedRectFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::IFloor {
			central_x,
			central_z,
			storey_height,
			top_left,
			top_right,
			bottom_left,
			bottom_right,
			openings,
			floor,
			ceiling,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = IFloor::new(IFloorParams {
				center_xz: Vec3::ZERO,
				top_left_length: *top_left,
				top_right_length: *top_right,
				central_rectangle: Vec2::new(*central_x, *central_z),
				bottom_left_length: *bottom_left,
				bottom_right_length: *bottom_right,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { IFloorSlab::Solid } else { IFloorSlab::None },
				ceiling: if *ceiling { IFloorSlab::Solid } else { IFloorSlab::None },
				..IFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::PitchedRectangularRoof {
			footprint_x,
			footprint_z,
			ridge_height,
			eave_height,
			ridge_inset,
			gables,
			no_walls,
			no_hips,
			openings,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = pitched_roof_from_preview(
				*footprint_x,
				*footprint_z,
				*ridge_height,
				*eave_height,
				*ridge_inset,
				*gables,
				*no_walls,
				*no_hips,
				openings,
			);
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::RectRingFloor {
			outer_x,
			outer_z,
			inner_x,
			inner_z,
			storey_height,
			openings,
			floor,
			ceiling,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = RectRingFloor::new(RectRingFloorParams {
				center_xz: Vec3::ZERO,
				outer: Vec2::new(*outer_x, *outer_z),
				inner: Vec2::new(*inner_x, *inner_z),
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { RectRingFloorSlab::Solid } else { RectRingFloorSlab::None },
				ceiling: if *ceiling { RectRingFloorSlab::Solid } else { RectRingFloorSlab::None },
				..RectRingFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::CircRingFloor {
			outer_radius,
			inner_radius,
			storey_height,
			openings,
			floor,
			ceiling,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = CircRingFloor::new(CircRingFloorParams {
				center_xz: Vec3::ZERO,
				outer_radius: *outer_radius,
				inner_radius: *inner_radius,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor { CircRingFloorSlab::Solid } else { CircRingFloorSlab::None },
				ceiling: if *ceiling { CircRingFloorSlab::Solid } else { CircRingFloorSlab::None },
				..CircRingFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::LesHallesFloorPlan { openings, .. }
		| PreviewSubject::LesHallesFullStorey { openings, .. }
		| PreviewSubject::LesHallesLivableFullStorey { openings, .. }
		| PreviewSubject::MixedUseLesHallesMonotower { openings, .. }
		| PreviewSubject::MixedUseLesHallesDevelopment { openings, .. } => {
			let plan = cache
				.les_halles_floor_plan
				.as_ref()
				.or_else(|| cache.les_halles_full_storey.as_ref().map(|s| &s.floor_plan))
				.or_else(|| cache.les_halles_livable_full_storey.as_ref().map(|s| &s.floor_plan))
				.or_else(|| {
					cache
						.mixed_use_les_halles_monotower
						.as_ref()
						.and_then(|t| t.floors.first())
						.map(|f| f.floor_plan())
				})
				.or_else(|| {
					cache
						.mixed_use_les_halles_development
						.as_ref()
						.and_then(|d| d.tower.floors.first())
						.map(|f| f.floor_plan())
				});
			let red = Color::srgb(0.95, 0.2, 0.2);
			// Inbound preview openings (accepted → cyan/amber, rejected → red).
			for (i, opening) in openings.iter().enumerate() {
				let accepted =
					plan.map(|p| les_halles_opening_accepted(p, opening)).unwrap_or(false);
				let color = if !accepted {
					red
				} else if i % 2 == 0 {
					cyan
				} else {
					amber
				};
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			if let Some(plan) = plan {
				// Authored Passage voids (inner stall doors / shaft clears) — same
				// cyan/amber wire boxes as the commercial-stall demos.
				let mut passage_i = 0usize;
				for (_id, opening) in plan.openings.iter() {
					if !matches!(opening.label, OpeningLabel::Passage) {
						continue;
					}
					let color = if passage_i % 2 == 0 { cyan } else { amber };
					gizmos.aabb_3d(opening.bounds, tf, color);
					passage_i += 1;
				}
				for (i, shaft) in plan.shaft_bounds.iter().enumerate() {
					let color = if i % 2 == 0 { magenta } else { Color::srgb(0.75, 0.35, 1.0) };
					gizmos.aabb_3d(*shaft, tf, color);
				}
			}
		}
		PreviewSubject::LesHallesFloorPlanExamples => {
			let lime = Color::srgb(0.35, 0.9, 0.4);
			let strip_alt = Color::srgb(0.2, 0.7, 0.85);
			for cell in &cache.les_halles_floor_plan_examples {
				let cell_tf = tf * Transform::from_translation(cell.offset);
				let mut strip_i = 0usize;
				for region in cell.plan.fillable_regions().within {
					if region.kind != SpaceKind::ExternalSpace {
						continue;
					}
					let color = if strip_i % 2 == 0 { lime } else { strip_alt };
					gizmos.aabb_3d(region.confines.bounds, cell_tf, color);
					strip_i += 1;
				}
				for (i, shaft) in cell.plan.shaft_bounds.iter().enumerate() {
					let color = if i % 2 == 0 { magenta } else { Color::srgb(0.75, 0.35, 1.0) };
					gizmos.aabb_3d(*shaft, cell_tf, color);
				}
			}
		}
		PreviewSubject::LesHallesLivableFullStoreyExamples => {
			for cell in &cache.les_halles_livable_full_storey_examples {
				let cell_tf = tf * Transform::from_translation(cell.offset);
				for (i, shaft) in cell.storey.floor_plan.shaft_bounds.iter().enumerate() {
					let color = if i % 2 == 0 { magenta } else { Color::srgb(0.75, 0.35, 1.0) };
					gizmos.aabb_3d(*shaft, cell_tf, color);
				}
			}
		}
		PreviewSubject::IApartmentFloorPlan { openings, .. }
		| PreviewSubject::IApartmentFullStorey { openings, .. } => {
			let plan = cache
				.i_apartment_floor_plan
				.as_ref()
				.or_else(|| cache.i_apartment_full_storey.as_ref().map(|s| &s.floor_plan));
			for (i, opening) in openings.iter().enumerate() {
				let accepted =
					plan.map(|p| i_apartment_opening_accepted(p, opening)).unwrap_or(false);
				let color = if !accepted {
					Color::srgb(0.95, 0.2, 0.2)
				} else if i % 2 == 0 {
					cyan
				} else {
					amber
				};
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			if let Some(plan) = plan {
				draw_i_apartment_primary_rect_gizmos(&mut gizmos, plan, tf);
				for (i, shaft) in plan.shaft_bounds.iter().enumerate() {
					let color = if i % 2 == 0 { magenta } else { Color::srgb(0.75, 0.35, 1.0) };
					gizmos.aabb_3d(*shaft, tf, color);
				}
			}
		}
		PreviewSubject::IApartmentFloorPlanExamples => {
			for cell in &cache.i_apartment_floor_plan_examples {
				let cell_tf = tf * Transform::from_translation(cell.offset);
				draw_i_apartment_primary_rect_gizmos(&mut gizmos, &cell.plan, cell_tf);
				for (i, shaft) in cell.plan.shaft_bounds.iter().enumerate() {
					let color = if i % 2 == 0 { magenta } else { Color::srgb(0.75, 0.35, 1.0) };
					gizmos.aabb_3d(*shaft, cell_tf, color);
				}
			}
		}
		PreviewSubject::IApartmentFullStoreyExamples => {
			let lime = Color::srgb(0.35, 0.95, 0.4);
			let sky = Color::srgb(0.35, 0.7, 1.0);
			for cell in &cache.i_apartment_full_storey_examples {
				let cell_tf = tf * Transform::from_translation(cell.offset);
				draw_i_apartment_primary_rect_gizmos(&mut gizmos, &cell.storey.floor_plan, cell_tf);
				for (i, shaft) in cell.storey.floor_plan.shaft_bounds.iter().enumerate() {
					let color = if i % 2 == 0 { magenta } else { Color::srgb(0.75, 0.35, 1.0) };
					gizmos.aabb_3d(*shaft, cell_tf, color);
				}
				for (pi, (_id, opening)) in cell
					.storey
					.floor_plan
					.openings
					.iter()
					.filter(|(_, o)| matches!(o.label, OpeningLabel::Passage))
					.enumerate()
				{
					let color = if pi % 2 == 0 { cyan } else { amber };
					gizmos.aabb_3d(opening.bounds, cell_tf, color);
				}
				for block in &cell.storey.blocks {
					for (hi, band) in block.halls.hall_bands.iter().enumerate() {
						let y0 = Vec3::from(block.confines.bounds.min).y;
						let y1 = Vec3::from(block.confines.bounds.max).y;
						let bounds = Aabb3d::from_min_max(
							Vec3::new(band.min.x, y0, band.min.y),
							Vec3::new(band.max.x, y1, band.max.y),
						);
						let color = if hi % 2 == 0 { lime } else { Color::srgb(0.2, 0.8, 0.55) };
						gizmos.aabb_3d(bounds, cell_tf, color);
					}
					for apt in &block.apartments {
						for part in apt.cells.iter() {
							gizmos.aabb_3d(part.confines.bounds, cell_tf, sky);
						}
					}
				}
			}
		}
		PreviewSubject::LivableApartmentsExamples => {
			for cell in &cache.livable_apartments_examples {
				let cell_tf = tf * Transform::from_translation(cell.offset);
				draw_livable_apartments_block_gizmos(
					&mut gizmos,
					&cell.block,
					cell_tf,
					cyan,
					amber,
					magenta,
				);
			}
		}
		PreviewSubject::LivableApartmentExamples => {
			for cell in &cache.livable_apartment_examples {
				let cell_tf = tf * Transform::from_translation(cell.offset);
				draw_livable_apartment_gizmos(&mut gizmos, &cell.apartment, cell_tf);
			}
		}
		PreviewSubject::LivableRectanglesExamples => {
			for cell in &cache.livable_rectangles_examples {
				let cell_tf = tf * Transform::from_translation(cell.offset);
				draw_livable_rectangle_gizmos(&mut gizmos, &cell.area, cell_tf);
			}
		}
		PreviewSubject::Bedroom { .. }
		| PreviewSubject::BedroomExamples
		| PreviewSubject::ResidentialBathroom { .. }
		| PreviewSubject::ResidentialHalfBathroom { .. }
		| PreviewSubject::ResidentialBathroomExamples
		| PreviewSubject::KitchenExamples
		| PreviewSubject::DiningRoomExamples
		| PreviewSubject::LivingRoomExamples
		| PreviewSubject::SittingRoomExamples
		| PreviewSubject::StudyExamples
		| PreviewSubject::BitesStall { .. }
		| PreviewSubject::BitesSitdownStall { .. }
		| PreviewSubject::BitesExamples
		| PreviewSubject::MiniMart { .. }
		| PreviewSubject::MiniMartExamples
		| PreviewSubject::PartsStall { .. }
		| PreviewSubject::PartsExamples
		| PreviewSubject::KnickKnackStall { .. }
		| PreviewSubject::KnickKnackExamples
		| PreviewSubject::PublicRestroom { .. }
		| PreviewSubject::PublicRestroomExamples => {
			// Cyan / amber wire Passage voids for stall demos.
			for (i, (bounds, offset)) in cache.bites_passages.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				let cell_tf = tf * Transform::from_translation(*offset);
				gizmos.aabb_3d(*bounds, cell_tf, color);
			}
		}
		PreviewSubject::HallsToShafts { openings, .. } => {
			draw_halls_to_shafts_gizmos(&mut gizmos, &cache, tf, openings, cyan, amber, magenta);
		}
		_ => {}
	}
}

/// Halls (lime), shafts (magenta), passages (cyan/amber), residuals (sky).
fn draw_halls_to_shafts_gizmos(
	gizmos: &mut Gizmos,
	cache: &CachedPreview,
	tf: Transform,
	openings: &[PreviewOpening],
	cyan: Color,
	amber: Color,
	magenta: Color,
) {
	let lime = Color::srgb(0.35, 0.95, 0.4);
	let sky = Color::srgb(0.35, 0.7, 1.0);
	let host_color = Color::srgb(0.75, 0.75, 0.8);

	let Some(preview) = cache.halls_to_shafts.as_ref() else {
		// Fallback: still show authored openings if fit failed.
		for (i, opening) in openings.iter().enumerate() {
			let color = match opening.label {
				OpeningLabel::Shaft => magenta,
				OpeningLabel::Passage => {
					if i % 2 == 0 {
						cyan
					} else {
						amber
					}
				}
				_ => Color::srgb(0.9, 0.9, 0.9),
			};
			gizmos.aabb_3d(opening.bounds(), tf, color);
		}
		return;
	};

	gizmos.aabb_3d(preview.host, tf, host_color);

	for (i, band) in preview.fit.hall_bands.iter().enumerate() {
		let y0 = Vec3::from(preview.host.min).y;
		let y1 = Vec3::from(preview.host.max).y;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(band.min.x, y0, band.min.y),
			Vec3::new(band.max.x, y1, band.max.y),
		);
		let color = if i % 2 == 0 { lime } else { Color::srgb(0.2, 0.8, 0.55) };
		gizmos.aabb_3d(bounds, tf, color);
	}

	for region in &preview.regions.within {
		if region.kind != SpaceKind::InternalSpace {
			continue;
		}
		gizmos.aabb_3d(region.confines.bounds, tf, sky);
	}

	// Prefer fitted openings on the HallsToShafts confines (post-sync AABBs).
	let mut passage_i = 0usize;
	let mut shaft_i = 0usize;
	for (_id, opening) in preview.fit.confines.openings.iter() {
		match opening.label {
			OpeningLabel::Shaft => {
				let color = if shaft_i % 2 == 0 { magenta } else { Color::srgb(0.75, 0.35, 1.0) };
				gizmos.aabb_3d(opening.bounds, tf, color);
				shaft_i += 1;
			}
			OpeningLabel::Passage => {
				let color = if passage_i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds, tf, color);
				passage_i += 1;
			}
			_ => {}
		}
	}
}

/// Whether an inbound Les Halles opening survived mapping onto the floor plan.
fn les_halles_opening_accepted(plan: &LesHallesFloorPlan, opening: &PreviewOpening) -> bool {
	let id = OpeningId::new(opening.id.clone());
	match opening.label {
		OpeningLabel::Shaft => plan.shaft_inbound.iter().any(|ids| ids.contains(&id)),
		_ => plan.gallery.mapped_opening(&id).is_some(),
	}
}

/// Whether an inbound I-Apartment opening survived mapping onto the floor plan.
fn i_apartment_opening_accepted(plan: &IApartmentFloorPlan, opening: &PreviewOpening) -> bool {
	let id = OpeningId::new(opening.id.clone());
	match opening.label {
		OpeningLabel::Shaft => plan.shaft_inbound.iter().any(|ids| ids.contains(&id)),
		_ => plan.shell.mapped_opening(&id).is_some() || plan.openings.get(&id).is_some(),
	}
}

/// Stroke-font face labels for [`LabelNode`]s (toggle via [`PreviewConfig::label_text`]).
///
/// Text is word-wrapped and scaled so the block fits inside each face.
pub fn draw_label_text_gizmos(
	mut gizmos: Gizmos,
	config: Res<PreviewConfig>,
	cache: Res<CachedPreview>,
) {
	if !config.label_text {
		return;
	}
	let labels = cache.label_nodes();
	if labels.is_empty() {
		return;
	}
	let root = config.transform;
	for label in &labels {
		// Placement scale is full extents; face offsets are in unit-cube local space.
		let local = pose(label.placement);
		let tf = root * local;
		let extents = label.geometry.extents();
		// (local face center, rotation, face width, face height) in world meters.
		// Local face offset, outward-facing rotation (text reads from outside), face size.
		// Top/bottom previously used the inward ±X rotations, so top text faced into the volume
		// (and looked mirrored from above). Flip those so both horizontals face outward.
		let faces = [
			(
				Vec3::new(0.5, 0.0, 0.0),
				Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
				extents.z,
				extents.y,
			),
			(
				Vec3::new(-0.5, 0.0, 0.0),
				Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
				extents.z,
				extents.y,
			),
			(
				Vec3::new(0.0, 0.5, 0.0),
				Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
					* Quat::from_rotation_z(std::f32::consts::PI),
				extents.x,
				extents.z,
			),
			(
				Vec3::new(0.0, -0.5, 0.0),
				Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
				extents.x,
				extents.z,
			),
			(Vec3::new(0.0, 0.0, 0.5), Quat::IDENTITY, extents.x, extents.y),
			(
				Vec3::new(0.0, 0.0, -0.5),
				Quat::from_rotation_y(std::f32::consts::PI),
				extents.x,
				extents.y,
			),
		];
		let color = label.style.color();
		for (offset, rot, face_w, face_h) in faces {
			let (wrapped, font_size) = fit_label_face_text(&label.text, face_w, face_h);
			let world = tf.transform_point(offset * 1.01);
			let iso = Isometry3d::new(world, tf.rotation * rot);
			gizmos.text(iso, &wrapped, font_size, Vec2::ZERO, color);
		}
	}
}

/// Word-wrap + shrink stroke font so the block fits inside `face_w` × `face_h` (meters).
fn fit_label_face_text(text: &str, face_w: f32, face_h: f32) -> (String, f32) {
	let max_w = (face_w * 0.72).max(0.04);
	let max_h = (face_h * 0.72).max(0.04);
	// Stroke font size is in world units (cap height ≈ font_size).
	// Keep labels compact so multi-word names stay readable inside thin AABBs.
	let mut font = (face_w.min(face_h) * 0.09).clamp(0.03, 0.22);
	let mut wrapped = wrap_label_text(text, max_w, font);
	for _ in 0..16 {
		wrapped = wrap_label_text(text, max_w, font);
		let (w, h) = measure_wrapped_label(&wrapped, font);
		if w <= max_w && h <= max_h {
			break;
		}
		font = (font * 0.82).max(0.025);
	}
	(wrapped, font)
}

fn wrap_label_text(text: &str, max_w: f32, font_size: f32) -> String {
	let char_w = (font_size * 0.55).max(1e-4);
	let max_chars = ((max_w / char_w).floor() as usize).max(1);
	let mut lines = Vec::new();
	let mut line = String::new();
	for word in text.split_whitespace() {
		if word.len() > max_chars {
			if !line.is_empty() {
				lines.push(std::mem::take(&mut line));
			}
			let mut rest = word;
			while rest.len() > max_chars {
				lines.push(rest[..max_chars].to_string());
				rest = &rest[max_chars..];
			}
			line = rest.to_string();
			continue;
		}
		if line.is_empty() {
			line.push_str(word);
		} else if line.len() + 1 + word.len() <= max_chars {
			line.push(' ');
			line.push_str(word);
		} else {
			lines.push(std::mem::take(&mut line));
			line.push_str(word);
		}
	}
	if !line.is_empty() {
		lines.push(line);
	}
	if lines.is_empty() {
		text.to_string()
	} else {
		lines.join("\n")
	}
}

fn measure_wrapped_label(wrapped: &str, font_size: f32) -> (f32, f32) {
	let char_w = font_size * 0.55;
	let line_h = font_size * 1.25;
	let mut width = 0.0_f32;
	let mut lines = 0usize;
	for line in wrapped.lines() {
		width = width.max(line.len() as f32 * char_w);
		lines += 1;
	}
	(width, lines.max(1) as f32 * line_h)
}

/// Massing AABBs (cyan) + valley segments (magenta) for roof-complex previews.
pub fn draw_roof_complex_gizmos(mut gizmos: Gizmos, config: Res<PreviewConfig>) {
	let PreviewSubject::RectangularPitchedRoofComplex {
		preset,
		overhang_fixed,
		overhang_ratio,
		end_cap_gable,
		gable_ridge,
		gable_eave,
		run_up,
		skylight,
	} = &config.subject
	else {
		return;
	};
	let tf = config.transform;
	let map = |p: Vec3| tf.transform_point(p);
	let cyan = Color::srgb(0.25, 0.95, 1.0);
	let magenta = Color::srgb(0.95, 0.25, 0.85);

	let params = build_roof_complex_params(
		preset,
		*overhang_fixed,
		*overhang_ratio,
		*end_cap_gable,
		*gable_ridge,
		*gable_eave,
		*run_up,
		*skylight,
	);
	for (i, vol) in params.volumes.iter().enumerate() {
		let color = if i % 2 == 0 { cyan } else { Color::srgb(1.0, 0.75, 0.2) };
		gizmos.aabb_3d(*vol, tf, color);
	}
	let shell = RectangularPitchedRoofComplex::new(params);
	for valley in shell.valleys() {
		gizmos.line(map(valley.eave_point), map(valley.ridge_point), magenta);
		gizmos.sphere(Isometry3d::from_translation(map(valley.eave_point)), 0.1, magenta);
		gizmos.sphere(Isometry3d::from_translation(map(valley.ridge_point)), 0.1, magenta);
	}
}

/// Rebuild a pitched roof from preview knobs (matches `/show pitched-rectangular-roof`).
fn pitched_roof_from_preview(
	footprint_x: f32,
	footprint_z: f32,
	ridge_height: f32,
	eave_height: f32,
	ridge_inset: f32,
	gables: bool,
	no_walls: bool,
	no_hips: bool,
	openings: &[PreviewOpening],
) -> PitchedRoof {
	let footprint = Vec2::new(footprint_x, footprint_z);
	let mut params = if no_hips && gables && ridge_inset <= 1e-4 {
		PitchedRoofParams::rectangular_gable(footprint, ridge_height, eave_height)
	} else {
		PitchedRoofParams::rectangular_hip(footprint, ridge_height, eave_height, ridge_inset)
	};
	for half in &mut params.halves {
		half.draw_in_wall_line = !no_walls;
		half.draw_in_half_hip = if no_hips { (false, false) } else { (true, true) };
		half.draw_in_half_gable_end = if gables { (true, true) } else { (false, false) };
	}
	params.openings = openings_from_preview(openings);
	PitchedRoof::new(params)
}

fn draw_mapped_opening_overlays<M: MapsOpenings>(
	gizmos: &mut Gizmos,
	map: impl Fn(Vec3) -> Vec3 + Copy,
	openings: &[PreviewOpening],
	shell: &M,
	lime: Color,
	orange: Color,
) {
	for opening in openings {
		let id = OpeningId::new(opening.id.clone());
		let Some(mapped) = shell.mapped_opening(&id) else {
			continue;
		};
		draw_opening_gizmos(gizmos, map, *mapped, lime);
		let (bl, br, ..) = mapped.endpoint_corners();
		let mid = (bl + br) * 0.5;
		let dir = Vec3::new(mapped.orientation.x, 0.0, mapped.orientation.y).normalize_or_zero();
		gizmos.arrow(map(mid), map(mid + dir * 1.25), orange).with_tip_length(0.2);
	}
}

/// Exclusive well box (cyan), walk-on (lime), walk-off (orange), landing (yellow),
/// last leading (magenta).
pub fn draw_connecting_stairwell_gizmos(mut gizmos: Gizmos, config: Res<PreviewConfig>) {
	let cells = match &config.subject {
		PreviewSubject::ConnectingStairwell { case, tread_fill, kind } => {
			vec![(
				Vec3::ZERO,
				crate::commands::show::connecting_stairwell::preview_stairwells(
					*case,
					*tread_fill,
					*kind,
				),
			)]
		}
		PreviewSubject::ConnectingStairwellExamples { kind } => {
			crate::commands::show::connecting_stairwell::pathological_gallery(*kind)
				.into_iter()
				.map(|c| (c.offset, c.stairwells))
				.collect()
		}
		_ => return,
	};
	let root = config.transform;
	for (offset, wells) in cells {
		let tf = root * Transform::from_translation(offset);
		for well in wells {
			draw_stairwell_well_gizmos(&mut gizmos, tf, &well);
		}
	}
}

fn draw_stairwell_well_gizmos(
	gizmos: &mut Gizmos,
	tf: Transform,
	well: &buildings::ConnectingStairwell,
) {
	let cyan = Color::srgb(0.2, 0.9, 0.95);
	let lime = Color::srgb(0.35, 0.95, 0.35);
	let orange = Color::srgb(1.0, 0.55, 0.15);
	let yellow = Color::srgb(1.0, 0.9, 0.2);
	let magenta = Color::srgb(0.95, 0.25, 0.85);
	let aabb = well.well();
	gizmos.aabb_3d(aabb.bounds, tf, cyan);
	draw_well_side_gizmo(gizmos, tf, aabb, aabb.walk_on, aabb.bottom_y(), lime);
	draw_well_side_gizmo(gizmos, tf, aabb, aabb.walk_off, aabb.top_y(), orange);
	for landing in well.mid_landings().iter().chain(well.upper_landing()) {
		let [a0, a1, b0, b1] = landing.corners();
		for (p, q) in [(a0, a1), (a1, b1), (b1, b0), (b0, a0)] {
			gizmos.line(tf.transform_point(p), tf.transform_point(q), yellow);
		}
	}
	if let Some(end) = well.last_tread_end() {
		let y = well
			.stairs()
			.last()
			.map(|s| s.placement.translation.y)
			.unwrap_or_else(|| aabb.top_y());
		let [p0, p1, p2, p3] = end.plan_quad();
		for (a, b) in [(p0, p1), (p1, p2), (p2, p3), (p3, p0)] {
			gizmos.line(
				tf.transform_point(Vec3::new(a.x, y, a.y)),
				tf.transform_point(Vec3::new(b.x, y, b.y)),
				magenta,
			);
		}
	}
}

fn draw_well_side_gizmo(
	gizmos: &mut Gizmos,
	tf: Transform,
	aabb: WellAabb,
	side: WellSide,
	y: f32,
	color: Color,
) {
	let along = match side {
		WellSide::NegX | WellSide::PosX => aabb.half_z(),
		WellSide::NegZ | WellSide::PosZ => aabb.half_x(),
	};
	let [a0, a1, ..] = aabb.side_strip(side, y, 0.05, along);
	gizmos.line(tf.transform_point(a0), tf.transform_point(a1), color);
}

/// Debug overlay for [`PreviewSubject::ConnectingHall`]: opening corners, orientation
/// arrows, path A→mid→B, and station dots.
pub fn draw_connecting_hall_gizmos(mut gizmos: Gizmos, config: Res<PreviewConfig>) {
	if !matches!(config.subject, PreviewSubject::ConnectingHall) {
		return;
	}
	let tf = config.transform;
	let map = |p: Vec3| tf.transform_point(p);

	let (end_a, end_b) = connecting_hall_demo_endpoints();
	let hall = ConnectingHall::rough_stone(end_a, end_b);
	let stations = hall.stations();
	let mid = hall.midpoint();

	let cyan = Color::srgb(0.2, 0.9, 0.95);
	let magenta = Color::srgb(0.95, 0.25, 0.85);
	let lime = Color::srgb(0.35, 0.95, 0.35);
	let orange = Color::srgb(1.0, 0.55, 0.15);
	let yellow = Color::srgb(1.0, 0.9, 0.2);
	let white = Color::srgb(0.95, 0.95, 0.95);

	draw_opening_gizmos(&mut gizmos, map, end_a, cyan);
	draw_opening_gizmos(&mut gizmos, map, end_b, magenta);

	// Path A → mid → B
	let a_c = stations[0].bottom_middle;
	let b_c = stations[2].bottom_middle;
	gizmos.line(map(a_c), map(mid), white);
	gizmos.line(map(mid), map(b_c), white);

	// Station / junction dots
	let r = 0.12;
	gizmos.sphere(Isometry3d::from_translation(map(a_c)), r, lime);
	gizmos.sphere(Isometry3d::from_translation(map(mid)), r * 1.25, yellow);
	gizmos.sphere(Isometry3d::from_translation(map(b_c)), r, orange);

	// Orientation arrows from opening centers (plan facing).
	let arrow_len = 1.5;
	let a_dir = Vec3::new(end_a.orientation.x, 0.0, end_a.orientation.y).normalize_or_zero();
	let b_dir = Vec3::new(end_b.orientation.x, 0.0, end_b.orientation.y).normalize_or_zero();
	gizmos.arrow(map(a_c), map(a_c + a_dir * arrow_len), lime).with_tip_length(0.25);
	gizmos
		.arrow(map(b_c), map(b_c + b_dir * arrow_len), orange)
		.with_tip_length(0.25);
}

fn draw_opening_gizmos(
	gizmos: &mut Gizmos,
	map: impl Fn(Vec3) -> Vec3,
	end: MappedOpening,
	color: Color,
) {
	let (bl, br, tl, tr) = end.endpoint_corners();
	let r = 0.08;
	for p in [bl, br, tl, tr] {
		gizmos.sphere(Isometry3d::from_translation(map(p)), r, color);
	}
	// Opening quad wireframe
	gizmos.line(map(bl), map(br), color);
	gizmos.line(map(br), map(tr), color);
	gizmos.line(map(tr), map(tl), color);
	gizmos.line(map(tl), map(bl), color);
}

/// Debug overlay for [`PreviewSubject::ConnectingShells`].
///
/// Color key (tower side):
/// - white: 15° grid after +7.5° clockwise bias (should sit on jambs / edges)
/// - lime: visible door = one segment (`t−30°` → `t−15°`), on-arc pre-widen
/// - orange: opening mid (`t−22.5°`)
/// - cyan: widened tower hall opening
/// - magenta: trazaloid hall opening
pub fn draw_connecting_shells_gizmos(mut gizmos: Gizmos, config: Res<PreviewConfig>) {
	if !matches!(config.subject, PreviewSubject::ConnectingShells) {
		return;
	}
	let tf = config.transform;
	let map = |p: Vec3| tf.transform_point(p);

	let demo = ConnectingShells::new();
	let floor = demo.tower().storey(0).expect("ground storey");
	let connect = OpeningId::new("connect");
	let end_tower_raw = floor.mapped_opening(&connect).expect("ground door").clone();
	let (end_tower_wide, end_traz) = demo.hall().endpoints();
	let door_t = 0.5;
	let seg = floor.segment_t();
	let c = floor.params().center_xz;
	let y = c.y + 0.15;

	let white = Color::srgb(0.95, 0.95, 0.95);
	let lime = Color::srgb(0.35, 0.95, 0.35);
	let cyan = Color::srgb(0.2, 0.9, 0.95);
	let magenta = Color::srgb(0.95, 0.25, 0.85);
	let orange = Color::srgb(1.0, 0.55, 0.15);

	for i in 0..24 {
		let t = i as f32 / 24.0;
		let p = floor.ring_point_at(t);
		gizmos.sphere(Isometry3d::from_translation(map(Vec3::new(p.x, y, p.z))), 0.06, white);
	}

	// One 15° segment clockwise of portal t (decreasing t).
	let j_lo = floor.ring_point_at(door_t - 2.0 * seg);
	let j_hi = floor.ring_point_at(door_t - seg);
	let j_mid = floor.ring_point_at(door_t - 1.5 * seg);
	for p in [j_lo, j_hi] {
		gizmos.sphere(Isometry3d::from_translation(map(Vec3::new(p.x, y, p.z))), 0.14, lime);
	}
	gizmos.line(map(Vec3::new(j_lo.x, y, j_lo.z)), map(Vec3::new(j_hi.x, y, j_hi.z)), lime);
	gizmos.sphere(Isometry3d::from_translation(map(Vec3::new(j_mid.x, y, j_mid.z))), 0.1, orange);

	draw_opening_gizmos(&mut gizmos, map, end_tower_raw, lime);
	draw_opening_gizmos(&mut gizmos, map, end_tower_wide, cyan);
	draw_opening_gizmos(&mut gizmos, map, end_traz, magenta);

	let (bl, br, ..) = end_tower_wide.endpoint_corners();
	let mid = (bl + br) * 0.5;
	let dir = Vec3::new(end_tower_wide.orientation.x, 0.0, end_tower_wide.orientation.y)
		.normalize_or_zero();
	gizmos.arrow(map(mid), map(mid + dir * 2.0), cyan).with_tip_length(0.3);
}

/// Authored bay corners as colored spheres (a0 red, a1 orange, b0 green, b1 cyan).
fn spawn_rectangle_debug_balls(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	parent: Transform,
	corners: [Vec3; 4],
) {
	let mesh = meshes.add(Sphere::new(0.12));
	let colors = [
		Color::srgb(0.95, 0.2, 0.15),
		Color::srgb(0.95, 0.55, 0.1),
		Color::srgb(0.2, 0.85, 0.25),
		Color::srgb(0.15, 0.75, 0.95),
	];
	for (p, color) in corners.into_iter().zip(colors) {
		let material =
			materials.add(StandardMaterial { base_color: color, unlit: true, ..default() });
		commands.spawn((
			Mesh3d(mesh.clone()),
			MeshMaterial3d(material),
			parent * Transform::from_translation(p),
			Visibility::default(),
			PreviewRoot,
		));
	}
}
