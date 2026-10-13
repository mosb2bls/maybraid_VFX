//! `/show` subcommand: preview a partition leaf or authored building.

pub mod approximated_circle;
pub mod arc_180;
pub mod arc_90;
pub mod arc_floor;
pub mod arc_sweep;
pub mod arc_tower;
pub mod bedroom;
pub mod bedroom_examples;
pub mod bites_examples;
pub mod bites_sitdown_stall;
pub mod bites_stall;
pub mod circ_ring_floor;
pub mod clipped_arc_sweep;
pub mod clipped_fitted_rectangle;
pub mod clipped_fitted_rectangular_strip;
pub mod clipped_quad_panel;
pub mod clipped_rectangle;
pub mod clipped_rectangular_strip;
pub mod clipped_ruled_strip;
pub mod clipped_tessellated_triangle;
pub mod commercial_stall;
pub mod commercial_stall_strip;
pub mod connecting_hall;
pub mod connecting_shells;
pub mod connecting_stairwell;
pub mod connecting_stairwell_examples;
pub mod dining_room_examples;
pub mod fitted_rectangle;
pub mod halls_to_shafts;
pub mod i_apartment_floor_plan;
pub mod i_apartment_floor_plan_examples;
pub mod i_apartment_full_storey;
pub mod i_apartment_full_storey_examples;
pub mod i_floor;
pub mod kitchen_examples;
pub mod knick_knack_examples;
pub mod knick_knack_stall;
pub mod les_halles_floor_plan;
pub mod les_halles_floor_plan_examples;
pub mod les_halles_full_storey;
pub mod les_halles_livable_full_storey;
pub mod les_halles_livable_full_storey_examples;
pub mod linear;
pub mod livable_apartment_examples;
pub mod livable_apartments_examples;
pub mod livable_rectangles_examples;
pub mod living_room_examples;
pub mod mini_mart;
pub mod mini_mart_examples;
pub mod mixed_use_les_halles_development;
pub mod mixed_use_les_halles_monotower;
pub mod noisy_rectangular_wall;
pub mod opening;
pub mod panel_complex;
pub mod parts_examples;
pub mod parts_stall;
pub mod pitch;
pub mod pitched_rectangular_roof;
pub mod polyline;
pub mod public_restroom;
pub mod public_restroom_examples;
pub mod quad_panel;
pub mod quad_panel_complex;
pub mod rect_floor;
pub mod rect_ring_floor;
pub mod rectangle;
pub mod rectangular_n_tube;
pub mod rectangular_pitched_roof_complex;
pub mod residential_bathroom;
pub mod residential_bathroom_examples;
pub mod residential_half_bathroom;
pub mod rounded_rect_floor;
pub mod ruled_pitch;
pub mod sitting_room_examples;
pub mod slice_90;
pub mod stacked_rings;
pub mod study_examples;
pub mod tessellated_triangle;
pub mod tessellated_triangle_3d;
pub mod transform;
pub mod trazaloid;
pub mod tube;
pub mod wizards_tower;

use bevy::prelude::*;
use clap::Subcommand;
use game_commands::ui::GameCommandStatusText;

pub use transform::ShowTransform;

use crate::preview::PreviewConfig;

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Straight rough-stonework linear segment (`panels/.../rectangle_001_{high,mid,low}_res.glb`).
	Linear(linear::Linear),
	/// 90° rough-stonework arc (`arcs/.../arc_90_001.glb`).
	Arc90(arc_90::Arc90),
	/// 180° rough-stonework arc (`arcs/.../arc_180_001.glb`).
	Arc180(arc_180::Arc180),
	/// 90° slice rough-stonework (`arcs/.../arc_90_slice_001.glb`).
	Slice90(slice_90::Slice90),
	/// Shepherd's-thatch pitched face (`rise`/`run`/`length`/`left`/`right`).
	Pitch(pitch::Pitch),
	/// Arbitrary panel-space triangle filled with right-triangle kits.
	TessellatedTriangle(tessellated_triangle::TessellatedTriangle),
	/// World-space triangle → 2D panel tessellation → posed onto the plane.
	TessellatedTriangle3d(tessellated_triangle_3d::TessellatedTriangle3d),
	/// World-space triangle minus a closed clip (may extend outside) → [`PanelComplex`].
	ClippedTessellatedTriangle(clipped_tessellated_triangle::ClippedTessellatedTriangle),
	/// Ruled quad minus a closed clip → [`PanelComplex`].
	ClippedQuadPanel(clipped_quad_panel::ClippedQuadPanel),
	/// Multi-bay ruled strip with optional per-bay clips.
	ClippedRuledStrip(clipped_ruled_strip::ClippedRuledStrip),
	/// Trapezoid cross-section polyline → four clipped ruled strip faces.
	Tube(tube::Tube),
	/// One-kink tube between two oriented openings.
	ConnectingHall(connecting_hall::ConnectingHall),
	/// Exclusive AABB stairwell (circular, rectangular, or run-and-landing).
	ConnectingStairwell(connecting_stairwell::ConnectingStairwell),
	/// Gallery of pathological exclusive-well stairwells.
	ConnectingStairwellExamples(connecting_stairwell_examples::ConnectingStairwellExamples),
	/// One circular storey shell with optional `--opening` plan entries.
	ArcFloor(arc_floor::ArcFloor),
	/// Stacked circular storey shell (explicit openings; no noise).
	ArcTower(arc_tower::ArcTower),
	/// ArcTower joined to a Trazaloid via a ConnectingHall.
	ConnectingShells(connecting_shells::ConnectingShells),
	/// Two-band trapezoidal-pyramid shell with waist reveal and optional `--opening`s.
	Trazaloid(trazaloid::Trazaloid),
	/// Two-half pitched roof (default: rectangular hip over shepherd's thatch).
	PitchedRectangularRoof(pitched_rectangular_roof::PitchedRectangularRoof),
	/// Orthogonal AABB pitched-roof complex with L/T valleys.
	RectangularPitchedRoofComplex(rectangular_pitched_roof_complex::RectangularPitchedRoofComplex),
	/// Orthonormal rectangular storey shell with optional `--opening` / `--door-*`.
	RectFloor(rect_floor::RectFloor),
	/// Rounded-rectangle storey shell (straight rectangle kits + ruled corners).
	RoundedRectFloor(rounded_rect_floor::RoundedRectFloor),
	/// I / T / U / L / Z storey shell from central bar + flanges.
	IFloor(i_floor::IFloorCmd),
	/// Rectangular ring storey (outer + inner walls, frame floor; openings for broad gaps).
	RectRingFloor(rect_ring_floor::RectRingFloor),
	/// Circular ring storey (outer + inner arcs, annulus floor).
	CircRingFloor(circ_ring_floor::CircRingFloor),
	/// Single oriented [`buildings::Rectangle`] kit (floor / wall / ceiling presets).
	Rectangle(rectangle::Rectangle),
	/// Oriented rectangle kit with an inset framed by rectangle kits.
	ClippedRectangle(clipped_rectangle::ClippedRectangle),
	/// Node-chain oriented rectangle strip with a mid-bay inset frame.
	ClippedRectangularStrip(clipped_rectangular_strip::ClippedRectangularStrip),
	/// Single best-fit [`buildings::FittedRectangle`] (four corners / presets).
	FittedRectangle(fitted_rectangle::FittedRectangle),
	/// Best-fit rectangle with an inset framed by rectangle kits.
	ClippedFittedRectangle(clipped_fitted_rectangle::ClippedFittedRectangle),
	/// Two-rail best-fit rectangle strip with a mid-bay inset frame.
	ClippedFittedRectangularStrip(clipped_fitted_rectangular_strip::ClippedFittedRectangularStrip),
	/// Closed n-gon cross-section polyline → n clipped rectangle strips.
	RectangularNTube(rectangular_n_tube::RectangularNTube),
	/// N-gon disk / annulus filled with right-triangle panel kits.
	ApproximatedCircle(approximated_circle::ApproximatedCircle),
	/// Circular fitted [`buildings::arcs::ArcSweep`] (not IR `partitions::ArcSweep`).
	ArcSweep(arc_sweep::ArcSweep),
	/// Circular arc with angular clip openings → solid + slice bands.
	ClippedArcSweep(clipped_arc_sweep::ClippedArcSweep),
	/// Two lines → two tessellated triangles + optional crease `JointNode`.
	QuadPanel(quad_panel::QuadPanel),
	/// Point-id triangle mesh → panels + crease joints (`id=(x,y,z) ... {a,b,c}`).
	PanelComplex(panel_complex::PanelComplex),
	/// Point-id quad mesh → panels + crease joints (`id=(x,y,z) ... {a0,a1,b0,b1}`).
	QuadPanelComplex(quad_panel_complex::QuadPanelComplex),
	/// Equal eave/ridge polylines → ruled quad strip + crease joints.
	RuledPitch(ruled_pitch::RuledPitch),
	/// L-shaped `Partition::polyline` (posed linears + joints).
	Polyline(polyline::Polyline),
	/// Noisy path → [`buildings::NoisyRectangularWall`] rectangle strip demo.
	NoisyRectangularWall(noisy_rectangular_wall::NoisyRectangularWall),
	/// Full Wizard's Tower (noise-derived floor count).
	WizardsTower(wizards_tower::WizardsTower),
	/// Stacked circular wall rings (validates kit radius/height scaling).
	StackedRings(stacked_rings::StackedRings),
	/// Common bedroom usage area (beds + walled closet / ensuite).
	Bedroom(bedroom::Bedroom),
	/// Gallery of CommonBedroom cells (passage AABBs drawn as gizmos).
	BedroomExamples(bedroom_examples::BedroomExamples),
	/// Residential full bathroom shell (label + passage keep-outs).
	ResidentialBathroom(residential_bathroom::ResidentialBathroom),
	/// Residential half bathroom shell (label + passage keep-outs).
	ResidentialHalfBathroom(residential_half_bathroom::ResidentialHalfBathroom),
	/// Gallery of full + half residential bathrooms (passage AABBs drawn as gizmos).
	ResidentialBathroomExamples(residential_bathroom_examples::ResidentialBathroomExamples),
	/// Gallery of Kitchen cells (passage AABBs drawn as gizmos).
	KitchenExamples(kitchen_examples::KitchenExamples),
	/// Gallery of DiningRoom cells (passage AABBs drawn as gizmos).
	DiningRoomExamples(dining_room_examples::DiningRoomExamples),
	/// Gallery of LivingRoom cells (passage AABBs drawn as gizmos).
	LivingRoomExamples(living_room_examples::LivingRoomExamples),
	/// Gallery of SittingRoom cells (passage AABBs drawn as gizmos).
	SittingRoomExamples(sitting_room_examples::SittingRoomExamples),
	/// Gallery of Study cells (passage AABBs drawn as gizmos).
	StudyExamples(study_examples::StudyExamples),
	/// Single commercial stall Label placeholder.
	CommercialStall(commercial_stall::CommercialStall),
	/// Commercial stall strip (packed Labels along a band).
	CommercialStallStrip(commercial_stall_strip::CommercialStallStrip),
	/// Bites stall interior (counters on long passages + kitchen remainder).
	BitesStall(bites_stall::BitesStall),
	/// Bites sit-down (counters + passage-connected seating + kitchen).
	BitesSitdownStall(bites_sitdown_stall::BitesSitdownStall),
	/// Gallery of bites + sit-down stalls (passage AABBs drawn as gizmos).
	BitesExamples(bites_examples::BitesExamples),
	/// MiniMart interior (clearances, office+door, register, aisles, shelves).
	MiniMart(mini_mart::MiniMart),
	/// Gallery of MiniMart stalls (passage AABBs drawn as gizmos).
	MiniMartExamples(mini_mart_examples::MiniMartExamples),
	/// Parts stall interior (office + parts pockets, passage clearances).
	PartsStall(parts_stall::PartsStall),
	/// Gallery of Parts stalls (passage AABBs drawn as gizmos).
	PartsExamples(parts_examples::PartsExamples),
	/// Knick-knack stall interior (passage clearances + wall displays).
	KnickKnackStall(knick_knack_stall::KnickKnackStall),
	/// Gallery of KnickKnack stalls (passage AABBs drawn as gizmos).
	KnickKnackExamples(knick_knack_examples::KnickKnackExamples),
	/// Public restroom interior (walled stalls + door, sinks, passage clearances).
	PublicRestroom(public_restroom::PublicRestroom),
	/// Gallery of PublicRestroom stalls (passage AABBs drawn as gizmos).
	PublicRestroomExamples(public_restroom_examples::PublicRestroomExamples),
	/// Les Halles floor plan (ring shell + residual within cells).
	LesHallesFloorPlan(les_halles_floor_plan::LesHallesFloorPlan),
	/// Gallery of Les Halles floor plans (varied extents/seeds / shaft placements).
	LesHallesFloorPlanExamples(les_halles_floor_plan_examples::LesHallesFloorPlanExamples),
	/// Les Halles full storey (shell + commercial stall strip fills).
	LesHallesFullStorey(les_halles_full_storey::LesHallesFullStorey),
	/// Les Halles livable full storey (shell + lengthwise RectangularLivableArea bays).
	LesHallesLivableFullStorey(les_halles_livable_full_storey::LesHallesLivableFullStorey),
	/// Gallery of livable Les Halles full storeys (varied extents/seeds).
	LesHallesLivableFullStoreyExamples(
		les_halles_livable_full_storey_examples::LesHallesLivableFullStoreyExamples,
	),
	/// Mixed-use Les Halles monotower (commercial storeys below, livable above).
	MixedUseLesHallesMonotower(mixed_use_les_halles_monotower::MixedUseLesHallesMonotower),
	/// Flattened Les Halles development (per-floor hosts, shaft stairs, pitched roof).
	MixedUseLesHallesDevelopment(mixed_use_les_halles_development::MixedUseLesHallesDevelopment),
	/// I-Apartment floor plan (IFloor + primary rect residuals).
	IApartmentFloorPlan(i_apartment_floor_plan::IApartmentFloorPlan),
	/// Gallery of I-Apartment floor plans via Fit (varied extents/seeds).
	IApartmentFloorPlanExamples(i_apartment_floor_plan_examples::IApartmentFloorPlanExamples),
	/// I-Apartment full storey (plan + LivableApartments per primary rect).
	IApartmentFullStorey(i_apartment_full_storey::IApartmentFullStorey),
	/// Gallery of I-Apartment full storeys (halls, groups, partition walls).
	IApartmentFullStoreyExamples(i_apartment_full_storey_examples::IApartmentFullStoreyExamples),
	/// Gallery of LivableApartments packs on rectangular hosts.
	LivableApartmentsExamples(livable_apartments_examples::LivableApartmentsExamples),
	/// Gallery of standalone LivableApartment layouts (entry + common/private).
	LivableApartmentExamples(livable_apartment_examples::LivableApartmentExamples),
	/// Gallery of standalone RectangularLivableArea fits (passages + strategies).
	LivableRectanglesExamples(livable_rectangles_examples::LivableRectanglesExamples),
	/// HallsToShafts carve demo (hall / shaft / passage / residual boxes).
	HallsToShafts(halls_to_shafts::HallsToShafts),
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		let preview = match self {
			Self::Linear(cmd) => Ok(cmd.into_preview()),
			Self::Arc90(cmd) => Ok(cmd.into_preview()),
			Self::Arc180(cmd) => Ok(cmd.into_preview()),
			Self::Slice90(cmd) => Ok(cmd.into_preview()),
			Self::Pitch(cmd) => Ok(cmd.into_preview()),
			Self::TessellatedTriangle(cmd) => Ok(cmd.into_preview()),
			Self::TessellatedTriangle3d(cmd) => Ok(cmd.into_preview()),
			Self::ClippedTessellatedTriangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedQuadPanel(cmd) => Ok(cmd.into_preview()),
			Self::ClippedRuledStrip(cmd) => Ok(cmd.into_preview()),
			Self::Tube(cmd) => Ok(cmd.into_preview()),
			Self::ConnectingHall(cmd) => Ok(cmd.into_preview()),
			Self::ConnectingStairwell(cmd) => Ok(cmd.into_preview()),
			Self::ConnectingStairwellExamples(cmd) => Ok(cmd.into_preview()),
			Self::ArcFloor(cmd) => cmd.into_preview(),
			Self::ArcTower(cmd) => Ok(cmd.into_preview()),
			Self::ConnectingShells(cmd) => Ok(cmd.into_preview()),
			Self::Trazaloid(cmd) => cmd.into_preview(),
			Self::PitchedRectangularRoof(cmd) => cmd.into_preview(),
			Self::RectangularPitchedRoofComplex(cmd) => Ok(cmd.into_preview()),
			Self::RectFloor(cmd) => cmd.into_preview(),
			Self::RoundedRectFloor(cmd) => cmd.into_preview(),
			Self::IFloor(cmd) => cmd.into_preview(),
			Self::RectRingFloor(cmd) => cmd.into_preview(),
			Self::CircRingFloor(cmd) => cmd.into_preview(),
			Self::Rectangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedRectangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedRectangularStrip(cmd) => Ok(cmd.into_preview()),
			Self::FittedRectangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedFittedRectangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedFittedRectangularStrip(cmd) => Ok(cmd.into_preview()),
			Self::RectangularNTube(cmd) => Ok(cmd.into_preview()),
			Self::ApproximatedCircle(cmd) => Ok(cmd.into_preview()),
			Self::ArcSweep(cmd) => Ok(cmd.into_preview()),
			Self::ClippedArcSweep(cmd) => Ok(cmd.into_preview()),
			Self::QuadPanel(cmd) => Ok(cmd.into_preview()),
			Self::PanelComplex(cmd) => Ok(cmd.into_preview()),
			Self::QuadPanelComplex(cmd) => Ok(cmd.into_preview()),
			Self::RuledPitch(cmd) => Ok(cmd.into_preview()),
			Self::Polyline(cmd) => Ok(cmd.into_preview()),
			Self::NoisyRectangularWall(cmd) => Ok(cmd.into_preview()),
			Self::WizardsTower(cmd) => Ok(cmd.into_preview()),
			Self::StackedRings(cmd) => Ok(cmd.into_preview()),
			Self::Bedroom(cmd) => Ok(cmd.into_preview()),
			Self::BedroomExamples(cmd) => Ok(cmd.into_preview()),
			Self::ResidentialBathroom(cmd) => Ok(cmd.into_preview()),
			Self::ResidentialHalfBathroom(cmd) => Ok(cmd.into_preview()),
			Self::ResidentialBathroomExamples(cmd) => Ok(cmd.into_preview()),
			Self::KitchenExamples(cmd) => Ok(cmd.into_preview()),
			Self::DiningRoomExamples(cmd) => Ok(cmd.into_preview()),
			Self::LivingRoomExamples(cmd) => Ok(cmd.into_preview()),
			Self::SittingRoomExamples(cmd) => Ok(cmd.into_preview()),
			Self::StudyExamples(cmd) => Ok(cmd.into_preview()),
			Self::CommercialStall(cmd) => Ok(cmd.into_preview()),
			Self::CommercialStallStrip(cmd) => Ok(cmd.into_preview()),
			Self::BitesStall(cmd) => Ok(cmd.into_preview()),
			Self::BitesSitdownStall(cmd) => Ok(cmd.into_preview()),
			Self::BitesExamples(cmd) => Ok(cmd.into_preview()),
			Self::MiniMart(cmd) => Ok(cmd.into_preview()),
			Self::MiniMartExamples(cmd) => Ok(cmd.into_preview()),
			Self::PartsStall(cmd) => Ok(cmd.into_preview()),
			Self::PartsExamples(cmd) => Ok(cmd.into_preview()),
			Self::KnickKnackStall(cmd) => Ok(cmd.into_preview()),
			Self::KnickKnackExamples(cmd) => Ok(cmd.into_preview()),
			Self::PublicRestroom(cmd) => Ok(cmd.into_preview()),
			Self::PublicRestroomExamples(cmd) => Ok(cmd.into_preview()),
			Self::LesHallesFloorPlan(cmd) => cmd.into_preview(),
			Self::LesHallesFloorPlanExamples(cmd) => Ok(cmd.into_preview()),
			Self::LesHallesFullStorey(cmd) => cmd.into_preview(),
			Self::LesHallesLivableFullStorey(cmd) => cmd.into_preview(),
			Self::LesHallesLivableFullStoreyExamples(cmd) => Ok(cmd.into_preview()),
			Self::MixedUseLesHallesMonotower(cmd) => cmd.into_preview(),
			Self::MixedUseLesHallesDevelopment(cmd) => cmd.into_preview(),
			Self::IApartmentFloorPlan(cmd) => cmd.into_preview(),
			Self::IApartmentFloorPlanExamples(cmd) => Ok(cmd.into_preview()),
			Self::IApartmentFullStorey(cmd) => cmd.into_preview(),
			Self::IApartmentFullStoreyExamples(cmd) => Ok(cmd.into_preview()),
			Self::LivableApartmentsExamples(cmd) => Ok(cmd.into_preview()),
			Self::LivableApartmentExamples(cmd) => Ok(cmd.into_preview()),
			Self::LivableRectanglesExamples(cmd) => Ok(cmd.into_preview()),
			Self::HallsToShafts(cmd) => cmd.into_preview(),
		};
		match preview {
			Ok((subject, transform)) => {
				commands.insert_resource(PreviewConfig { label_text: true, subject, transform });
			}
			Err(err) => {
				error!("show failed: {err}");
				commands.insert_resource(GameCommandStatusText(format!("show failed: {err}")));
			}
		}
	}
}
