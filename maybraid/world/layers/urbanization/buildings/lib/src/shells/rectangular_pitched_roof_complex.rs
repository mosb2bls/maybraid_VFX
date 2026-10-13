//! Orthogonal AABB roof complex: pitched roofs with true valleys at L/T junctions.
//!
//! Each [`Aabb3d`] is a roof massing box (top midline = ridge, bottom long edges =
//! wall plates). Side overhang expands eaves; free ends take [`EndCap`] hip/gable.
//! Concave plan corners form valleys via facing pitch-plane intersection, and
//! neighboring [`PitchedRoof`]s are truncated to meet on those valleys.

mod decompose;
mod geometry;
mod openings;
mod plan_holes;
mod topology;
mod valleys;

#[cfg(test)]
mod tests;

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers, MassingVolume};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;

use crate::openings::{MappedOpenings, OpeningLabel, Openings};
use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;
use crate::shells::pitched_rectangular_roof::{PitchedRoof, PitchedRoofParams, RoofHalf};

use decompose::decompose_volumes;
use geometry::VolumeCandidate;
use openings::apply_openings;
use plan_holes::punch_plan_holes;
use topology::resolve_junctions;
pub use valleys::ValleySegment;
use valleys::{apply_valleys, finish_coaxial_ridge_meets};

/// Side-eave overhang policy (also used for gable end projections).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overhang {
	/// Absolute meters.
	Fixed(f32),
	/// Fraction of the eave-to-eave (short-axis) span.
	Ratio(f32),
}

impl Default for Overhang {
	fn default() -> Self {
		Self::Fixed(0.3)
	}
}

impl Overhang {
	/// Metres of side-eave (or barge) projection for a massing whose short span is `short_span`.
	pub fn resolve(self, short_span: f32) -> f32 {
		match self {
			Self::Fixed(v) => v.max(0.0),
			Self::Ratio(r) => (r.max(0.0) * short_span).max(0.0),
		}
	}
}

/// Free-end treatment for massing boxes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndCap {
	/// Banked hips on free ends.
	Hip,
	/// Gable ends; ridge / eave may project past the massing by the given overhangs.
	Gable { ridge: Overhang, eave: Overhang },
}

impl Default for EndCap {
	fn default() -> Self {
		Self::Hip
	}
}

/// How unequal ridges meet at a valley junction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RidgeJunction {
	/// Blend junction height from the lower ridge (`0`) toward the higher (`1`).
	RunUp(f32),
}

impl Default for RidgeJunction {
	fn default() -> Self {
		Self::RunUp(0.0)
	}
}

impl RidgeJunction {
	/// Junction ridge height for two authored ridge elevations.
	pub fn resolve(self, y_a: f32, y_b: f32) -> f32 {
		match self {
			Self::RunUp(t) => {
				let t = t.clamp(0.0, 1.0);
				let lo = y_a.min(y_b);
				let hi = y_a.max(y_b);
				lo + (hi - lo) * t
			}
		}
	}
}

/// Authored parameters for a [`RectangularPitchedRoofComplex`].
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularPitchedRoofComplexParams {
	pub volumes: Vec<Aabb3d>,
	pub overhang: Overhang,
	pub end_cap: EndCap,
	/// Unequal-ridge meet policy at valleys.
	pub ridge_junction: RidgeJunction,
	/// World-space voids applied after geometry is solved (nearest roof wins).
	pub openings: Openings,
	pub style: PanelStyle,
	pub joint_thickness: f32,
}

impl Default for RectangularPitchedRoofComplexParams {
	fn default() -> Self {
		Self::l_shape()
	}
}

impl RectangularPitchedRoofComplexParams {
	pub fn new(volumes: Vec<Aabb3d>) -> Self {
		Self {
			volumes,
			overhang: Overhang::default(),
			end_cap: EndCap::default(),
			ridge_junction: RidgeJunction::default(),
			openings: Openings::new(),
			style: PanelStyle::ShepherdsThatch,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
		}
	}

	/// Single massing box centered on X, long along X.
	pub fn single(extent_x: f32, extent_z: f32, y0: f32, y1: f32) -> Self {
		let hx = extent_x * 0.5;
		let hz = extent_z * 0.5;
		Self::new(vec![Aabb3d::from_min_max(Vec3::new(-hx, y0, -hz), Vec3::new(hx, y1, hz))])
	}

	/// L: long-X bar + long-Z stem sharing the −X/−Z corner.
	pub fn l_shape() -> Self {
		// A: x[-2, 8] × z[-2, 2]  (long X)
		// B: x[-2, 2] × z[-2, 8]  (long Z)
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(8.0, 4.5, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(2.0, 4.5, 8.0)),
		])
	}

	/// L with a taller stem ridge (same eave plate, different ridge heights).
	pub fn l_shape_stepped_ridge() -> Self {
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(8.0, 4.2, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(2.0, 5.5, 8.0)),
		])
	}

	/// L with a raised stem volume (higher eave plate and ridge).
	pub fn l_shape_stepped_eave() -> Self {
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.0, -2.0), Vec3::new(8.0, 4.0, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 3.2, -2.0), Vec3::new(2.0, 5.8, 8.0)),
		])
	}

	/// T: long-X bar + long-Z stem centered on the bar.
	pub fn t_shape() -> Self {
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.5, -2.0), Vec3::new(8.0, 4.5, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(2.0, 4.5, 8.0)),
		])
	}

	/// T with a taller / higher stem than the cross-bar.
	pub fn t_shape_stepped() -> Self {
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.0, -2.0), Vec3::new(8.0, 3.8, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.8, -2.0), Vec3::new(2.0, 5.5, 8.0)),
		])
	}

	/// One large hall gable (long X) with three smaller perpendicular bay gables.
	///
	/// Hall: lower eaves, higher ridge. Bays: higher eaves, lower ridges.
	pub fn hall_and_bays() -> Self {
		let hall = Aabb3d::from_min_max(Vec3::new(-14.0, 2.0, -3.0), Vec3::new(14.0, 5.8, 3.0));
		// Bays sit on +Z, overlapping the hall so each forms a T junction.
		let bay = |cx: f32| {
			Aabb3d::from_min_max(Vec3::new(cx - 2.0, 3.2, 1.0), Vec3::new(cx + 2.0, 4.6, 10.0))
		};
		Self::new(vec![hall, bay(-8.0), bay(0.0), bay(8.0)])
			.end_cap(EndCap::Gable { ridge: Overhang::Fixed(0.8), eave: Overhang::Fixed(0.7) })
	}

	/// Several pitch masses with no plan overlap (no valleys).
	pub fn disjoint() -> Self {
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-14.0, 2.5, -2.0), Vec3::new(-6.0, 4.5, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.0, -8.0), Vec3::new(2.0, 4.0, 0.0)),
			Aabb3d::from_min_max(Vec3::new(6.0, 2.8, -1.5), Vec3::new(14.0, 5.0, 1.5)),
			Aabb3d::from_min_max(Vec3::new(0.0, 2.2, 4.0), Vec3::new(4.0, 3.8, 12.0)),
		])
	}

	/// An L/T cluster plus a couple of non-intersecting satellites.
	pub fn mixed() -> Self {
		Self::new(vec![
			// Intersecting L
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(8.0, 4.5, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(2.0, 4.5, 8.0)),
			// T stem on the bar
			Aabb3d::from_min_max(Vec3::new(3.0, 2.5, -6.0), Vec3::new(7.0, 4.5, -1.0)),
			// Disjoint satellites
			Aabb3d::from_min_max(Vec3::new(-14.0, 2.0, 6.0), Vec3::new(-8.0, 4.0, 10.0)),
			Aabb3d::from_min_max(Vec3::new(12.0, 2.2, -2.0), Vec3::new(16.0, 3.8, 6.0)),
		])
	}

	/// Closed rectangular courtyard ring (four L corners on the inner court).
	pub fn ring() -> Self {
		// Outer ~[-8,8]², inner court ~[-4,4]².
		Self::new(vec![
			// North / South — long X
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.5, 4.0), Vec3::new(8.0, 4.5, 8.0)),
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.5, -8.0), Vec3::new(8.0, 4.5, -4.0)),
			// West / East — long Z
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.5, -8.0), Vec3::new(-4.0, 4.5, 8.0)),
			Aabb3d::from_min_max(Vec3::new(4.0, 2.5, -8.0), Vec3::new(8.0, 4.5, 8.0)),
		])
	}

	/// Courtyard ring with per-side ridge / eave heights (RunUp matters at corners).
	pub fn ring_stepped() -> Self {
		Self::new(vec![
			// North — tall ridge
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.5, 4.0), Vec3::new(8.0, 5.2, 8.0)),
			// South — low
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.0, -8.0), Vec3::new(8.0, 3.8, -4.0)),
			// West — mid, raised plate
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.8, -8.0), Vec3::new(-4.0, 4.6, 8.0)),
			// East — high plate + ridge
			Aabb3d::from_min_max(Vec3::new(4.0, 3.0, -8.0), Vec3::new(8.0, 5.5, 8.0)),
		])
	}

	/// Ring with a long southern leg (P footprint).
	pub fn p_shape() -> Self {
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.5, 4.0), Vec3::new(8.0, 4.5, 8.0)),
			// South extends past the east wall → the P's stem.
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.5, -8.0), Vec3::new(14.0, 4.5, -4.0)),
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.5, -8.0), Vec3::new(-4.0, 4.5, 8.0)),
			Aabb3d::from_min_max(Vec3::new(4.0, 2.5, -8.0), Vec3::new(8.0, 4.5, 8.0)),
		])
	}

	/// Two parallel long-X pitches on the same midline: different ridge heights
	/// and eave spans. Forms coaxial step junctions on the short sides.
	pub fn coaxial_parallel() -> Self {
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-10.0, 2.0, -2.0), Vec3::new(10.0, 4.0, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-6.0, 2.8, -3.5), Vec3::new(6.0, 5.5, 3.5)),
		])
	}

	/// Full orthogonal cross (+): decomposed into four L-meeting arms.
	pub fn pathological_cross() -> Self {
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-10.0, 2.5, -2.0), Vec3::new(10.0, 4.5, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -10.0), Vec3::new(2.0, 4.5, 10.0)),
		])
	}

	pub fn overhang(mut self, overhang: Overhang) -> Self {
		self.overhang = overhang;
		self
	}

	pub fn end_cap(mut self, end_cap: EndCap) -> Self {
		self.end_cap = end_cap;
		self
	}

	pub fn ridge_junction(mut self, ridge_junction: RidgeJunction) -> Self {
		self.ridge_junction = ridge_junction;
		self
	}

	pub fn style(mut self, style: PanelStyle) -> Self {
		self.style = style;
		self
	}

	pub fn openings(mut self, openings: Openings) -> Self {
		self.openings = openings;
		self
	}

	/// Subtract axis-aligned plan holes from the authored massing boxes.
	///
	/// Use this for tower / courtyard voids. Pitch [`OpeningLabel::Passage`] /
	/// [`OpeningLabel::Aperture`] openings only clip a face after the volumes
	/// have already been solved.
	pub fn with_plan_holes(mut self, holes: impl IntoIterator<Item = Aabb3d>) -> Self {
		self.volumes = punch_plan_holes(self.volumes, holes);
		self
	}

	/// Solve geometry (ignoring current openings), author a pitch opening on a
	/// resolved roof half, and attach it under `id`.
	pub fn with_pitch_opening(
		mut self,
		roof: usize,
		half: usize,
		u: f32,
		v: f32,
		width: f32,
		height: f32,
		id: impl Into<crate::openings::OpeningId>,
		label: OpeningLabel,
	) -> Self {
		let mut bare = self.clone();
		bare.openings = Openings::new();
		let geo = bare.build();
		if let Some(opening) = geo.pitch_opening(roof, half, u, v, width, height, label) {
			self.openings.insert(id, opening);
		}
		self
	}

	pub fn build(self) -> RectangularPitchedRoofComplex {
		RectangularPitchedRoofComplex::new(self)
	}
}

/// Resolved orthogonal roof complex.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularPitchedRoofComplex {
	params: RectangularPitchedRoofComplexParams,
	roofs: Vec<PitchedRoof>,
	valleys: Vec<ValleySegment>,
	/// Openings that survived nearest-roof assignment and face clipping.
	openings: Openings,
	mapped: MappedOpenings,
}

impl RectangularPitchedRoofComplex {
	pub fn new(params: RectangularPitchedRoofComplexParams) -> Self {
		let (roofs, valleys, openings, mapped) = resolve(&params);
		Self { params, roofs, valleys, openings, mapped }
	}

	pub fn params(&self) -> &RectangularPitchedRoofComplexParams {
		&self.params
	}

	pub fn roofs(&self) -> &[PitchedRoof] {
		&self.roofs
	}

	pub fn valleys(&self) -> &[ValleySegment] {
		&self.valleys
	}

	/// Stamp a roof shader look onto every child pitch (kit style unchanged).
	pub fn with_surface_material(mut self, material: MaterialRef) -> Self {
		self.roofs = self
			.roofs
			.into_iter()
			.map(|roof| roof.with_surface_material(material.clone()))
			.collect();
		self
	}
}

impl BuildingComponents for RectangularPitchedRoofComplex {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for roof in &self.roofs {
			out.extend(roof.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for roof in &self.roofs {
			out.extend(roof.joint_nodes_for_level(level));
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let material = self.roofs.first().and_then(|roof| roof.surface_material().cloned());
		let volumes: Vec<_> = self
			.params()
			.volumes
			.iter()
			.map(|aabb| {
				let xz = Aabb2d {
					min: Vec2::new(aabb.min.x, aabb.min.z),
					max: Vec2::new(aabb.max.x, aabb.max.z),
				};
				let along_x = (aabb.max.x - aabb.min.x) >= (aabb.max.z - aabb.min.z);
				MassingVolume::roof_pitch(
					xz,
					aabb.min.y,
					(aabb.max.y - aabb.min.y).max(0.3),
					along_x,
				)
				.with_material_opt(material.clone())
			})
			.collect();
		if volumes.is_empty() {
			None
		} else {
			Some(BuildingStructuralLodProbe::from_volumes(volumes))
		}
	}
}

fn resolve(
	params: &RectangularPitchedRoofComplexParams,
) -> (Vec<PitchedRoof>, Vec<ValleySegment>, Openings, MappedOpenings) {
	if params.volumes.is_empty() {
		return (Vec::new(), Vec::new(), Openings::new(), MappedOpenings::new());
	}

	let mut volumes: Vec<VolumeCandidate> = decompose_volumes(&params.volumes)
		.into_iter()
		.map(|aabb| VolumeCandidate::from_aabb(aabb, params.overhang))
		.collect();

	let junctions = resolve_junctions(&mut volumes);
	let mut valleys = apply_valleys(&mut volumes, &junctions, params.ridge_junction);

	for vol in &mut volumes {
		vol.apply_end_caps(params.end_cap);
	}
	// Hip: meet the lower run ridge on the hip centerline edge (not under the
	// higher ridge tip). Eaves stay on the end wall.
	finish_coaxial_ridge_meets(&mut volumes, &junctions.coaxial, params.end_cap, &mut valleys);

	let roofs: Vec<PitchedRoof> = volumes.iter().map(|vol| emit_roof(vol, params)).collect();
	let (roofs, openings, mapped) = apply_openings(roofs, &params.openings);

	(roofs, valleys, openings, mapped)
}

fn emit_roof(vol: &VolumeCandidate, params: &RectangularPitchedRoofComplexParams) -> PitchedRoof {
	let hip = matches!(params.end_cap, EndCap::Hip);
	let gable = matches!(params.end_cap, EndCap::Gable { .. });
	let free = vol.end_free;

	let halves = [0usize, 1].map(|side| {
		let mut half = RoofHalf::new(
			vol.ridge.as_tuple(),
			vol.eave[side].as_tuple(),
			vol.wall[side].as_tuple(),
		)
		.draw_in_wall_line(true);
		if hip {
			half = half.draw_in_half_hip((free[0], free[1]));
		}
		if gable {
			half = half.draw_in_half_gable_end((free[0], free[1]));
		}
		half
	});

	PitchedRoofParams::new(halves)
		.style(params.style)
		.joint_thickness(params.joint_thickness)
		.build()
}
