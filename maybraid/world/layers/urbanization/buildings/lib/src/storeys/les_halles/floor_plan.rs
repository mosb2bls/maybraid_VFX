//! Les Halles floor plan: gallery ring (walls + floor) and balcony floor ring.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{
	ring_strip_xz, BuildingComponents, BuildingStructuralLodProbe, Layers, MassingVolume,
};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{
	aabb_near_plane, aabb_xz_center, aabb_xz_overlap_area, Confines, FillRegion, FillableRegions,
	Fit, FitError, SpaceKind, StackRegion,
};
use crate::openings::{
	generate_stall_doors as gen_stall_doors, generate_windows as gen_windows,
	sync_connectable_openings_from_mapped, Opening, OpeningId, OpeningLabel, Openings,
};
use crate::paneling::fitted_rectangle::FittedRectangle;
use crate::paneling::panel_complex::{PanelPoint, DEFAULT_PANEL_THICKNESS};
use crate::paneling::pillar::PanelPillarLine;
use crate::paneling::rectangle::Rectangle;
use crate::shells::ortho::{OrthoSide, PlanRect, EPS};
use crate::shells::rect_ring_floor::{
	RectRingFloor, RectRingFloorParams, RectRingFloorSide, RectRingFloorSlab,
};

use super::parameterized::{
	footprint_extents, LesHallesParameterized, LesHallesPlacedDoor, LesHallesShaftPlacement,
	LesHallesStallDoor,
};
use super::SCOPE;

/// Pull leaves away from free-run ends so passage AABBs clear adjacent ring walls
/// (~`standing_face_opening` half-thickness pad). Shrink is charged against each
/// door’s [`LesHallesPlacedDoor::allowed_error`].
const INNER_DOOR_END_CLEARANCE: f32 = 0.45;
/// Corner F2C clear may use at most this fraction of the gallery-inner along-half.
const CORNER_CLEAR_MAX_FRAC: f32 = 0.45;
/// Minimum along-width for shaft clears (corner F2C and mid-side face clears).
const MIN_SHAFT_CLEAR: f32 = 1.2;
/// Pedestrian arcade cut through a façade panel (meters).
const MIN_BREEZEWAY_WIDTH: f32 = 2.8;
const MAX_BREEZEWAY_WIDTH: f32 = 5.5;
const BREEZEWAY_RUN_FRAC: f32 = 0.22;
const BREEZEWAY_SIDE_MARGIN: f32 = 0.6;
/// Square pier on the arcade courtyard line (meters).
const ARCADE_PILLAR_WIDTH: f32 = 0.6;
/// Target center-to-center spacing along an arcade colonnade run (meters).
const ARCADE_PILLAR_SPACING: f32 = 4.5;

/// How a Les Halles ring authors façade / inner-wall openings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LesHallesOpeningProgram {
	/// Stall doors on the gallery inner wall; packed apertures on the outer façade.
	#[default]
	CommercialGallery,
	/// Floor-to-ceiling breezeway at the midspan of each outer free panel, leftover
	/// façade windows, and no gallery inner walls (open to the courtyard).
	GroundArcade,
}

/// Axis-aligned plan rectangle from a center + full XZ size (`size.y` = Z extent).
#[derive(Debug, Clone, Copy)]
struct PlanFrame {
	x0: f32,
	x1: f32,
	z0: f32,
	z1: f32,
}

impl PlanFrame {
	fn from_center_size(center: Vec3, size: Vec2) -> Self {
		Self {
			x0: center.x - size.x * 0.5,
			x1: center.x + size.x * 0.5,
			z0: center.z - size.y * 0.5,
			z1: center.z + size.y * 0.5,
		}
	}
}

fn along_half(side: OrthoSide, size: Vec2) -> f32 {
	match side {
		OrthoSide::North | OrthoSide::South => size.x * 0.5,
		OrthoSide::East | OrthoSide::West => size.y * 0.5,
	}
}

/// World-space base point on the gallery-inner rectangle for an along-mid offset.
fn along_world_on_inner(side: OrthoSide, along: f32, center_xz: Vec3, gallery_inner: Vec2) -> Vec3 {
	match side {
		OrthoSide::South => {
			Vec3::new(center_xz.x + along, center_xz.y, center_xz.z - gallery_inner.y * 0.5)
		}
		OrthoSide::North => {
			Vec3::new(center_xz.x + along, center_xz.y, center_xz.z + gallery_inner.y * 0.5)
		}
		OrthoSide::East => {
			Vec3::new(center_xz.x + gallery_inner.x * 0.5, center_xz.y, center_xz.z + along)
		}
		OrthoSide::West => {
			Vec3::new(center_xz.x - gallery_inner.x * 0.5, center_xz.y, center_xz.z + along)
		}
	}
}

/// Clamp authored corner clear length to the gallery-inner half-run.
fn clamped_corner_clear(clear_len: f32, along_half: f32) -> f32 {
	clear_len.min(along_half * CORNER_CLEAR_MAX_FRAC).max(MIN_SHAFT_CLEAR)
}

/// Along-coord (vs side mid) where the solid wall ends / F2C clear begins.
fn corner_wall_terminus(half: f32, clear: f32, end_i: usize) -> f32 {
	if end_i == 0 {
		-half + clear
	} else {
		half - clear
	}
}

/// Structural Les Halles plan.
///
/// - **Gallery** — rectangular ring with outer facade walls, inner walls facing
///   the balcony, and a solid floor (commercial band). Ceiling is optional
///   ([`RectRingFloorSlab::None`] by default).
/// - **Balcony** — floor-only annulus between the gallery’s inner wall and the
///   courtyard (no walls).
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesFloorPlan {
	pub parameterized: LesHallesParameterized,
	/// Storey plan center; `y` is the floor elevation.
	pub center_xz: Vec3,
	/// Full outer footprint width (X) / depth (Z).
	pub outer: Vec2,
	/// Gallery inner / balcony outer — wall line between gallery and balcony.
	pub gallery_inner: Vec2,
	/// Open courtyard width (X) / depth (Z).
	pub courtyard: Vec2,
	pub storey_height: f32,
	pub roll: f32,
	/// Ceiling slab on the gallery ring ([`RectRingFloorSlab::None`] by default).
	pub ceiling: RectRingFloorSlab,
	/// Merged inbound + generated openings for the gallery shell.
	///
	/// Inbound [`OpeningLabel::Shaft`] openings keep their ids; their bounds are
	/// rewritten onto a fitted shaft via [`Self::map_inbound_shafts`]. Shafts are
	/// only authored when at least one inbound opening maps to that slot.
	pub openings: Openings,
	/// Active shaft volumes (only slots that received inbound shaft openings).
	pub shaft_bounds: Vec<Aabb3d>,
	/// Stable placement slot index (`0…3`) for each entry in [`Self::shaft_bounds`].
	pub shaft_slots: Vec<usize>,
	/// Inbound shaft [`OpeningId`]s remapped onto each active shaft (parallel).
	pub shaft_inbound: Vec<Vec<OpeningId>>,
	/// Gallery ring (outer walls, floor, optional ceiling; inner walls omitted on arcade).
	pub gallery: RectRingFloor,
	/// Floor-only balcony annulus pieces (no walls).
	pub balcony_floors: Vec<FittedRectangle>,
	/// Radial walls across the gallery at each shaft (mid-side faces or corner run ends).
	pub shaft_walls: Vec<Rectangle>,
	/// Courtyard colonnade on the ground arcade (`GroundArcade` only).
	pub arcade_pillars: Vec<PanelPillarLine>,
}

impl LesHallesFloorPlan {
	/// Stall-door catalog — see [`crate::openings::generate_stall_doors`].
	pub fn generate_stall_doors(cfg: &NoiseConfig, center: Vec3) -> Vec<LesHallesStallDoor> {
		gen_stall_doors(cfg, center)
	}

	/// Exterior aperture catalog — see [`crate::openings::generate_windows`].
	pub fn generate_windows(cfg: &NoiseConfig, center: Vec3) -> Vec<LesHallesStallDoor> {
		gen_windows(cfg, center)
	}

	/// Four gallery strips (outer minus courtyard), each this storey's height.
	pub fn massing_volumes(&self, material: Option<MaterialRef>) -> Vec<MassingVolume> {
		let center = Vec2::new(self.center_xz.x, self.center_xz.z);
		ring_strip_xz(center, self.outer, self.courtyard)
			.into_iter()
			.map(|xz| {
				MassingVolume::cuboid(xz, self.center_xz.y, self.storey_height)
					.with_material_opt(material.clone())
			})
			.collect()
	}

	pub fn structural_probe(&self, material: Option<MaterialRef>) -> BuildingStructuralLodProbe {
		BuildingStructuralLodProbe::from_volumes(self.massing_volumes(material))
	}

	/// Deterministic structure from already-sampled parameters (towering path).
	///
	/// Gallery ceiling defaults to [`RectRingFloorSlab::None`].
	pub fn from_parameterized(
		params: LesHallesParameterized,
		confines: &Confines,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_parameterized_with_ceiling(params, confines, RectRingFloorSlab::None)
	}

	/// Same as [`Self::from_parameterized`] with an explicit gallery ceiling.
	pub fn from_parameterized_with_ceiling(
		params: LesHallesParameterized,
		confines: &Confines,
		ceiling: RectRingFloorSlab,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_parameterized_with(
			params,
			confines,
			ceiling,
			LesHallesOpeningProgram::CommercialGallery,
		)
	}

	/// Structure from sampled parameters, ceiling slab, and façade opening program.
	pub fn from_parameterized_with(
		params: LesHallesParameterized,
		confines: &Confines,
		ceiling: RectRingFloorSlab,
		program: LesHallesOpeningProgram,
	) -> Result<(Self, FillableRegions), FitError> {
		let (extent_x, extent_z, height) = footprint_extents(confines)?;
		let ring = params.ring_width();
		let courtyard_x = extent_x - 2.0 * ring;
		let courtyard_z = extent_z - 2.0 * ring;
		if courtyard_x < super::parameterized::MIN_COURTYARD
			|| courtyard_z < super::parameterized::MIN_COURTYARD
		{
			return Err(FitError::TooSmall { reason: "footprint" });
		}

		let center = confines.center();
		let y0 = confines.bounds.min.y;
		let center_xz = Vec3::new(center.x, y0, center.z);
		let outer = Vec2::new(extent_x, extent_z);
		let gallery_inner = Vec2::new(
			(extent_x - 2.0 * params.gallery_width).max(EPS),
			(extent_z - 2.0 * params.gallery_width).max(EPS),
		);
		let courtyard = Vec2::new(courtyard_x, courtyard_z);

		// Candidate slots for mapping; only slots that receive inbound shafts are kept.
		let candidates = Self::shaft_aabbs(
			center_xz,
			outer,
			params.gallery_width,
			height,
			params.shaft_placement,
			params.mid_shaft_side,
		);

		let mut openings = confines.openings.clone();
		let inbound_by_slot = Self::map_inbound_shafts(
			&mut openings,
			center_xz,
			outer,
			params.shaft_placement,
			&candidates,
		);
		let mut shaft_bounds = Vec::new();
		let mut shaft_slots = Vec::new();
		let mut shaft_inbound = Vec::new();
		for (slot, ids) in inbound_by_slot.into_iter().enumerate() {
			if ids.is_empty() {
				continue;
			}
			shaft_slots.push(slot);
			shaft_bounds.push(candidates[slot]);
			shaft_inbound.push(ids);
		}

		openings.extend(&Self::generated_openings(
			&params,
			center_xz,
			outer,
			gallery_inner,
			height,
			&shaft_bounds,
			&shaft_slots,
			program,
		));

		let gallery = Self::build_gallery(
			center_xz,
			outer,
			gallery_inner,
			height,
			ceiling,
			&openings,
			program != LesHallesOpeningProgram::GroundArcade,
		);
		// Drop unmapped Passage/Aperture and sync truncated AABBs from the gallery
		// so commercial strips never see boarded or oversized voids.
		sync_connectable_openings_from_mapped(&mut openings, &gallery);
		let balcony_floors = Self::build_balcony_floors(center_xz, gallery_inner, courtyard, y0);
		// Mid-side: both radial faces of each shaft. Corners: one end wall per
		// abutting run, seated where the inner wall clear begins (end of the
		// solid gallery wall) — not on the shaft faces (that boxed the corner).
		let shaft_walls = match params.shaft_placement {
			LesHallesShaftPlacement::Corners => Self::build_corner_shaft_end_walls(
				center_xz,
				outer,
				gallery_inner,
				height,
				params.corner_clear_len(),
				&shaft_slots,
			),
			LesHallesShaftPlacement::MidSides => {
				Self::build_shaft_walls(center_xz, outer, gallery_inner, height, &shaft_bounds)
			}
		};

		let arcade_pillars = if program == LesHallesOpeningProgram::GroundArcade {
			Self::build_arcade_pillars(
				center_xz,
				gallery_inner,
				height,
				&shaft_bounds,
				&shaft_slots,
				params.shaft_placement,
				params.corner_clear_len(),
			)
		} else {
			Vec::new()
		};

		let plan = Self {
			parameterized: params,
			center_xz,
			outer,
			gallery_inner,
			courtyard,
			storey_height: height,
			roll: confines.roll,
			ceiling,
			openings,
			shaft_bounds,
			shaft_slots,
			shaft_inbound,
			gallery,
			balcony_floors,
			shaft_walls,
			arcade_pillars,
		};

		let regions = plan.fillable_regions();
		Ok((plan, regions))
	}

	/// Residual gallery strips / balcony / shaft confines and stack region.
	///
	/// - [`SpaceKind::ExternalSpace`] — commercial gallery strips (straight
	///   sections between shaft clears) with openings that intersect the strip.
	/// - [`SpaceKind::Walkway`] — balcony walking bands.
	/// - [`SpaceKind::InternalSpace`] — active shaft cells.
	pub fn fillable_regions(&self) -> FillableRegions {
		let y0 = self.center_xz.y;
		let y1 = y0 + self.storey_height;
		let ox0 = self.center_xz.x - self.outer.x * 0.5;
		let ox1 = self.center_xz.x + self.outer.x * 0.5;
		let oz0 = self.center_xz.z - self.outer.y * 0.5;
		let oz1 = self.center_xz.z + self.outer.y * 0.5;
		let gx = self.parameterized.gallery_width;
		let bx = self.parameterized.balcony_width;
		let ix0 = self.center_xz.x - self.courtyard.x * 0.5;
		let ix1 = self.center_xz.x + self.courtyard.x * 0.5;
		let iz0 = self.center_xz.z - self.courtyard.y * 0.5;
		let iz1 = self.center_xz.z + self.courtyard.y * 0.5;

		let mut within = Vec::new();

		// Commercial strips stop at the same shaft clears as the inner wall
		// (corner buffer / mid-side shaft spans).
		let sections = Self::commercial_fill_sections(
			self.center_xz,
			self.outer,
			self.gallery_inner,
			&self.shaft_bounds,
			&self.shaft_slots,
			self.parameterized.shaft_placement,
			self.parameterized.corner_clear_len(),
		);
		for section in &sections {
			let bounds =
				Self::gallery_strip_bounds(self.center_xz, self.outer, gx, y0, y1, section);
			let openings = subset_openings_intersecting(&self.openings, &bounds);
			within.push(FillRegion::new(
				SpaceKind::ExternalSpace,
				Confines::new(bounds, self.roll, openings),
			));
		}

		// Balcony bands (inner walking ring), four sides.
		within.push(FillRegion::new(
			SpaceKind::Walkway,
			Self::band_confine(ix0 - bx, ix1 + bx, iz0 - bx, iz0, y0, y1, self.roll),
		));
		within.push(FillRegion::new(
			SpaceKind::Walkway,
			Self::band_confine(ix0 - bx, ix1 + bx, iz1, iz1 + bx, y0, y1, self.roll),
		));
		within.push(FillRegion::new(
			SpaceKind::Walkway,
			Self::band_confine(ix1, ix1 + bx, iz0, iz1, y0, y1, self.roll),
		));
		within.push(FillRegion::new(
			SpaceKind::Walkway,
			Self::band_confine(ix0 - bx, ix0, iz0, iz1, y0, y1, self.roll),
		));

		for (i, shaft) in self.shaft_bounds.iter().enumerate() {
			let slot = self.shaft_slots.get(i).copied().unwrap_or(i);
			let mut openings = Openings::new();
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft", slot.to_string()),
				Opening::new(*shaft, OpeningLabel::Shaft),
			);
			if let Some(ids) = self.shaft_inbound.get(i) {
				for id in ids {
					openings.insert(id.clone(), Opening::new(*shaft, OpeningLabel::Shaft));
				}
			}
			within.push(FillRegion::new(
				SpaceKind::InternalSpace,
				Confines::new(*shaft, self.roll, openings),
			));
		}

		let atop = vec![StackRegion {
			bounds: Aabb2d { min: Vec2::new(ox0, oz0), max: Vec2::new(ox1, oz1) },
			height: self.storey_height,
			roll: self.roll,
			openings: self.openings.clone(),
		}];

		FillableRegions { within, atop }
	}

	/// Gallery-depth AABB for one straight commercial section.
	fn gallery_strip_bounds(
		center_xz: Vec3,
		outer: Vec2,
		gallery_width: f32,
		y0: f32,
		y1: f32,
		section: &AlongSection,
	) -> Aabb3d {
		let ox0 = center_xz.x - outer.x * 0.5;
		let ox1 = center_xz.x + outer.x * 0.5;
		let oz0 = center_xz.z - outer.y * 0.5;
		let oz1 = center_xz.z + outer.y * 0.5;
		let a0 = section.along0.min(section.along1);
		let a1 = section.along0.max(section.along1);
		match section.side {
			OrthoSide::South => Aabb3d::from_min_max(
				Vec3::new(center_xz.x + a0, y0, oz0),
				Vec3::new(center_xz.x + a1, y1, oz0 + gallery_width),
			),
			OrthoSide::North => Aabb3d::from_min_max(
				Vec3::new(center_xz.x + a0, y0, oz1 - gallery_width),
				Vec3::new(center_xz.x + a1, y1, oz1),
			),
			OrthoSide::East => Aabb3d::from_min_max(
				Vec3::new(ox1 - gallery_width, y0, center_xz.z + a0),
				Vec3::new(ox1, y1, center_xz.z + a1),
			),
			OrthoSide::West => Aabb3d::from_min_max(
				Vec3::new(ox0, y0, center_xz.z + a0),
				Vec3::new(ox0 + gallery_width, y1, center_xz.z + a1),
			),
		}
	}

	/// Build a residual [`Confines`] cell for one axis-aligned band of the ring.
	fn band_confine(
		min_x: f32,
		max_x: f32,
		min_z: f32,
		max_z: f32,
		y0: f32,
		y1: f32,
		roll: f32,
	) -> Confines {
		Confines::new(
			Aabb3d::from_min_max(Vec3::new(min_x, y0, min_z), Vec3::new(max_x, y1, max_z)),
			roll,
			Openings::new(),
		)
	}

	/// Candidate shaft AABBs for all four placement slots (may later be filtered
	/// to only those that receive inbound shaft openings).
	///
	/// - **Corners:** the corner gallery square is `gallery_width` on a side; the
	///   shaft is half that square (half-extents), seated on the inner corner so
	///   the outer facade stays solid.
	/// - **MidSides:** full gallery depth minus outer inset; along-wall size is
	///   [`LesHallesParameterized::mid_shaft_side`].
	fn shaft_aabbs(
		center_xz: Vec3,
		outer: Vec2,
		gallery_width: f32,
		height: f32,
		placement: LesHallesShaftPlacement,
		mid_shaft_side: f32,
	) -> Vec<Aabb3d> {
		let y0 = center_xz.y;
		let y1 = y0 + height;
		let half = mid_shaft_side.max(EPS) * 0.5;
		let ox0 = center_xz.x - outer.x * 0.5;
		let ox1 = center_xz.x + outer.x * 0.5;
		let oz0 = center_xz.z - outer.y * 0.5;
		let oz1 = center_xz.z + outer.y * 0.5;
		let gw = gallery_width.max(EPS);
		// Keep outer facade solid behind mid-side shafts.
		let outer_inset = (DEFAULT_PANEL_THICKNESS + 0.15).min(gw * 0.35);
		// Half-extents of the corner gallery square → shaft side length.
		let shaft_side = (gw * 0.5).max(EPS);

		match placement {
			LesHallesShaftPlacement::Corners => vec![
				// SW, SE, NE, NW — inner half of each corner square.
				Aabb3d::from_min_max(
					Vec3::new(ox0 + shaft_side, y0, oz0 + shaft_side),
					Vec3::new(ox0 + gw, y1, oz0 + gw),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox1 - gw, y0, oz0 + shaft_side),
					Vec3::new(ox1 - shaft_side, y1, oz0 + gw),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox1 - gw, y0, oz1 - gw),
					Vec3::new(ox1 - shaft_side, y1, oz1 - shaft_side),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox0 + shaft_side, y0, oz1 - gw),
					Vec3::new(ox0 + gw, y1, oz1 - shaft_side),
				),
			],
			LesHallesShaftPlacement::MidSides => vec![
				Aabb3d::from_min_max(
					Vec3::new(center_xz.x - half, y0, oz0 + outer_inset),
					Vec3::new(center_xz.x + half, y1, oz0 + gw),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox1 - gw, y0, center_xz.z - half),
					Vec3::new(ox1 - outer_inset, y1, center_xz.z + half),
				),
				Aabb3d::from_min_max(
					Vec3::new(center_xz.x - half, y0, oz1 - gw),
					Vec3::new(center_xz.x + half, y1, oz1 - outer_inset),
				),
				Aabb3d::from_min_max(
					Vec3::new(ox0 + outer_inset, y0, center_xz.z - half),
					Vec3::new(ox0 + gw, y1, center_xz.z + half),
				),
			],
		}
	}

	/// Small inbound [`OpeningLabel::Shaft`] requests centered on every candidate
	/// slot — useful for playground demos and tests that want all shafts active.
	pub fn shaft_requests_for_all_slots(
		params: &LesHallesParameterized,
		confines: &Confines,
	) -> Openings {
		Self::shaft_requests_for_slots(params, confines, &[0, 1, 2, 3])
	}

	/// Small [`OpeningLabel::Shaft`] requests for the given placement slots (`0…3`).
	pub fn shaft_requests_for_slots(
		params: &LesHallesParameterized,
		confines: &Confines,
		slots: &[usize],
	) -> Openings {
		let (extent_x, extent_z, height) = match footprint_extents(confines) {
			Ok(v) => v,
			Err(_) => return Openings::new(),
		};
		let y0 = confines.bounds.min.y;
		let c = confines.center();
		let center_xz = Vec3::new(c.x, y0, c.z);
		let outer = Vec2::new(extent_x, extent_z);
		let candidates = Self::shaft_aabbs(
			center_xz,
			outer,
			params.gallery_width,
			height,
			params.shaft_placement,
			params.mid_shaft_side,
		);
		let mut openings = Openings::new();
		for &slot in slots {
			let Some(shaft) = candidates.get(slot) else {
				continue;
			};
			let mid = Vec3::from((shaft.min + shaft.max) * 0.5);
			let half = 0.4_f32;
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft_req", slot.to_string()),
				Opening::new(
					Aabb3d::from_min_max(
						Vec3::new(mid.x - half, y0, mid.z - half),
						Vec3::new(mid.x + half, y0 + height.min(3.0), mid.z + half),
					),
					OpeningLabel::Shaft,
				),
			);
		}
		openings
	}

	/// Plan-space (XZ) regions that claim inbound shafts for each fitted slot.
	///
	/// - **Corners:** four quadrants about the footprint center (SW, SE, NE, NW).
	/// - **MidSides:** N/S end bands (full width) and E/W middle bands (half width),
	///   an exclusive partition of the outer footprint.
	///
	/// Region order matches [`Self::shaft_aabbs`].
	fn shaft_mapping_regions(
		center_xz: Vec3,
		outer: Vec2,
		placement: LesHallesShaftPlacement,
	) -> Vec<Aabb2d> {
		let ox0 = center_xz.x - outer.x * 0.5;
		let ox1 = center_xz.x + outer.x * 0.5;
		let oz0 = center_xz.z - outer.y * 0.5;
		let oz1 = center_xz.z + outer.y * 0.5;
		let cx = center_xz.x;
		let cz = center_xz.z;

		match placement {
			LesHallesShaftPlacement::Corners => vec![
				// SW, SE, NE, NW
				Aabb2d { min: Vec2::new(ox0, oz0), max: Vec2::new(cx, cz) },
				Aabb2d { min: Vec2::new(cx, oz0), max: Vec2::new(ox1, cz) },
				Aabb2d { min: Vec2::new(cx, cz), max: Vec2::new(ox1, oz1) },
				Aabb2d { min: Vec2::new(ox0, cz), max: Vec2::new(cx, oz1) },
			],
			LesHallesShaftPlacement::MidSides => {
				// End thirds (N/S, full X) + middle third split E/W.
				let z_lo = oz0 + (oz1 - oz0) / 3.0;
				let z_hi = oz1 - (oz1 - oz0) / 3.0;
				vec![
					// S, E, N, W
					Aabb2d { min: Vec2::new(ox0, oz0), max: Vec2::new(ox1, z_lo) },
					Aabb2d { min: Vec2::new(cx, z_lo), max: Vec2::new(ox1, z_hi) },
					Aabb2d { min: Vec2::new(ox0, z_hi), max: Vec2::new(ox1, oz1) },
					Aabb2d { min: Vec2::new(ox0, z_lo), max: Vec2::new(cx, z_hi) },
				]
			}
		}
	}

	/// Rewrite inbound [`OpeningLabel::Shaft`] bounds onto fitted shaft slots.
	///
	/// Picks the mapping region with greatest XZ overlap; ties / no-overlap fall
	/// back to the region whose center is closest to the request center. Returns
	/// inbound ids per `shaft_bounds` index (empty slots stay empty).
	pub fn map_inbound_shafts(
		openings: &mut Openings,
		center_xz: Vec3,
		outer: Vec2,
		placement: LesHallesShaftPlacement,
		shaft_bounds: &[Aabb3d],
	) -> Vec<Vec<OpeningId>> {
		let regions = Self::shaft_mapping_regions(center_xz, outer, placement);
		debug_assert_eq!(regions.len(), shaft_bounds.len());
		let mut inbound: Vec<Vec<OpeningId>> =
			(0..shaft_bounds.len()).map(|_| Vec::new()).collect();

		let shaft_ids: Vec<OpeningId> = openings
			.iter()
			.filter(|(_, o)| matches!(o.label, OpeningLabel::Shaft))
			.map(|(id, _)| id.clone())
			.collect();

		for id in shaft_ids {
			let Some(opening) = openings.openings.get_mut(&id) else {
				continue;
			};
			let Some(slot) = best_shaft_slot(&opening.bounds, &regions) else {
				continue;
			};
			opening.bounds = shaft_bounds[slot];
			inbound[slot].push(id);
		}
		inbound
	}

	/// Gallery facade + balcony-facing openings, plus shaft voids / clears.
	///
	/// - **[`LesHallesOpeningProgram::CommercialGallery`]:** outer apertures packed
	///   from the window catalog; inner stall doors per straight section.
	/// - **[`LesHallesOpeningProgram::GroundArcade`]:** a floor-to-ceiling breezeway
	///   at the midspan of each outer free panel and windows on leftover façade.
	///   Inner gallery walls are omitted on the shell, so no inner passages or
	///   shaft clears are authored.
	/// - Shaft clears (corner / mid-side) apply to commercial / livable storeys.
	fn generated_openings(
		params: &LesHallesParameterized,
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		height: f32,
		shaft_bounds: &[Aabb3d],
		shaft_slots: &[usize],
		program: LesHallesOpeningProgram,
	) -> Openings {
		let mut openings = match program {
			LesHallesOpeningProgram::CommercialGallery => Self::generated_gallery_leaf_openings(
				params,
				center_xz,
				outer,
				gallery_inner,
				height,
				shaft_bounds,
				shaft_slots,
			),
			LesHallesOpeningProgram::GroundArcade => Self::generated_arcade_leaf_openings(
				params,
				center_xz,
				outer,
				gallery_inner,
				height,
				shaft_bounds,
				shaft_slots,
			),
		};

		for (i, shaft) in shaft_bounds.iter().enumerate() {
			let slot = shaft_slots.get(i).copied().unwrap_or(i);
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft", slot.to_string()),
				Opening::new(*shaft, OpeningLabel::Shaft),
			);
		}

		// Floor-to-ceiling clears on the gallery inner wall (active shafts only).
		// Arcade omits that wall, so these would never map.
		if program != LesHallesOpeningProgram::GroundArcade {
			match params.shaft_placement {
				LesHallesShaftPlacement::Corners => {
					Self::insert_corner_shaft_clears(
						&mut openings,
						center_xz,
						gallery_inner,
						height,
						params.corner_clear_len(),
						shaft_slots,
					);
				}
				LesHallesShaftPlacement::MidSides => {
					for (i, shaft) in shaft_bounds.iter().enumerate() {
						let slot = shaft_slots.get(i).copied().unwrap_or(i);
						let smin = Vec3::from(shaft.min);
						let smax = Vec3::from(shaft.max);
						for (side, along) in
							Self::shaft_inner_sides(center_xz, gallery_inner, *shaft)
						{
							let clear_w = match side {
								OrthoSide::North | OrthoSide::South => {
									(smax.x - smin.x).max(MIN_SHAFT_CLEAR)
								}
								OrthoSide::East | OrthoSide::West => {
									(smax.z - smin.z).max(MIN_SHAFT_CLEAR)
								}
							};
							let mut clear = RectRingFloor::side_passage_opening(
								side,
								center_xz,
								gallery_inner,
								clear_w,
								height,
							);
							clear.bounds = offset_opening_along_side(clear.bounds, side, along);
							openings.insert(
								OpeningId::scoped(
									SCOPE,
									"shaft_clear",
									format!("{slot}_{}", side_slot(side)),
								),
								clear,
							);
						}
					}
				}
			}
		}

		openings
	}

	/// Stall doors + packed outer apertures (commercial / livable storeys).
	fn generated_gallery_leaf_openings(
		params: &LesHallesParameterized,
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		height: f32,
		shaft_bounds: &[Aabb3d],
		shaft_slots: &[usize],
	) -> Openings {
		let mut openings = Openings::new();
		let door_h = (height * 0.72 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE)
			.clamp(2.0 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE, height.max(2.0));
		let win_h = (height * 0.42).clamp(1.0, height.max(1.0));
		let sill = (height * 0.28).clamp(0.7, height * 0.45);

		// Outer facade: pack apertures on free runs (skip behind mid-side shafts).
		let outer_sections =
			Self::outer_free_sections(center_xz, outer, shaft_bounds, params.shaft_placement);
		for (si, section) in outer_sections.iter().enumerate() {
			Self::insert_outer_windows_on_run(
				&mut openings,
				params,
				section,
				center_xz,
				outer,
				win_h,
				sill,
				si,
				"",
			);
		}

		// Inner stall doors on each straight section between shaft clears.
		let sections = Self::inner_straight_sections(
			center_xz,
			gallery_inner,
			shaft_bounds,
			shaft_slots,
			params.shaft_placement,
			params.corner_clear_len(),
		);
		let expected = params.expected_inner_section_count(shaft_bounds.len());
		debug_assert_eq!(
			sections.len(),
			expected,
			"inner straight sections: got {} expected {} ({:?}, {} shafts)",
			sections.len(),
			expected,
			params.shaft_placement,
			shaft_bounds.len()
		);
		assert!(
			sections.len() == expected,
			"Les Halles inner wall: expected {expected} straight sections for {:?} with {} shafts, got {}",
			params.shaft_placement,
			shaft_bounds.len(),
			sections.len()
		);

		for (si, section) in sections.iter().enumerate() {
			let run = (section.along1 - section.along0).max(0.0);
			let placed = params.fit_doors_on_run(run);
			assert!(
				!placed.is_empty(),
				"Les Halles inner wall section {si} ({:?}) has no door on run {run:.2}",
				section.side
			);
			let mut authored = 0usize;
			for (di, door) in placed.iter().enumerate() {
				let Some(door) = clamp_placed_door_to_run(*door, run, INNER_DOOR_END_CLEARANCE)
				else {
					// Required shrink exceeds allowed_error — omit rather than
					// author a boarded / wrong-edge void for the stall strip.
					continue;
				};
				insert_inner_door(
					&mut openings,
					si,
					di,
					section.side,
					center_xz,
					gallery_inner,
					door,
					door_h,
					section.along0,
				);
				authored += 1;
			}
			if authored == 0 {
				// End clearance ate every leaf — keep one unclamped door and let
				// gallery map-time truncate / drop decide.
				insert_inner_door(
					&mut openings,
					si,
					0,
					section.side,
					center_xz,
					gallery_inner,
					placed[0],
					door_h,
					section.along0,
				);
			}
		}

		openings
	}

	/// Midspan breezeways through each outer free panel, plus leftover windows.
	fn generated_arcade_leaf_openings(
		params: &LesHallesParameterized,
		center_xz: Vec3,
		outer: Vec2,
		_gallery_inner: Vec2,
		height: f32,
		shaft_bounds: &[Aabb3d],
		_shaft_slots: &[usize],
	) -> Openings {
		let mut openings = Openings::new();
		let win_h = (height * 0.42).clamp(1.0, height.max(1.0));
		let sill = (height * 0.28).clamp(0.7, height * 0.45);

		let outer_sections =
			Self::outer_free_sections(center_xz, outer, shaft_bounds, params.shaft_placement);

		for (si, section) in outer_sections.iter().enumerate() {
			let run = (section.along1 - section.along0).max(0.0);
			if let Some((width, along_mid)) = arcade_breezeway_on_run(run, section.along0) {
				let mut outer_cut = RectRingFloor::side_passage_opening(
					section.side,
					center_xz,
					outer,
					width,
					height,
				);
				outer_cut.bounds =
					offset_opening_along_side(outer_cut.bounds, section.side, along_mid);
				openings
					.insert(OpeningId::scoped(SCOPE, "outer_breezeway", si.to_string()), outer_cut);

				let lo = along_mid - width * 0.5;
				let hi = along_mid + width * 0.5;
				Self::insert_outer_windows_on_run(
					&mut openings,
					params,
					&AlongSection { side: section.side, along0: section.along0, along1: lo },
					center_xz,
					outer,
					win_h,
					sill,
					si,
					"l",
				);
				Self::insert_outer_windows_on_run(
					&mut openings,
					params,
					&AlongSection { side: section.side, along0: hi, along1: section.along1 },
					center_xz,
					outer,
					win_h,
					sill,
					si,
					"r",
				);
			} else {
				Self::insert_outer_windows_on_run(
					&mut openings,
					params,
					section,
					center_xz,
					outer,
					win_h,
					sill,
					si,
					"",
				);
			}
		}

		openings
	}

	fn insert_outer_windows_on_run(
		openings: &mut Openings,
		params: &LesHallesParameterized,
		section: &AlongSection,
		center_xz: Vec3,
		outer: Vec2,
		win_h: f32,
		sill: f32,
		si: usize,
		tag: &str,
	) {
		let run = (section.along1 - section.along0).max(0.0);
		if run < 0.8 {
			return;
		}
		let placed = params.fit_windows_on_run(run);
		for (wi, win) in placed.iter().enumerate() {
			let along_mid = section.along0 + win.along + win.width * 0.5;
			let mut opening = RectRingFloor::side_aperture_opening(
				section.side,
				center_xz,
				outer,
				win.width,
				win_h,
				sill,
			);
			opening.bounds = offset_opening_along_side(opening.bounds, section.side, along_mid);
			let slot =
				if tag.is_empty() { format!("{si}_{wi}") } else { format!("{si}_{tag}_{wi}") };
			openings.insert(OpeningId::scoped(SCOPE, "outer_aperture", slot), opening);
		}
	}

	/// Large F2C clears at the abutting ends of active corner shafts.
	fn insert_corner_shaft_clears(
		openings: &mut Openings,
		center_xz: Vec3,
		gallery_inner: Vec2,
		height: f32,
		clear_len: f32,
		shaft_slots: &[usize],
	) {
		for &slot in shaft_slots {
			for (side, end_i) in corner_abutments(slot) {
				let half = along_half(side, gallery_inner);
				let clear = clamped_corner_clear(clear_len, half);
				// Clear leaf centered in the occupied end span.
				let along_mid = corner_wall_terminus(half, clear * 0.5, end_i);
				let mut opening = RectRingFloor::side_passage_opening(
					side,
					center_xz,
					gallery_inner,
					clear,
					height,
				);
				opening.bounds = offset_opening_along_side(opening.bounds, side, along_mid);
				openings.insert(
					OpeningId::scoped(SCOPE, "shaft_clear", format!("{}_{end_i}", side_slot(side))),
					opening,
				);
			}
		}
	}

	/// Free outer-wall runs (skip mid-side shaft bands; corners leave the facade free).
	fn outer_free_sections(
		center_xz: Vec3,
		outer: Vec2,
		shaft_bounds: &[Aabb3d],
		placement: LesHallesShaftPlacement,
	) -> Vec<AlongSection> {
		let mut sections = Vec::new();
		for side in RectRingFloorSide::all() {
			let half = along_half(side, outer);
			let occupied = match placement {
				LesHallesShaftPlacement::Corners => Vec::new(),
				LesHallesShaftPlacement::MidSides => {
					Self::shaft_along_spans(center_xz, outer, side, shaft_bounds)
				}
			};
			sections.extend(free_sections_from_occupied(side, half, &occupied));
		}
		sections
	}

	/// ExternalSpace gallery strips for commercial / livable fill.
	///
	/// - **Corners:** stop at the same clear buffer as the inner wall
	///   (`corner_clear_len` past `gallery_inner`). N/S still use the outer
	///   along-axis so inactive corner ends keep the outer corner square; E/W
	///   stay on `gallery_inner` so corners are not double-covered.
	/// - **MidSides:** N/S run the outer length with shaft AABB clears; E/W stay
	///   on `gallery_inner`.
	fn commercial_fill_sections(
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		shaft_bounds: &[Aabb3d],
		shaft_slots: &[usize],
		placement: LesHallesShaftPlacement,
		corner_clear_len: f32,
	) -> Vec<AlongSection> {
		let mut sections = Vec::new();
		for side in RectRingFloorSide::all() {
			let (half, occupied) = match (placement, side) {
				// N/S keep the outer along-axis so inactive corners retain the
				// outer square; stop at the same clear as the inner wall.
				(LesHallesShaftPlacement::Corners, OrthoSide::North | OrthoSide::South) => {
					let half = along_half(side, outer);
					(
						half,
						corner_strip_occupied_spans(
							side,
							half,
							along_half(side, gallery_inner),
							corner_clear_len,
							shaft_slots,
						),
					)
				}
				// E/W stay on gallery_inner so corner squares are not double-covered.
				(LesHallesShaftPlacement::Corners, OrthoSide::East | OrthoSide::West) => {
					let half = along_half(side, gallery_inner);
					(half, corner_occupied_spans(side, half, corner_clear_len, shaft_slots))
				}
				(LesHallesShaftPlacement::MidSides, OrthoSide::North | OrthoSide::South) => {
					let half = along_half(side, outer);
					(half, Self::shaft_along_spans(center_xz, outer, side, shaft_bounds))
				}
				(LesHallesShaftPlacement::MidSides, OrthoSide::East | OrthoSide::West) => {
					let half = along_half(side, gallery_inner);
					(half, Self::shaft_along_spans(center_xz, gallery_inner, side, shaft_bounds))
				}
			};
			sections.extend(free_sections_from_occupied(side, half, &occupied));
		}
		sections
	}

	/// Free inner-wall runs between shaft clears (one section per run).
	fn inner_straight_sections(
		center_xz: Vec3,
		gallery_inner: Vec2,
		shaft_bounds: &[Aabb3d],
		shaft_slots: &[usize],
		placement: LesHallesShaftPlacement,
		corner_clear_len: f32,
	) -> Vec<AlongSection> {
		let mut sections = Vec::new();
		for side in RectRingFloorSide::all() {
			let half = along_half(side, gallery_inner);
			let occupied = match placement {
				LesHallesShaftPlacement::Corners => {
					corner_occupied_spans(side, half, corner_clear_len, shaft_slots)
				}
				LesHallesShaftPlacement::MidSides => {
					Self::shaft_along_spans(center_xz, gallery_inner, side, shaft_bounds)
				}
			};
			sections.extend(free_sections_from_occupied(side, half, &occupied));
		}
		sections
	}

	/// Along-wall spans (relative to side midpoint) occupied by shafts on an outer side.
	fn shaft_along_spans(
		center_xz: Vec3,
		outer: Vec2,
		side: OrthoSide,
		shaft_bounds: &[Aabb3d],
	) -> Vec<(f32, f32)> {
		let ox0 = center_xz.x - outer.x * 0.5;
		let ox1 = center_xz.x + outer.x * 0.5;
		let oz0 = center_xz.z - outer.y * 0.5;
		let oz1 = center_xz.z + outer.y * 0.5;
		let band = 1.0_f32;
		let mut spans = Vec::new();
		for shaft in shaft_bounds {
			let smin = Vec3::from(shaft.min);
			let smax = Vec3::from(shaft.max);
			let near = match side {
				OrthoSide::South => smin.z <= oz0 + band,
				OrthoSide::North => smax.z >= oz1 - band,
				OrthoSide::East => smax.x >= ox1 - band,
				OrthoSide::West => smin.x <= ox0 + band,
			};
			if !near {
				continue;
			}
			let (lo, hi) = match side {
				OrthoSide::North | OrthoSide::South => (smin.x - center_xz.x, smax.x - center_xz.x),
				OrthoSide::East | OrthoSide::West => (smin.z - center_xz.z, smax.z - center_xz.z),
			};
			spans.push((lo.min(hi), lo.max(hi)));
		}
		spans
	}

	/// Which gallery-inner sides a shaft abuts, with along-side offset from mid.
	fn shaft_inner_sides(
		center_xz: Vec3,
		gallery_inner: Vec2,
		shaft: Aabb3d,
	) -> Vec<(OrthoSide, f32)> {
		let mid = Vec3::from((shaft.min + shaft.max) * 0.5);
		let smin = Vec3::from(shaft.min);
		let smax = Vec3::from(shaft.max);
		let gx0 = center_xz.x - gallery_inner.x * 0.5;
		let gx1 = center_xz.x + gallery_inner.x * 0.5;
		let gz0 = center_xz.z - gallery_inner.y * 0.5;
		let gz1 = center_xz.z + gallery_inner.y * 0.5;
		// Shaft sits in the gallery band; treat as abutting if its AABB reaches the
		// inner wall plane (within a small thickness tolerance).
		let tol = 0.35_f32;

		let mut out = Vec::new();
		if aabb_near_plane(smin.z, smax.z, gz0, tol) {
			out.push((OrthoSide::South, mid.x - center_xz.x));
		}
		if aabb_near_plane(smin.z, smax.z, gz1, tol) {
			out.push((OrthoSide::North, mid.x - center_xz.x));
		}
		if aabb_near_plane(smin.x, smax.x, gx1, tol) {
			out.push((OrthoSide::East, mid.z - center_xz.z));
		}
		if aabb_near_plane(smin.x, smax.x, gx0, tol) {
			out.push((OrthoSide::West, mid.z - center_xz.z));
		}
		if out.is_empty() {
			let dists = [
				(OrthoSide::South, (mid.z - gz0).abs(), mid.x - center_xz.x),
				(OrthoSide::North, (mid.z - gz1).abs(), mid.x - center_xz.x),
				(OrthoSide::East, (mid.x - gx1).abs(), mid.z - center_xz.z),
				(OrthoSide::West, (mid.x - gx0).abs(), mid.z - center_xz.z),
			];
			let best = dists
				.into_iter()
				.min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
				.unwrap();
			out.push((best.0, best.2));
		}
		out
	}

	/// Both radial faces of each mid-side shaft (outer → gallery-inner).
	fn build_shaft_walls(
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		height: f32,
		shaft_bounds: &[Aabb3d],
	) -> Vec<Rectangle> {
		let y0 = center_xz.y;
		let outer_f = PlanFrame::from_center_size(center_xz, outer);
		let inner_f = PlanFrame::from_center_size(center_xz, gallery_inner);
		let t = DEFAULT_PANEL_THICKNESS;
		let mut walls = Vec::new();

		for shaft in shaft_bounds {
			let smin = Vec3::from(shaft.min);
			let smax = Vec3::from(shaft.max);
			for (side, _) in Self::shaft_inner_sides(center_xz, gallery_inner, *shaft) {
				let alongs = match side {
					OrthoSide::North | OrthoSide::South => [smin.x, smax.x],
					OrthoSide::East | OrthoSide::West => [smin.z, smax.z],
				};
				for along in alongs {
					walls.push(radial_across_gallery(side, along, y0, height, t, outer_f, inner_f));
				}
			}
		}
		walls.retain(|w| w.edge.length() > EPS);
		walls
	}

	/// One radial per gallery run abutting a corner shaft, at the wall terminus.
	///
	/// Placement matches [`corner_occupied_spans`]: free inner-wall runs end where
	/// the F2C clear starts ([`corner_wall_terminus`]).
	fn build_corner_shaft_end_walls(
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		height: f32,
		clear_len: f32,
		shaft_slots: &[usize],
	) -> Vec<Rectangle> {
		let y0 = center_xz.y;
		let outer_f = PlanFrame::from_center_size(center_xz, outer);
		let inner_f = PlanFrame::from_center_size(center_xz, gallery_inner);
		let t = DEFAULT_PANEL_THICKNESS;
		let mut walls = Vec::new();

		for &slot in shaft_slots {
			for (side, end_i) in corner_abutments(slot) {
				let half = along_half(side, gallery_inner);
				let clear = clamped_corner_clear(clear_len, half);
				let along = corner_wall_terminus(half, clear, end_i);
				let along_world = match side {
					OrthoSide::North | OrthoSide::South => center_xz.x + along,
					OrthoSide::East | OrthoSide::West => center_xz.z + along,
				};
				walls.push(radial_across_gallery(
					side,
					along_world,
					y0,
					height,
					t,
					outer_f,
					inner_f,
				));
			}
		}
		walls.retain(|w| w.edge.length() > EPS);
		walls
	}

	/// Colonnade along free inner-gallery runs (replaces the omitted courtyard wall).
	fn build_arcade_pillars(
		center_xz: Vec3,
		gallery_inner: Vec2,
		height: f32,
		shaft_bounds: &[Aabb3d],
		shaft_slots: &[usize],
		placement: LesHallesShaftPlacement,
		corner_clear_len: f32,
	) -> Vec<PanelPillarLine> {
		let sections = Self::inner_straight_sections(
			center_xz,
			gallery_inner,
			shaft_bounds,
			shaft_slots,
			placement,
			corner_clear_len,
		);
		let mut lines = Vec::new();
		for section in &sections {
			let start =
				along_world_on_inner(section.side, section.along0, center_xz, gallery_inner);
			let end = along_world_on_inner(section.side, section.along1, center_xz, gallery_inner);
			let line = PanelPillarLine::along_rough_stone(
				start,
				end,
				height,
				ARCADE_PILLAR_WIDTH,
				ARCADE_PILLAR_SPACING,
			);
			if !line.is_empty() {
				lines.push(line);
			}
		}
		let sep = ARCADE_PILLAR_WIDTH * 0.85;
		let merged = PanelPillarLine::new(lines.into_iter().flat_map(|l| l.pillars)).dedup_xz(sep);
		if merged.is_empty() {
			Vec::new()
		} else {
			vec![merged]
		}
	}

	fn build_gallery(
		center_xz: Vec3,
		outer: Vec2,
		gallery_inner: Vec2,
		storey_height: f32,
		ceiling: RectRingFloorSlab,
		openings: &Openings,
		inner_walls: bool,
	) -> RectRingFloor {
		RectRingFloor::new(
			RectRingFloorParams::new(center_xz, outer, gallery_inner, storey_height)
				.floor(RectRingFloorSlab::Solid)
				.ceiling(ceiling)
				.inner_walls(inner_walls)
				.openings(openings.clone()),
		)
	}

	/// Floor-only balcony annulus between `gallery_inner` and `courtyard`.
	fn build_balcony_floors(
		center_xz: Vec3,
		gallery_inner: Vec2,
		courtyard: Vec2,
		y0: f32,
	) -> Vec<FittedRectangle> {
		let gx0 = center_xz.x - gallery_inner.x * 0.5;
		let gx1 = center_xz.x + gallery_inner.x * 0.5;
		let gz0 = center_xz.z - gallery_inner.y * 0.5;
		let gz1 = center_xz.z + gallery_inner.y * 0.5;
		let cx0 = center_xz.x - courtyard.x * 0.5;
		let cx1 = center_xz.x + courtyard.x * 0.5;
		let cz0 = center_xz.z - courtyard.y * 0.5;
		let cz1 = center_xz.z + courtyard.y * 0.5;
		let t = DEFAULT_PANEL_THICKNESS;
		let style = PanelStyle::RoughStonework;

		let mut out = Vec::new();
		// North / South take full gallery-inner width; East / West take courtyard depth only.
		if gz1 - cz1 > EPS {
			out.push(floor_rect(style, gx0, gx1, cz1, gz1, y0, t));
		}
		if cz0 - gz0 > EPS {
			out.push(floor_rect(style, gx0, gx1, gz0, cz0, y0, t));
		}
		if cx0 - gx0 > EPS {
			out.push(floor_rect(style, gx0, cx0, cz0, cz1, y0, t));
		}
		if gx1 - cx1 > EPS {
			out.push(floor_rect(style, cx1, gx1, cz0, cz1, y0, t));
		}
		out
	}
}

impl Fit for LesHallesFloorPlan {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = LesHallesParameterized::sample(confines, noise)?;
		Self::from_parameterized(params, confines)
	}
}

impl BuildingComponents for LesHallesFloorPlan {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.gallery.panel_nodes_for_level(level);
		for floor in &self.balcony_floors {
			out.extend(floor.panel_nodes_for_level(level));
		}
		for wall in &self.shaft_walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		for line in &self.arcade_pillars {
			out.extend(line.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.gallery.joint_nodes_for_level(level)
	}
}

/// Free straight run on a gallery side (along coords vs side mid).
///
/// Used for outer facade packing, inner doors, and commercial strip residuals.
#[derive(Debug, Clone, Copy)]
struct AlongSection {
	side: OrthoSide,
	along0: f32,
	along1: f32,
}

fn side_slot(side: OrthoSide) -> &'static str {
	match side {
		OrthoSide::South => "s",
		OrthoSide::East => "e",
		OrthoSide::North => "n",
		OrthoSide::West => "w",
	}
}

/// Corner slot → (side_a, end_a, side_b, end_b) where end `0` is the negative-along end.
fn corner_clear_ends(slot: usize) -> Option<(OrthoSide, usize, OrthoSide, usize)> {
	match slot {
		0 => Some((OrthoSide::South, 0, OrthoSide::West, 0)), // SW
		1 => Some((OrthoSide::South, 1, OrthoSide::East, 0)), // SE
		2 => Some((OrthoSide::North, 1, OrthoSide::East, 1)), // NE
		3 => Some((OrthoSide::North, 0, OrthoSide::West, 1)), // NW
		_ => None,
	}
}

/// Abutting `(side, end_i)` pairs for an active corner shaft slot.
fn corner_abutments(slot: usize) -> impl Iterator<Item = (OrthoSide, usize)> {
	corner_clear_ends(slot)
		.into_iter()
		.flat_map(|(a, ea, b, eb)| [(a, ea), (b, eb)])
}

fn corner_end_on_side(slot: usize, side: OrthoSide) -> Option<usize> {
	corner_abutments(slot).find(|(s, _)| *s == side).map(|(_, end_i)| end_i)
}

/// Inner-wall / E-W strip spans occupied by corner clears on `side`.
fn corner_occupied_spans(
	side: OrthoSide,
	half: f32,
	clear_len: f32,
	shaft_slots: &[usize],
) -> Vec<(f32, f32)> {
	let clear = clamped_corner_clear(clear_len, half);
	let mut occupied = Vec::new();
	for &slot in shaft_slots {
		let Some(end_i) = corner_end_on_side(slot, side) else {
			continue;
		};
		if end_i == 0 {
			occupied.push((-half, corner_wall_terminus(half, clear, 0)));
		} else {
			occupied.push((corner_wall_terminus(half, clear, 1), half));
		}
	}
	occupied
}

/// N/S strip occupied spans for corner shafts on the **outer** along-axis.
///
/// Removes the corner gallery square plus the wall clear buffer so residuals
/// stop where the inner wall already opens. Inactive corners keep the outer end free.
fn corner_strip_occupied_spans(
	side: OrthoSide,
	outer_half: f32,
	gallery_inner_half: f32,
	clear_len: f32,
	shaft_slots: &[usize],
) -> Vec<(f32, f32)> {
	let clear = clamped_corner_clear(clear_len, gallery_inner_half);
	let mut occupied = Vec::new();
	for &slot in shaft_slots {
		let Some(end_i) = corner_end_on_side(slot, side) else {
			continue;
		};
		if end_i == 0 {
			occupied.push((-outer_half, corner_wall_terminus(gallery_inner_half, clear, 0)));
		} else {
			occupied.push((corner_wall_terminus(gallery_inner_half, clear, 1), outer_half));
		}
	}
	occupied
}

/// Radial wall across gallery depth on `side` at a world along-coordinate.
///
/// N/S: `along_world` is X; E/W: `along_world` is Z. Edge runs outer → gallery-inner.
fn radial_across_gallery(
	side: OrthoSide,
	along_world: f32,
	y0: f32,
	height: f32,
	thickness: f32,
	outer: PlanFrame,
	inner: PlanFrame,
) -> Rectangle {
	match side {
		OrthoSide::South => radial_wall(
			Vec3::new(along_world, y0, outer.z0),
			Vec3::new(0.0, 0.0, inner.z0 - outer.z0),
			height,
			thickness,
		),
		OrthoSide::North => radial_wall(
			Vec3::new(along_world, y0, outer.z1),
			Vec3::new(0.0, 0.0, inner.z1 - outer.z1),
			height,
			thickness,
		),
		OrthoSide::East => radial_wall(
			Vec3::new(outer.x1, y0, along_world),
			Vec3::new(inner.x1 - outer.x1, 0.0, 0.0),
			height,
			thickness,
		),
		OrthoSide::West => radial_wall(
			Vec3::new(outer.x0, y0, along_world),
			Vec3::new(inner.x0 - outer.x0, 0.0, 0.0),
			height,
			thickness,
		),
	}
}

fn insert_inner_door(
	openings: &mut Openings,
	si: usize,
	di: usize,
	side: OrthoSide,
	center_xz: Vec3,
	gallery_inner: Vec2,
	door: LesHallesPlacedDoor,
	door_h: f32,
	section_along0: f32,
) {
	let along_mid = section_along0 + door.along + door.width * 0.5;
	let mut opening =
		RectRingFloor::side_passage_opening(side, center_xz, gallery_inner, door.width, door_h);
	opening.bounds = offset_opening_along_side(opening.bounds, side, along_mid);
	openings.insert(OpeningId::scoped(SCOPE, "inner_door", format!("{si}_{di}")), opening);
}

/// Shrink a packed leaf so it stays `clearance` inside the free run.
///
/// Charges shrink against [`LesHallesPlacedDoor::allowed_error`]. Returns
/// [`None`] when the required shrink exceeds that budget (or the leaf becomes
/// unusably narrow).
fn clamp_placed_door_to_run(
	mut door: LesHallesPlacedDoor,
	run: f32,
	clearance: f32,
) -> Option<LesHallesPlacedDoor> {
	let clearance = clearance.clamp(0.0, run * 0.45);
	let mut budget = door.allowed_error.max(0.0);
	let lo = clearance;
	let hi = (run - clearance).max(lo);
	if hi - lo < 0.5 {
		// Run too short for end clearance — keep packed placement; map-time
		// truncate / drop still applies.
		return Some(door);
	}
	if door.along < lo {
		let shrink = lo - door.along;
		if shrink > budget + 1e-3 {
			return None;
		}
		door.width = (door.width - shrink).max(0.4);
		door.along = lo;
		budget -= shrink;
	}
	if door.along + door.width > hi {
		let shrink = door.along + door.width - hi;
		if shrink > budget + 1e-3 {
			return None;
		}
		door.width = (door.width - shrink).max(0.4);
	}
	if door.width + 1e-3 < 0.4 || door.along + door.width > run + 1e-3 {
		return None;
	}
	door.allowed_error = budget;
	Some(door)
}

/// Openings whose AABB intersects `bounds` (shaft volumes excluded — those stay
/// on shaft [`SpaceKind::InternalSpace`] cells). Wall clears remain [`OpeningLabel::Passage`].
fn subset_openings_intersecting(openings: &Openings, bounds: &Aabb3d) -> Openings {
	let mut out = Openings::new();
	let region = Aabb2d {
		min: Vec2::new(Vec3::from(bounds.min).x, Vec3::from(bounds.min).z),
		max: Vec2::new(Vec3::from(bounds.max).x, Vec3::from(bounds.max).z),
	};
	let y0 = Vec3::from(bounds.min).y;
	let y1 = Vec3::from(bounds.max).y;
	for (id, opening) in openings.iter() {
		if matches!(opening.label, OpeningLabel::Shaft) {
			continue;
		}
		if aabb_xz_overlap_area(&opening.bounds, &region) <= 1e-4 {
			continue;
		}
		let omin = Vec3::from(opening.bounds.min);
		let omax = Vec3::from(opening.bounds.max);
		if omax.y < y0 - 1e-3 || omin.y > y1 + 1e-3 {
			continue;
		}
		out.insert(id.clone(), opening.clone());
	}
	out
}

fn free_sections_from_occupied(
	side: OrthoSide,
	half: f32,
	occupied: &[(f32, f32)],
) -> Vec<AlongSection> {
	let mut occupied: Vec<(f32, f32)> = occupied.to_vec();
	occupied.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
	let mut sections = Vec::new();
	let mut cursor = -half;
	for (lo, hi) in occupied {
		let lo = lo.clamp(-half, half);
		let hi = hi.clamp(-half, half);
		if lo > cursor + 0.4 {
			sections.push(AlongSection { side, along0: cursor, along1: lo });
		}
		cursor = cursor.max(hi);
	}
	if half > cursor + 0.4 {
		sections.push(AlongSection { side, along0: cursor, along1: half });
	}
	sections
}

/// Greatest XZ overlap with a mapping region; closest region center if no overlap.
fn best_shaft_slot(request: &Aabb3d, regions: &[Aabb2d]) -> Option<usize> {
	if regions.is_empty() {
		return None;
	}
	let rc = aabb_xz_center(request);

	let mut best_i = 0usize;
	let mut best_area = -1.0_f32;
	let mut best_dist = f32::INFINITY;
	for (i, region) in regions.iter().enumerate() {
		let area = aabb_xz_overlap_area(request, region);
		let cx = (region.min.x + region.max.x) * 0.5;
		let cz = (region.min.y + region.max.y) * 0.5;
		let dist = (rc.x - cx).hypot(rc.y - cz);
		let better = area > best_area + 1e-6
			|| ((area - best_area).abs() <= 1e-6 && dist < best_dist - 1e-6);
		if better {
			best_i = i;
			best_area = area;
			best_dist = dist;
		}
	}
	Some(best_i)
}

fn offset_opening_along_side(bounds: Aabb3d, side: OrthoSide, delta: f32) -> Aabb3d {
	let (dx, dz) = match side {
		OrthoSide::North | OrthoSide::South => (delta, 0.0),
		OrthoSide::East | OrthoSide::West => (0.0, delta),
	};
	let min = Vec3::from(bounds.min) + Vec3::new(dx, 0.0, dz);
	let max = Vec3::from(bounds.max) + Vec3::new(dx, 0.0, dz);
	Aabb3d::from_min_max(min, max)
}

fn arcade_breezeway_on_run(run: f32, along0: f32) -> Option<(f32, f32)> {
	if run < MIN_BREEZEWAY_WIDTH + 2.0 * BREEZEWAY_SIDE_MARGIN {
		return None;
	}
	let width = (run * BREEZEWAY_RUN_FRAC)
		.clamp(MIN_BREEZEWAY_WIDTH, MAX_BREEZEWAY_WIDTH)
		.min(run - 2.0 * BREEZEWAY_SIDE_MARGIN);
	if width < MIN_BREEZEWAY_WIDTH {
		return None;
	}
	Some((width, along0 + run * 0.5))
}

fn radial_wall(origin: Vec3, edge: Vec3, height: f32, thickness: f32) -> Rectangle {
	Rectangle::rough_stone(origin, edge, height, thickness, 0.0)
}

fn floor_rect(
	style: PanelStyle,
	min_x: f32,
	max_x: f32,
	min_z: f32,
	max_z: f32,
	y: f32,
	thickness: f32,
) -> FittedRectangle {
	let plan = PlanRect::new(
		Vec3::new(0.5 * (min_x + max_x), y, 0.5 * (min_z + max_z)),
		(max_x - min_x).max(EPS),
		(max_z - min_z).max(EPS),
	);
	let t = thickness.max(1e-4);
	FittedRectangle::new(
		style,
		PanelPoint::new(plan.sw(), t),
		PanelPoint::new(plan.se(), t),
		PanelPoint::new(plan.nw(), t),
		PanelPoint::new(plan.ne(), t),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fit::{aabb_xz_near_eq, Fit};
	use crate::openings::{MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use procedural_common::{NoiseConfig, NoiseParams};

	fn nominal_confines() -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-24.0, 0.0, -18.0),
			Vec3::new(24.0, 4.0, 18.0),
		))
	}

	fn tiny_confines() -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(3.0, 3.0, 3.0)))
	}

	fn fixed_params(placement: LesHallesShaftPlacement) -> LesHallesParameterized {
		let cfg = NoiseConfig::new(NoiseParams::default());
		LesHallesParameterized {
			gallery_width: 6.0,
			balcony_width: 4.0,
			courtyard_fraction: 0.5,
			shaft_placement: placement,
			mid_shaft_side: 4.0,
			opening_density: 0.7,
			doors: gen_stall_doors(&cfg, Vec3::ZERO),
			windows: gen_windows(&cfg, Vec3::ZERO),
		}
	}

	fn confines_with_all_shafts(params: &LesHallesParameterized) -> Confines {
		let base = nominal_confines();
		let openings = LesHallesFloorPlan::shaft_requests_for_all_slots(params, &base);
		Confines::new(base.bounds, 0.0, openings)
	}

	fn fit_with_all_shafts(
		noise: NoiseParams,
	) -> (LesHallesFloorPlan, crate::fit::FillableRegions) {
		let base = nominal_confines();
		let params = LesHallesParameterized::sample(&base, noise).unwrap();
		let confines = confines_with_all_shafts(&params);
		LesHallesFloorPlan::from_parameterized(params, &confines).unwrap()
	}

	#[test]
	fn too_small_footprint_rejects() {
		let err = LesHallesFloorPlan::fit_to_confines(&tiny_confines(), NoiseParams::default())
			.expect_err("tiny footprint");
		assert!(matches!(err, FitError::TooSmall { reason: "footprint" }));
	}

	#[test]
	fn sample_is_deterministic() {
		let c = nominal_confines();
		let noise = NoiseParams { seed: 42, ..NoiseParams::default() };
		let a = LesHallesParameterized::sample(&c, noise).unwrap();
		let b = LesHallesParameterized::sample(&c, noise).unwrap();
		assert_eq!(a.shaft_placement, b.shaft_placement);
		assert!((a.gallery_width - b.gallery_width).abs() < 1e-5);
		assert!((a.balcony_width - b.balcony_width).abs() < 1e-5);
	}

	#[test]
	fn gallery_has_no_ceiling_by_default_and_balcony_has_floors() {
		let (plan, _) = fit_with_all_shafts(NoiseParams::default());
		assert!(!plan.gallery.has_ceiling());
		assert!(plan.gallery.has_floor());
		assert!(!plan.balcony_floors.is_empty());
		assert!(plan.gallery_inner.x > plan.courtyard.x);
		assert!(plan.outer.x > plan.gallery_inner.x);
	}

	#[test]
	fn emits_outer_apertures_and_inner_doors_per_section() {
		let (plan, _) = fit_with_all_shafts(NoiseParams::default());
		// Outer: apertures only.
		assert!(plan.openings.iter().all(|(id, o)| !id.as_str().contains("outer_passage")
			&& !(id.as_str().contains("outer_") && matches!(o.label, OpeningLabel::Passage))));
		assert!(plan.openings.iter().any(|(id, o)| id.as_str().contains("outer_aperture")
			&& matches!(o.label, OpeningLabel::Aperture)));
		// Inner: doors only (no shop apertures); at least one per straight section.
		assert!(plan.openings.iter().all(|(id, _)| !id.as_str().contains("inner_aperture")));
		let expected = plan.parameterized.expected_inner_section_count(plan.shaft_bounds.len());
		let section_doors: std::collections::HashSet<usize> = plan
			.openings
			.iter()
			.filter_map(|(id, _)| {
				let s = id.as_str();
				let rest = s.strip_prefix("les_halles_inner_door_")?;
				let (si, _) = rest.split_once('_')?;
				si.parse().ok()
			})
			.collect();
		assert_eq!(section_doors.len(), expected);
		// Every retained Passage/Aperture must be cut into the gallery (no boarded
		// voids left for commercial strips to seed on).
		for (id, opening) in plan.openings.iter() {
			if matches!(opening.label, OpeningLabel::Passage | OpeningLabel::Aperture) {
				assert!(
					plan.gallery.mapped_opening(id).is_some(),
					"unmapped connectable {} survived sync",
					id.as_str()
				);
			}
		}
		use crate::openings::MapsOpenings;
		let outer_id = plan
			.openings
			.iter()
			.find(|(id, _)| id.as_str().contains("outer_aperture_"))
			.map(|(id, _)| id.clone())
			.expect("outer aperture");
		assert!(plan.gallery.mapped_opening(&outer_id).is_some());
		assert!(!plan.shaft_walls.is_empty());
	}

	#[test]
	fn arcade_program_cuts_midspan_breezeways() {
		let base = nominal_confines();
		let params = LesHallesParameterized::sample(&base, NoiseParams::default()).unwrap();
		let confines = confines_with_all_shafts(&params);
		let (plan, _) = LesHallesFloorPlan::from_parameterized_with(
			params,
			&confines,
			RectRingFloorSlab::None,
			LesHallesOpeningProgram::GroundArcade,
		)
		.unwrap();
		assert!(plan.openings.iter().any(|(id, o)| {
			id.as_str().contains("outer_breezeway") && matches!(o.label, OpeningLabel::Passage)
		}));
		assert!(plan.openings.iter().all(|(id, _)| !id.as_str().contains("inner_breezeway")));
		assert!(plan.openings.iter().all(|(id, _)| !id.as_str().contains("inner_door")));
		assert!(plan.openings.iter().any(|(id, o)| {
			id.as_str().contains("outer_aperture") && matches!(o.label, OpeningLabel::Aperture)
		}));
		assert_eq!(plan.gallery.wall_count(), 4, "arcade omits the inner courtyard wall loop");
		assert!(
			plan.arcade_pillars.iter().any(|l| !l.is_empty()),
			"arcade colonnade should stand on the courtyard line"
		);
		for (id, opening) in plan.openings.iter() {
			if matches!(opening.label, OpeningLabel::Passage | OpeningLabel::Aperture) {
				assert!(
					plan.gallery.mapped_opening(id).is_some(),
					"unmapped arcade connectable {}",
					id.as_str()
				);
			}
		}
	}

	#[test]
	fn no_shafts_without_inbound_requests() {
		let (plan, regions) =
			LesHallesFloorPlan::fit_to_confines(&nominal_confines(), NoiseParams::default())
				.unwrap();
		assert!(plan.shaft_bounds.is_empty());
		assert!(plan.shaft_walls.is_empty());
		assert!(plan.openings.iter().all(|(id, _)| !id.as_str().contains("shaft")));
		let externals =
			regions.within.iter().filter(|r| r.kind == SpaceKind::ExternalSpace).count();
		let walkways = regions.within.iter().filter(|r| r.kind == SpaceKind::Walkway).count();
		assert_eq!(externals, 4);
		assert_eq!(walkways, 4);
		assert_eq!(regions.within.len(), 8); // strips + balcony; no shafts
	}

	#[test]
	fn playground_seed_retains_only_mapped_passages() {
		// `/show les-halles-full-storey --extent 48,4,36 --seed 1337`
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-24.0, 0.0, -18.0),
			Vec3::new(24.0, 4.0, 18.0),
		));
		let noise = NoiseParams { seed: 1337, ..NoiseParams::default() };
		let (plan, regions) = LesHallesFloorPlan::fit_to_confines(&confines, noise).unwrap();
		let mut passage_n = 0usize;
		for (id, opening) in plan.openings.iter() {
			if matches!(opening.label, OpeningLabel::Passage) {
				passage_n += 1;
				assert!(
					plan.gallery.mapped_opening(id).is_some(),
					"boarded passage {} leaked into plan.openings",
					id.as_str()
				);
			}
		}
		assert!(passage_n > 0, "expected authored inner doors");
		// External strips must not see unmapped passages either.
		for region in regions.within.iter().filter(|r| r.kind == SpaceKind::ExternalSpace) {
			for (id, opening) in region.confines.openings.iter() {
				if matches!(opening.label, OpeningLabel::Passage) {
					assert!(
						plan.gallery.mapped_opening(id).is_some(),
						"strip carries unmapped passage {}",
						id.as_str()
					);
				}
			}
		}
	}

	#[test]
	fn awkward_se_passage_maps_despite_nearer_east_mid() {
		// Playground repro: SE corner door closer to East mid than South mid, but
		// the AABB only intersects the South outer wall volume.
		let base = nominal_confines();
		let params = LesHallesParameterized::sample(
			&base,
			NoiseParams { seed: 42, ..NoiseParams::default() },
		)
		.unwrap();
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("awkward"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(20.5, 0.2, -19.0),
				Vec3::new(23.5, 3.5, -17.9),
			)),
		);
		let confines = Confines::new(base.bounds, 0.0, openings);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(params, &confines).unwrap();
		use crate::openings::MapsOpenings;
		assert!(
			plan.gallery.mapped_opening(&OpeningId::new("awkward")).is_some(),
			"inbound SE passage must cut the South outer wall"
		);
	}

	#[test]
	fn preserves_inbound_opening_ids_and_emits_scoped_shafts() {
		let base = nominal_confines();
		let params = LesHallesParameterized::sample(&base, NoiseParams::default()).unwrap();
		let mut openings = LesHallesFloorPlan::shaft_requests_for_all_slots(&params, &base);
		openings.insert(
			OpeningId::new("inbound_door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(-0.5, 0.0, -18.2),
				Vec3::new(0.5, 2.2, -17.8),
			)),
		);
		let confines = Confines::new(base.bounds, 0.0, openings);
		let (plan, regions) = LesHallesFloorPlan::from_parameterized(params, &confines).unwrap();
		assert!(plan.openings.get(&OpeningId::new("inbound_door")).is_some());
		assert_eq!(plan.shaft_bounds.len(), 4);
		assert!(plan.openings.get(&OpeningId::scoped(SCOPE, "shaft", "0")).is_some());
		assert!(matches!(
			plan.openings.get(&OpeningId::scoped(SCOPE, "shaft", "0")).unwrap().label,
			OpeningLabel::Shaft
		));
		assert!(plan.gallery.wall_count() >= 4);
		assert!(regions.within.iter().filter(|r| r.kind == SpaceKind::ExternalSpace).count() >= 4);
		assert_eq!(regions.within.iter().filter(|r| r.kind == SpaceKind::InternalSpace).count(), 4);
		assert_eq!(regions.atop.len(), 1);
	}

	#[test]
	fn from_parameterized_matches_fit_structure() {
		let noise = NoiseParams { seed: 7, ..NoiseParams::default() };
		let base = nominal_confines();
		let params = LesHallesParameterized::sample(&base, noise).unwrap();
		let confines = confines_with_all_shafts(&params);
		let (via_params, _) =
			LesHallesFloorPlan::from_parameterized(params.clone(), &confines).unwrap();
		let (via_again, _) = LesHallesFloorPlan::from_parameterized(params, &confines).unwrap();
		assert_eq!(via_params.outer, via_again.outer);
		assert_eq!(via_params.gallery_inner, via_again.gallery_inner);
		assert_eq!(via_params.courtyard, via_again.courtyard);
		assert_eq!(
			via_params.parameterized.shaft_placement,
			via_again.parameterized.shaft_placement
		);
		assert_eq!(via_params.shaft_bounds.len(), 4);
	}

	#[test]
	fn corner_shaft_is_half_of_gallery_square_with_large_clears() {
		let params = fixed_params(LesHallesShaftPlacement::Corners);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(
			params.clone(),
			&confines_with_all_shafts(&params),
		)
		.unwrap();
		let gw = plan.parameterized.gallery_width;
		let shaft = &plan.shaft_bounds[0];
		let smin = Vec3::from(shaft.min);
		let smax = Vec3::from(shaft.max);
		assert!((smax.x - smin.x - gw * 0.5).abs() < 1e-3);
		assert!((smax.z - smin.z - gw * 0.5).abs() < 1e-3);
		let clears = plan
			.openings
			.iter()
			.filter(|(id, _)| id.as_str().contains("shaft_clear_"))
			.count();
		assert_eq!(clears, 8, "two clears per active corner");
		assert_eq!(
			plan.shaft_walls.len(),
			plan.shaft_bounds.len() * 2,
			"one end wall per abutting gallery run (2 per corner shaft)"
		);
	}

	#[test]
	fn corner_end_walls_sit_at_inner_wall_terminus() {
		let params = fixed_params(LesHallesShaftPlacement::Corners);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(
			params.clone(),
			&confines_with_all_shafts(&params),
		)
		.unwrap();
		let half = along_half(OrthoSide::South, plan.gallery_inner);
		let clear = clamped_corner_clear(plan.parameterized.corner_clear_len(), half);
		let expected_x = plan.center_xz.x + corner_wall_terminus(half, clear, 1);
		let outer_f = PlanFrame::from_center_size(plan.center_xz, plan.outer);
		let inner_f = PlanFrame::from_center_size(plan.center_xz, plan.gallery_inner);
		let se_south = plan.shaft_walls.iter().find(|w| {
			(w.origin.x - expected_x).abs() < 1e-3
				&& (w.origin.z - outer_f.z0).abs() < 1e-3
				&& (w.edge.z - (inner_f.z0 - outer_f.z0)).abs() < 1e-3
		});
		assert!(
			se_south.is_some(),
			"SE south end wall must sit at inner-wall clear start x={expected_x}"
		);
	}

	#[test]
	fn corner_shafts_cut_gallery_floor_at_every_slot() {
		let params = fixed_params(LesHallesShaftPlacement::Corners);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(
			params.clone(),
			&confines_with_all_shafts(&params),
		)
		.unwrap();
		assert_eq!(plan.shaft_bounds.len(), 4);
		for (i, shaft) in plan.shaft_bounds.iter().enumerate() {
			let mid = Vec3::from((shaft.min + shaft.max) * 0.5);
			assert!(
				!plan.gallery.floor_covers_xz(mid.x, mid.z),
				"shaft {i} center ({}, {}) must be a floor cutout",
				mid.x,
				mid.z
			);
		}
	}

	#[test]
	fn corner_external_strips_stop_at_shaft_clear_buffer() {
		let params = fixed_params(LesHallesShaftPlacement::Corners);
		let (plan, regions) = LesHallesFloorPlan::from_parameterized(
			params.clone(),
			&confines_with_all_shafts(&params),
		)
		.unwrap();
		let half = along_half(OrthoSide::South, plan.gallery_inner);
		let clear = clamped_corner_clear(plan.parameterized.corner_clear_len(), half);
		// Just inside the south-east clear buffer (should not be ExternalSpace).
		let in_buffer = Vec3::new(
			plan.center_xz.x + half - clear * 0.5,
			plan.center_xz.y + 0.5,
			plan.center_xz.z - plan.outer.y * 0.5 + plan.parameterized.gallery_width * 0.5,
		);
		let in_strip = regions.within.iter().any(|r| {
			r.kind == SpaceKind::ExternalSpace && aabb_contains_xz_y(&r.confines.bounds, in_buffer)
		});
		assert!(!in_strip, "corner clear buffer must not be covered by ExternalSpace residuals");
		// Mid-side of the south strip (should still be fillable).
		let mid_south = Vec3::new(
			plan.center_xz.x,
			plan.center_xz.y + 0.5,
			plan.center_xz.z - plan.outer.y * 0.5 + plan.parameterized.gallery_width * 0.5,
		);
		assert!(
			regions.within.iter().any(|r| {
				r.kind == SpaceKind::ExternalSpace
					&& aabb_contains_xz_y(&r.confines.bounds, mid_south)
			}),
			"south strip mid must remain ExternalSpace"
		);
	}

	#[test]
	fn mid_side_shafts_keep_radial_walls() {
		let params = fixed_params(LesHallesShaftPlacement::MidSides);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(
			params.clone(),
			&confines_with_all_shafts(&params),
		)
		.unwrap();
		assert!(!plan.shaft_bounds.is_empty());
		assert!(!plan.shaft_walls.is_empty());
	}

	#[test]
	fn corner_shaft_mapping_rewrites_inbound_by_quadrant() {
		// Request in SE quadrant (and one straddling SE/NE — NE wins by overlap).
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("req_se"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(2.0, 0.0, -6.0), Vec3::new(4.0, 3.0, -4.0)),
				OpeningLabel::Shaft,
			),
		);
		openings.insert(
			OpeningId::new("req_straddle"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(1.0, 0.0, -1.0), Vec3::new(5.0, 3.0, 3.0)),
				OpeningLabel::Shaft,
			),
		);
		let confines = Confines::new(nominal_confines().bounds, 0.0, openings);
		let (plan, regions) = LesHallesFloorPlan::from_parameterized(
			fixed_params(LesHallesShaftPlacement::Corners),
			&confines,
		)
		.unwrap();

		assert_eq!(plan.shaft_bounds.len(), 2);
		assert!(plan.shaft_slots.contains(&1));
		assert!(plan.shaft_slots.contains(&2));
		let se_idx = plan.shaft_slots.iter().position(|&s| s == 1).unwrap();
		let ne_idx = plan.shaft_slots.iter().position(|&s| s == 2).unwrap();
		let se_open = plan.openings.get(&OpeningId::new("req_se")).unwrap();
		assert!(aabb_xz_near_eq(&se_open.bounds, &plan.shaft_bounds[se_idx], 1e-4));
		assert!(plan.shaft_inbound[se_idx].contains(&OpeningId::new("req_se")));

		let straddle = plan.openings.get(&OpeningId::new("req_straddle")).unwrap();
		assert!(aabb_xz_near_eq(&straddle.bounds, &plan.shaft_bounds[ne_idx], 1e-4));
		assert!(plan.shaft_inbound[ne_idx].contains(&OpeningId::new("req_straddle")));
		assert!(regions.within.iter().any(|r| {
			r.kind == SpaceKind::InternalSpace
				&& r.confines.openings.get(&OpeningId::new("req_se")).is_some()
		}));
	}

	#[test]
	fn external_strips_reach_outer_corners_without_mid_shafts() {
		let (plan, regions) =
			LesHallesFloorPlan::fit_to_confines(&nominal_confines(), NoiseParams::default())
				.unwrap();
		assert!(plan.shaft_bounds.is_empty());
		let ox1 = plan.center_xz.x + plan.outer.x * 0.5;
		let oz0 = plan.center_xz.z - plan.outer.y * 0.5;
		// SE outer corner sample point inside the gallery band.
		let se = Vec3::new(ox1 - 0.5, plan.center_xz.y + 0.5, oz0 + 0.5);
		let covered = regions.within.iter().any(|r| {
			r.kind == SpaceKind::ExternalSpace && aabb_contains_xz_y(&r.confines.bounds, se)
		});
		assert!(covered, "SE gallery corner must be inside an ExternalSpace strip");
	}

	fn aabb_contains_xz_y(bounds: &Aabb3d, p: Vec3) -> bool {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		p.x >= min.x - 1e-3
			&& p.x <= max.x + 1e-3
			&& p.y >= min.y - 1e-3
			&& p.y <= max.y + 1e-3
			&& p.z >= min.z - 1e-3
			&& p.z <= max.z + 1e-3
	}

	#[test]
	fn external_strips_carry_subsetted_facade_openings() {
		let (plan, regions) = fit_with_all_shafts(NoiseParams::default());
		let strips: Vec<_> =
			regions.within.iter().filter(|r| r.kind == SpaceKind::ExternalSpace).collect();
		let expected = plan.parameterized.expected_inner_section_count(plan.shaft_bounds.len());
		assert_eq!(strips.len(), expected);
		assert!(
			strips.iter().any(|r| {
				r.confines.openings.iter().any(|(id, _)| {
					id.as_str().contains("inner_door") || id.as_str().contains("outer_aperture")
				})
			}),
			"at least one strip should inherit facade openings"
		);
	}

	#[test]
	fn midside_shaft_mapping_uses_end_and_middle_bands() {
		let mut openings = Openings::new();
		// South end band
		openings.insert(
			OpeningId::new("req_s"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(-1.0, 0.0, -17.0), Vec3::new(1.0, 3.0, -14.0)),
				OpeningLabel::Shaft,
			),
		);
		// East middle band
		openings.insert(
			OpeningId::new("req_e"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(16.0, 0.0, -1.0), Vec3::new(22.0, 3.0, 1.0)),
				OpeningLabel::Shaft,
			),
		);
		let confines = Confines::new(nominal_confines().bounds, 0.0, openings);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(
			fixed_params(LesHallesShaftPlacement::MidSides),
			&confines,
		)
		.unwrap();

		assert_eq!(plan.shaft_bounds.len(), 2);
		let s_idx = plan.shaft_slots.iter().position(|&s| s == 0).unwrap();
		let e_idx = plan.shaft_slots.iter().position(|&s| s == 1).unwrap();
		assert!(aabb_xz_near_eq(
			&plan.openings.get(&OpeningId::new("req_s")).unwrap().bounds,
			&plan.shaft_bounds[s_idx],
			1e-4
		));
		assert!(aabb_xz_near_eq(
			&plan.openings.get(&OpeningId::new("req_e")).unwrap().bounds,
			&plan.shaft_bounds[e_idx],
			1e-4
		));
		assert!(plan.shaft_inbound[s_idx].contains(&OpeningId::new("req_s")));
		assert!(plan.shaft_inbound[e_idx].contains(&OpeningId::new("req_e")));
		assert!(plan.parameterized.mid_shaft_side >= 2.4);
	}
}
