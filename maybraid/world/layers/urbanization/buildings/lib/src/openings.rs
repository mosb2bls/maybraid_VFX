//! Opening plans, shell records, and construct-time geometry maps.
//!
//! An [`Opening`] is a void the receiving type should avoid filling with geometry.
//! Shells that honor connectable openings (`Passage`, `Aperture`, `Shaft`) record
//! them and optionally map each id onto contact geometry ([`MappedOpening`]).
//!
//! ## Generated ids
//!
//! Floor plans that add openings should use [`OpeningId::scoped`] so ids stay
//! stable across towering when `scope`, `role`, and `slot` match
//! (`{scope}_{role}_{slot}`, e.g. `les_halles_shaft_nw`). Preserve inbound
//! confine opening ids unchanged; only generated openings use scoped ids.
//! Full\* / towers should reuse FloorPlan-emitted shaft ids when lifting
//! openings storey-to-storey (Y-translate bounds, keep the same id).
//!
//! ## Façade bays
//!
//! Shared door/window packing lives in [`facade_bays`] ([`BaySpec`], [`PlacedBay`]).

pub mod facade_bays;

pub use facade_bays::{
	fit_bays_on_run, fit_windows_on_run, generate_bay_catalog, generate_stall_doors,
	generate_windows, BaySpec, PlacedBay,
};

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use building_components::normalize_xz;
use std::collections::HashMap;

/// Stable identity for an opening within a plan or shell record.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OpeningId(pub String);

impl OpeningId {
	pub fn new(id: impl Into<String>) -> Self {
		Self(id.into())
	}

	/// Deterministic generated id: `{scope}_{role}_{slot}`.
	///
	/// Use for openings authored by a floor plan / typology. Inbound confine
	/// openings keep their original ids.
	pub fn scoped(scope: impl AsRef<str>, role: impl AsRef<str>, slot: impl AsRef<str>) -> Self {
		Self::new(format!("{}_{}_{}", scope.as_ref(), role.as_ref(), slot.as_ref()))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl From<&str> for OpeningId {
	fn from(value: &str) -> Self {
		Self::new(value)
	}
}

impl From<String> for OpeningId {
	fn from(value: String) -> Self {
		Self(value)
	}
}

/// Semantic role of an opening in a plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpeningLabel {
	/// Perimeter void / do not build wall here.
	Boundary,
	/// General keep-out (typically not retained on shell records).
	Exclusion,
	/// Walkable / connectable cut.
	Passage,
	/// Non-circulating cut (window-like).
	Aperture,
	/// Vertical circulation void (stairs, lifts, …).
	Shaft,
	/// Experiments only; not a stacking contract.
	Custom(String),
}

impl OpeningLabel {
	/// Labels shells typically retain and may map to contact geometry.
	pub fn is_connectable(&self) -> bool {
		matches!(self, Self::Passage | Self::Aperture | Self::Shaft)
	}

	/// Labels that may cut floor / ceiling slabs (not wall passages / apertures).
	pub fn cuts_slab(&self) -> bool {
		matches!(self, Self::Boundary | Self::Exclusion | Self::Shaft | Self::Custom(_))
	}
}

/// A void volume with a label.
#[derive(Debug, Clone, PartialEq)]
pub struct Opening {
	pub bounds: Aabb3d,
	pub label: OpeningLabel,
}

impl Opening {
	pub fn new(bounds: Aabb3d, label: OpeningLabel) -> Self {
		Self { bounds, label }
	}

	pub fn passage(bounds: Aabb3d) -> Self {
		Self::new(bounds, OpeningLabel::Passage)
	}

	pub fn aperture(bounds: Aabb3d) -> Self {
		Self::new(bounds, OpeningLabel::Aperture)
	}
}

/// Plan or realized record of openings keyed by id.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Openings {
	pub openings: HashMap<OpeningId, Opening>,
}

impl Openings {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn insert(&mut self, id: impl Into<OpeningId>, opening: Opening) -> &mut Self {
		self.openings.insert(id.into(), opening);
		self
	}

	pub fn with(mut self, id: impl Into<OpeningId>, opening: Opening) -> Self {
		self.insert(id, opening);
		self
	}

	pub fn get(&self, id: &OpeningId) -> Option<&Opening> {
		self.openings.get(id)
	}

	pub fn iter(&self) -> impl Iterator<Item = (&OpeningId, &Opening)> {
		self.openings.iter()
	}

	pub fn is_empty(&self) -> bool {
		self.openings.is_empty()
	}

	pub fn len(&self) -> usize {
		self.openings.len()
	}

	/// Insert all openings from `other`, overwriting on id collision.
	pub fn extend(&mut self, other: &Openings) {
		for (id, opening) in other.iter() {
			self.openings.insert(id.clone(), opening.clone());
		}
	}

	/// Copy of `self` with all openings from `other` merged in.
	pub fn merged_with(&self, other: &Openings) -> Self {
		let mut out = self.clone();
		out.extend(other);
		out
	}
}

/// Outward-facing opening quad (looking along the mapped orientation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MappedOpeningQuad {
	pub lower_left: Vec3,
	pub lower_right: Vec3,
	pub upper_left: Vec3,
	pub upper_right: Vec3,
}

impl MappedOpeningQuad {
	pub fn new(lower_left: Vec3, lower_right: Vec3, upper_left: Vec3, upper_right: Vec3) -> Self {
		Self { lower_left, lower_right, upper_left, upper_right }
	}

	pub fn corners(self) -> (Vec3, Vec3, Vec3, Vec3) {
		(self.lower_left, self.lower_right, self.upper_left, self.upper_right)
	}
}

/// An opening mapped onto shell contact geometry.
///
/// `orientation` is the outward facing in plan (\(x, z\)), matching the former
/// `ConnectingHallEndpoint::orientation` contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MappedOpening {
	pub face: MappedOpeningQuad,
	pub orientation: Vec2,
}

impl MappedOpening {
	pub fn new(face: MappedOpeningQuad, orientation: Vec2) -> Self {
		Self { face, orientation }
	}

	pub fn from_corners(
		lower_left: Vec3,
		lower_right: Vec3,
		upper_left: Vec3,
		upper_right: Vec3,
		orientation: Vec2,
	) -> Self {
		Self::new(
			MappedOpeningQuad::new(lower_left, lower_right, upper_left, upper_right),
			orientation,
		)
	}

	pub fn endpoint_corners(&self) -> (Vec3, Vec3, Vec3, Vec3) {
		self.face.corners()
	}

	/// Expand the opening horizontally past the jambs by `side_overrun` meters each side.
	///
	/// Expansion is from the opening midline along ±[`Self::orientation`]'s right, so it
	/// stays centered even if the authored corners were left/right swapped.
	pub fn widened(self, side_overrun: f32) -> Self {
		let overrun = side_overrun.max(0.0);
		let Some(orient) = normalize_xz(self.orientation) else {
			return self;
		};
		let right = Vec3::new(-orient.y, 0.0, orient.x);
		let (bl, br, tl, tr) = self.endpoint_corners();
		let bottom_mid = (bl + br) * 0.5;
		let top_mid = (tl + tr) * 0.5;
		let half_b = 0.5 * bl.distance(br) + overrun;
		let half_t = 0.5 * tl.distance(tr) + overrun;
		Self {
			face: MappedOpeningQuad::new(
				bottom_mid - right * half_b,
				bottom_mid + right * half_b,
				top_mid - right * half_t,
				top_mid + right * half_t,
			),
			orientation: self.orientation,
		}
	}

	/// Raise the lintel by `header_overrun` meters along the face up-direction.
	///
	/// Bottom corners stay put; tops move along bottom-mid → top-mid so pitched
	/// openings keep their wall plane while connectors clear the door head.
	pub fn raised(self, header_overrun: f32) -> Self {
		let overrun = header_overrun.max(0.0);
		if overrun <= 0.0 {
			return self;
		}
		let (bl, br, tl, tr) = self.endpoint_corners();
		let bottom_mid = (bl + br) * 0.5;
		let top_mid = (tl + tr) * 0.5;
		let face_up = (top_mid - bottom_mid).normalize_or_zero();
		let lift =
			if face_up.length_squared() > 0.0 { face_up * overrun } else { Vec3::Y * overrun };
		Self {
			face: MappedOpeningQuad::new(bl, br, tl + lift, tr + lift),
			orientation: self.orientation,
		}
	}
}

/// Construct-time maps from opening id to contact geometry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MappedOpenings(pub HashMap<OpeningId, MappedOpening>);

impl MappedOpenings {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn insert(&mut self, id: impl Into<OpeningId>, mapped: MappedOpening) -> &mut Self {
		self.0.insert(id.into(), mapped);
		self
	}

	pub fn get(&self, id: &OpeningId) -> Option<&MappedOpening> {
		self.0.get(id)
	}
}

/// Types that record openings and may expose mapped contact geometry.
///
/// Construction consumes an openings plan; [`mapped_opening`](Self::mapped_opening)
/// returns only openings this construction actually mapped — never a virtualized guess.
pub trait MapsOpenings {
	fn openings(&self) -> &Openings;

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening>;
}

/// Drop unmapped Passage/Aperture and copy shell AABBs (post-truncate) onto `openings`.
///
/// Shaft / Exclusion / other labels are kept as authored. Wall-mapped connectables
/// from `shell` overwrite plan entries so truncated leaves match the geometry.
pub fn sync_connectable_openings_from_mapped(openings: &mut Openings, shell: &impl MapsOpenings) {
	openings.openings.retain(|id, opening| match opening.label {
		OpeningLabel::Passage | OpeningLabel::Aperture => shell.mapped_opening(id).is_some(),
		_ => true,
	});
	for (id, opening) in shell.openings().iter() {
		openings.insert(id.clone(), opening.clone());
	}
}
